//! Native Rust port of **bkcrack**'s known-plaintext attack on ZipCrypto
//! (traditional PKWARE encryption), i.e. Biham & Kocher's attack.
//!
//! Ported closely from bkcrack <https://github.com/kimci86/bkcrack> by Kevin
//! "kimci86" Falcoz (zlib license). The lookup tables, `Keys` update rules,
//! `Data` preparation, `Zreduction` and the `Attack` recursion mirror the C++
//! source 1:1 so behavior matches. All 32-bit arithmetic uses wrapping ops to
//! reproduce C++ `std::uint32_t` semantics exactly.
//!
//! Reference: E. Biham and P. C. Kocher, "A Known Plaintext Attack on the PKZIP
//! Stream Cipher" (1994).
#![allow(clippy::needless_range_loop)]

use std::sync::OnceLock;

/// Multiplicative constant used in traditional PKWARE encryption.
const MULT: u32 = 0x0808_8405;
/// Multiplicative inverse of `MULT` modulo 2^32.
const MULTINV: u32 = 0xd94f_a8cd;

/// Number of contiguous known plaintext bytes required by the attack.
const CONTIGUOUS_SIZE: usize = 8;
/// Total number of known plaintext bytes required by the attack.
const ATTACK_SIZE: usize = 12;
/// Size of the traditional PKWARE encryption header prepended to each entry.
pub const ENCRYPTION_HEADER_SIZE: usize = 12;

/// Bit mask for bits `[begin, end)` (mirrors bkcrack's `mask<begin,end>`).
#[inline]
const fn mask(begin: u32, end: u32) -> u32 {
    (u32::MAX << begin) & (u32::MAX >> (32 - end))
}

#[inline]
const fn lsb(x: u32) -> u8 {
    x as u8
}

#[inline]
const fn msb(x: u32) -> u8 {
    (x >> 24) as u8
}

/// Maximum difference between A and B[x,32) knowing A = B + b, b a byte.
#[inline]
const fn maxdiff(x: u32) -> u32 {
    mask(0, x).wrapping_add(0xff)
}

// ---------------------------------------------------------------------------
// Lookup tables (built once, lazily).
// ---------------------------------------------------------------------------

struct Tables {
    crctab: [u32; 256],
    crcinvtab: [u32; 256],
    /// x such that msb(x*mult^-1) == i or i-1.
    fiber2: Vec<Vec<u8>>, // len 256
    /// x such that msb(x*mult^-1) == i-1, i or i+1.
    fiber3: Vec<Vec<u8>>, // len 256
    keystreamtab: Vec<u8>, // len 1<<14
    /// keystreaminvfiltertab[ki][zi_10_16>>10] flattened as [ki*64 + slot].
    invfilter: Vec<Vec<u32>>, // len 256*64
    invexists: Vec<bool>,     // len 256*64
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(build_tables)
}

fn build_tables() -> Tables {
    // CRC32 tables (polynomial 0xedb88320).
    let mut crctab = [0u32; 256];
    let mut crcinvtab = [0u32; 256];
    for b in 0u32..256 {
        let mut crc = b;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb8_8320;
            } else {
                crc >>= 1;
            }
        }
        crctab[b as usize] = crc;
        crcinvtab[msb(crc) as usize] = (crc << 8) ^ b;
    }

    // Multiplication fibers.
    let mut fiber2: Vec<Vec<u8>> = vec![Vec::new(); 256];
    let mut fiber3: Vec<Vec<u8>> = vec![Vec::new(); 256];
    let mut prodinv: u32 = 0; // x * mult^-1
    for x in 0u32..256 {
        let m = msb(prodinv) as u32;
        fiber2[m as usize].push(x as u8);
        fiber2[((m + 1) & 0xff) as usize].push(x as u8);

        fiber3[((m + 255) & 0xff) as usize].push(x as u8);
        fiber3[m as usize].push(x as u8);
        fiber3[((m + 1) & 0xff) as usize].push(x as u8);

        prodinv = prodinv.wrapping_add(MULTINV);
    }

    // Keystream tables.
    let mut keystreamtab = vec![0u8; 1 << 14];
    let mut invfilter: Vec<Vec<u32>> = vec![Vec::new(); 256 * 64];
    let mut invexists = vec![false; 256 * 64];
    let mut z_2_16: u32 = 0;
    while z_2_16 < (1 << 16) {
        let k = lsb((z_2_16 | 2).wrapping_mul(z_2_16 | 3) >> 8);
        keystreamtab[(z_2_16 >> 2) as usize] = k;
        let slot = (z_2_16 >> 10) as usize; // 0..64
        invfilter[k as usize * 64 + slot].push(z_2_16);
        invexists[k as usize * 64 + slot] = true;
        z_2_16 += 4;
    }

    Tables { crctab, crcinvtab, fiber2, fiber3, keystreamtab, invfilter, invexists }
}

#[inline]
fn crc32(pval: u32, b: u8) -> u32 {
    (pval >> 8) ^ tables().crctab[(lsb(pval) ^ b) as usize]
}

#[inline]
fn crc32inv(crc: u32, b: u8) -> u32 {
    (crc << 8) ^ tables().crcinvtab[msb(crc) as usize] ^ (b as u32)
}

/// Yi[24,32) from Zi and Z{i-1}.
#[inline]
fn get_yi_24_32(zi: u32, zim1: u32) -> u32 {
    (crc32inv(zi, 0) ^ zim1) << 24
}

/// Z{i-1}[10,32) from Zi[2,32).
#[inline]
fn get_zim1_10_32(zi_2_32: u32) -> u32 {
    crc32inv(zi_2_32, 0) & mask(10, 32)
}

#[inline]
fn keystream_byte(zi: u32) -> u8 {
    tables().keystreamtab[((zi & mask(0, 16)) >> 2) as usize]
}

/// Zi[2,16) values with given [10,16) bits whose keystream byte is `ki`.
#[inline]
fn get_zi_2_16(ki: u8, zi_10_16: u32) -> &'static [u32] {
    &tables().invfilter[ki as usize * 64 + ((zi_10_16 & mask(0, 16)) >> 10) as usize]
}

#[inline]
fn has_zi_2_16(ki: u8, zi_10_16: u32) -> bool {
    tables().invexists[ki as usize * 64 + ((zi_10_16 & mask(0, 16)) >> 10) as usize]
}

#[inline]
fn fiber2(i: u8) -> &'static [u8] {
    &tables().fiber2[i as usize]
}

#[inline]
fn fiber3(i: u8) -> &'static [u8] {
    &tables().fiber3[i as usize]
}

// ---------------------------------------------------------------------------
// Cipher state.
// ---------------------------------------------------------------------------

/// The three 32-bit internal keys of the ZipCrypto cipher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keys {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Default for Keys {
    fn default() -> Self {
        Keys { x: 0x1234_5678, y: 0x2345_6789, z: 0x3456_7890 }
    }
}

impl Keys {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Keys { x, y, z }
    }

    /// Keys derived from a password. Used by tests and available for callers that
    /// already know the password (e.g. verifying a recovered key set).
    #[allow(dead_code)]
    pub fn from_password(password: &[u8]) -> Self {
        let mut k = Keys::default();
        for &p in password {
            k.update(p);
        }
        k
    }

    #[inline]
    pub fn update(&mut self, p: u8) {
        self.x = crc32(self.x, p);
        self.y = self.y.wrapping_add(lsb(self.x) as u32).wrapping_mul(MULT).wrapping_add(1);
        self.z = crc32(self.z, msb(self.y));
    }

    #[inline]
    pub fn update_backward(&mut self, c: u8) {
        self.z = crc32inv(self.z, msb(self.y));
        self.y = self.y.wrapping_sub(1).wrapping_mul(MULTINV).wrapping_sub(lsb(self.x) as u32);
        let k = self.get_k();
        self.x = crc32inv(self.x, c ^ k);
    }

    /// Advance the state over `ciphertext[current..target]` (deciphering).
    /// Part of the faithful port (bkcrack's `Keys::update` over a range); kept for
    /// symmetry with [`Keys::update_backward_range`] even though the node's
    /// single-contiguous-plaintext path doesn't need forward range updates.
    #[allow(dead_code)]
    fn update_forward_range(&mut self, ciphertext: &[u8], current: usize, target: usize) {
        for &c in &ciphertext[current..target] {
            let k = self.get_k();
            self.update(c ^ k);
        }
    }

    /// Rewind the state over `ciphertext[target..current]`.
    fn update_backward_range(&mut self, ciphertext: &[u8], current: usize, target: usize) {
        for &c in ciphertext[target..current].iter().rev() {
            self.update_backward(c);
        }
    }

    #[inline]
    pub fn get_k(&self) -> u8 {
        keystream_byte(self.z)
    }
}

// ---------------------------------------------------------------------------
// Data preparation.
// ---------------------------------------------------------------------------

/// Data needed for an attack: ciphertext (incl. encryption header), a contiguous
/// run of known plaintext, and the derived keystream. This is a simplified port
/// of bkcrack's `Data` supporting a single contiguous plaintext block (no extra
/// scattered plaintext, which the node UI does not expose).
pub struct Data<'a> {
    pub ciphertext: &'a [u8],
    pub plaintext: Vec<u8>,
    pub keystream: Vec<u8>,
    /// Plaintext/keystream offset relative to ciphertext *with* encryption header.
    pub offset: usize,
}

impl<'a> Data<'a> {
    /// `offset_arg` is the plaintext offset relative to the ciphertext *without*
    /// the encryption header (may be negative, down to -12).
    pub fn new(ciphertext: &'a [u8], plaintext: Vec<u8>, offset_arg: i32) -> Result<Self, String> {
        if ciphertext.len() < ATTACK_SIZE {
            return Err(format!("密文太短，无法攻击（至少 {ATTACK_SIZE} 字节）"));
        }
        if ciphertext.len() < plaintext.len() {
            return Err("密文比明文还短".into());
        }
        let min_offset = -(ENCRYPTION_HEADER_SIZE as i32);
        if offset_arg < min_offset {
            return Err(format!("明文偏移 {offset_arg} 太小（最小 {min_offset}）"));
        }
        if (ciphertext.len() as i64)
            < ENCRYPTION_HEADER_SIZE as i64 + offset_arg as i64 + plaintext.len() as i64
        {
            return Err(format!("明文偏移 {offset_arg} 太大，超出密文范围"));
        }
        if plaintext.len() < CONTIGUOUS_SIZE {
            return Err(format!(
                "连续明文不足（{} 字节，至少 {CONTIGUOUS_SIZE}）",
                plaintext.len()
            ));
        }
        if plaintext.len() < ATTACK_SIZE {
            return Err(format!(
                "已知明文不足（{} 字节，至少 {ATTACK_SIZE}）",
                plaintext.len()
            ));
        }

        let offset = (ENCRYPTION_HEADER_SIZE as i32 + offset_arg) as usize;
        let keystream: Vec<u8> = plaintext
            .iter()
            .zip(ciphertext[offset..].iter())
            .map(|(p, c)| p ^ c)
            .collect();

        Ok(Data { ciphertext, plaintext, keystream, offset })
    }
}

// ---------------------------------------------------------------------------
// Z reduction.
// ---------------------------------------------------------------------------

#[inline]
fn bset(v: &mut [u64], i: u32) {
    v[(i >> 6) as usize] |= 1u64 << (i & 63);
}

#[inline]
fn bget(v: &[u64], i: u32) -> bool {
    v[(i >> 6) as usize] & (1u64 << (i & 63)) != 0
}

/// Generate the initial Zi[10,32) candidates from the last keystream byte.
fn zreduction_init(keystream: &[u8]) -> (Vec<u32>, usize) {
    let index = keystream.len() - 1;
    let mut zi_vector = Vec::new();
    for zi_10_32_shifted in 0u32..(1 << 22) {
        if has_zi_2_16(keystream[index], zi_10_32_shifted << 10) {
            zi_vector.push(zi_10_32_shifted << 10);
        }
    }
    (zi_vector, index)
}

/// Reduce the Zi[10,32) candidate set using the contiguous keystream, tracking
/// the smallest intermediate set. Faithful port of `Zreduction::reduce`.
fn zreduction_reduce<P: Fn(f32), C: Fn() -> bool>(
    keystream: &[u8],
    zi_vector: &mut Vec<u32>,
    index: &mut usize,
    on_progress: &P,
    is_cancelled: &C,
    lo: f32,
    hi: f32,
) -> Result<(), AttackError> {
    const TRACK_SIZE_THRESHOLD: usize = 1 << 16;
    const WAIT_SIZE_THRESHOLD: usize = 1 << 8;

    let mut tracking = false;
    let mut best_copy: Vec<u32> = Vec::new();
    let mut best_index = *index;
    let mut best_size = TRACK_SIZE_THRESHOLD;

    let mut waiting = false;
    let mut wait: usize = 0;

    let mut zim1_10_32_vector: Vec<u32> = Vec::new();
    let mut zim1_10_32_set = vec![0u64; (1usize << 22) / 64];

    let total = keystream.len() - CONTIGUOUS_SIZE;
    let mut done = 0usize;

    let mut i = *index;
    while i >= CONTIGUOUS_SIZE {
        if is_cancelled() {
            return Err(AttackError::Cancelled);
        }
        zim1_10_32_vector.clear();
        zim1_10_32_set.iter_mut().for_each(|w| *w = 0);
        let mut number_of_zim1_2_32 = 0usize;

        let ks_i = keystream[i];
        let ks_im1 = keystream[i - 1];
        for &zi_10_32 in zi_vector.iter() {
            for &zi_2_16 in get_zi_2_16(ks_i, zi_10_32) {
                let zim1_10_32 = get_zim1_10_32(zi_10_32 | zi_2_16);
                if !bget(&zim1_10_32_set, zim1_10_32 >> 10) && has_zi_2_16(ks_im1, zim1_10_32) {
                    zim1_10_32_vector.push(zim1_10_32);
                    bset(&mut zim1_10_32_set, zim1_10_32 >> 10);
                    number_of_zim1_2_32 += get_zi_2_16(ks_im1, zim1_10_32).len();
                }
            }
        }

        // Track the smallest Zi[2,32) set.
        if number_of_zim1_2_32 <= best_size {
            tracking = true;
            best_index = i - 1;
            best_size = number_of_zim1_2_32;
            waiting = false;
        } else if tracking {
            if best_index == i {
                std::mem::swap(&mut best_copy, zi_vector);
                if best_size <= WAIT_SIZE_THRESHOLD {
                    waiting = true;
                    wait = best_size * 4;
                }
            }
            // bkcrack does `--wait == 0` on a size_t; when `wait` is 0 (best_size
            // was 0, i.e. no candidates survived — bad plaintext) it wraps around
            // rather than breaking. `wrapping_sub` reproduces that and avoids a
            // debug underflow panic.
            if waiting {
                wait = wait.wrapping_sub(1);
                if wait == 0 {
                    break;
                }
            }
        }

        std::mem::swap(zi_vector, &mut zim1_10_32_vector);

        done += 1;
        on_progress(lo + (hi - lo) * (done as f32 / total.max(1) as f32));

        if i == 0 {
            break;
        }
        i -= 1;
    }

    if tracking {
        if best_index != CONTIGUOUS_SIZE - 1 {
            std::mem::swap(zi_vector, &mut best_copy);
        }
        *index = best_index;
    } else {
        *index = CONTIGUOUS_SIZE - 1;
    }
    Ok(())
}

/// Extend Zi[10,32) values into Zi[2,32) values. Faithful port of `generate`.
fn zreduction_generate(keystream: &[u8], zi_vector: &mut Vec<u32>, index: usize) {
    let n = zi_vector.len();
    let ks = keystream[index];
    for i in 0..n {
        let base = zi_vector[i];
        let v = get_zi_2_16(ks, base);
        if v.is_empty() {
            continue;
        }
        for &zi_2_16 in &v[1..] {
            zi_vector.push(base | zi_2_16);
        }
        zi_vector[i] = base | v[0];
    }
}

// ---------------------------------------------------------------------------
// Attack.
// ---------------------------------------------------------------------------

/// Reason an attack did not yield keys.
#[derive(Debug)]
pub enum AttackError {
    /// The provided data cannot be used for an attack (bad lengths/offsets).
    Data(String),
    /// The attack completed but found no keys (plaintext likely wrong).
    NoSolution,
    /// The operation was cancelled.
    Cancelled,
}

struct Attack<'a> {
    data: &'a Data<'a>,
    /// Starting index of the used plaintext/keystream.
    index: usize,
    zlist: [u32; CONTIGUOUS_SIZE],
    ylist: [u32; CONTIGUOUS_SIZE],
    xlist: [u32; CONTIGUOUS_SIZE],
    found: Option<Keys>,
}

impl<'a> Attack<'a> {
    fn new(data: &'a Data<'a>, zi_index: usize) -> Self {
        Attack {
            data,
            index: zi_index + 1 - CONTIGUOUS_SIZE,
            zlist: [0; CONTIGUOUS_SIZE],
            ylist: [0; CONTIGUOUS_SIZE],
            xlist: [0; CONTIGUOUS_SIZE],
            found: None,
        }
    }

    fn carryout(&mut self, z7_2_32: u32) -> bool {
        self.zlist[7] = z7_2_32;
        self.explore_zlists(7);
        self.found.is_some()
    }

    fn explore_zlists(&mut self, i: usize) {
        if self.found.is_some() {
            return;
        }
        if i != 0 {
            // Generate Z{i-1}[2,32) values.
            let zim1_10_32 = get_zim1_10_32(self.zlist[i]);
            let ks = self.data.keystream[self.index + i - 1];
            for &zim1_2_16 in get_zi_2_16(ks, zim1_10_32) {
                self.zlist[i - 1] = zim1_10_32 | zim1_2_16;

                // Find Zi[0,2) from CRC32^-1.
                self.zlist[i] &= mask(2, 32);
                self.zlist[i] |= (crc32inv(self.zlist[i], 0) ^ self.zlist[i - 1]) >> 8;

                if i < 7 {
                    self.ylist[i + 1] = get_yi_24_32(self.zlist[i + 1], self.zlist[i]);
                }

                self.explore_zlists(i - 1);
                if self.found.is_some() {
                    return;
                }
            }
        } else {
            // Z-list complete: iterate over possible Y7 values.
            let mut y7_8_24: u32 = 0;
            let mut prod: u32 =
                (MULTINV.wrapping_mul(msb(self.ylist[7]) as u32) << 24).wrapping_sub(MULTINV);
            while y7_8_24 < (1 << 24) {
                let idx = msb(self.ylist[6]).wrapping_sub(msb(prod));
                for &y7_0_8 in fiber3(idx) {
                    if prod
                        .wrapping_add(MULTINV.wrapping_mul(y7_0_8 as u32))
                        .wrapping_sub(self.ylist[6] & mask(24, 32))
                        <= maxdiff(24)
                    {
                        self.ylist[7] =
                            (y7_0_8 as u32) | y7_8_24 | (self.ylist[7] & mask(24, 32));
                        self.explore_ylists(7);
                        if self.found.is_some() {
                            return;
                        }
                    }
                }
                y7_8_24 += 1 << 8;
                prod = prod.wrapping_add(MULTINV << 8);
            }
        }
    }

    fn explore_ylists(&mut self, i: usize) {
        if self.found.is_some() {
            return;
        }
        if i != 3 {
            let fy = self.ylist[i].wrapping_sub(1).wrapping_mul(MULTINV);
            let ffy = fy.wrapping_sub(1).wrapping_mul(MULTINV);

            let idx = msb(ffy.wrapping_sub(self.ylist[i - 2] & mask(24, 32)));
            for &xi_0_8 in fiber2(idx) {
                let yim1 = fy.wrapping_sub(xi_0_8 as u32);
                if ffy
                    .wrapping_sub(MULTINV.wrapping_mul(xi_0_8 as u32))
                    .wrapping_sub(self.ylist[i - 2] & mask(24, 32))
                    <= maxdiff(24)
                    && msb(yim1) == msb(self.ylist[i - 1])
                {
                    self.ylist[i - 1] = yim1;
                    self.xlist[i] = xi_0_8 as u32;
                    self.explore_ylists(i - 1);
                    if self.found.is_some() {
                        return;
                    }
                }
            }
        } else {
            self.test_xlist();
        }
    }

    fn test_xlist(&mut self) {
        let data = self.data;

        // Compute X5, X6, X7.
        for i in 5..=7usize {
            self.xlist[i] = (crc32(self.xlist[i - 1], data.plaintext[self.index + i - 1])
                & mask(8, 32))
                | (lsb(self.xlist[i]) as u32);
        }

        // Compute X3.
        let mut x = self.xlist[7];
        for i in (3..=6usize).rev() {
            x = crc32inv(x, data.plaintext[self.index + i]);
        }

        // Check X3 fits with Y1[26,32).
        let y1_26_32 = get_yi_24_32(self.zlist[1], self.zlist[0]) & mask(26, 32);
        let check = self.ylist[3]
            .wrapping_sub(1)
            .wrapping_mul(MULTINV)
            .wrapping_sub(lsb(x) as u32)
            .wrapping_sub(1)
            .wrapping_mul(MULTINV)
            .wrapping_sub(y1_26_32);
        if check > maxdiff(26) {
            return;
        }

        // Filter forward against remaining contiguous plaintext.
        let mut keys_forward = Keys::new(self.xlist[7], self.ylist[7], self.zlist[7]);
        keys_forward.update(data.plaintext[self.index + 7]);
        {
            let mut ci = data.offset + self.index + 8;
            for pi in (self.index + 8)..data.plaintext.len() {
                let c = data.ciphertext[ci];
                if (c ^ keys_forward.get_k()) != data.plaintext[pi] {
                    return;
                }
                keys_forward.update(data.plaintext[pi]);
                ci += 1;
            }
        }

        // Filter backward.
        let mut keys_backward = Keys::new(x, self.ylist[3], self.zlist[3]);
        for j in (0..(self.index + 3)).rev() {
            let c = data.ciphertext[data.offset + j];
            keys_backward.update_backward(c);
            if (c ^ keys_backward.get_k()) != data.plaintext[j] {
                return;
            }
        }

        // All tests passed: rewind to the initial state to get the keys.
        keys_backward.update_backward_range(data.ciphertext, data.offset, 0);
        self.found = Some(keys_backward);
    }
}

/// Run the attack over an explicit set of Zi[2,32) candidates at `zi_index`,
/// **across all available CPU cores** (like bkcrack): worker threads pull
/// candidates from a shared counter, the first to find valid keys stops the rest.
/// Progress is reported from the shared completed-candidate counter and
/// cancellation is polled per candidate.
#[allow(clippy::too_many_arguments)]
fn attack_over_candidates<P, C>(
    data: &Data,
    candidates: &[u32],
    zi_index: usize,
    on_progress: &P,
    is_cancelled: &C,
    lo: f32,
    hi: f32,
) -> Result<Option<Keys>, AttackError>
where
    P: Fn(f32) + Sync,
    C: Fn() -> bool + Sync,
{
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    let n = candidates.len();
    if n == 0 {
        return Ok(None);
    }

    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let cancelled = AtomicBool::new(false);
    let result: Mutex<Option<Keys>> = Mutex::new(None);

    let threads = std::thread::available_parallelism()
        .map(|c| c.get())
        .unwrap_or(1)
        .clamp(1, n);

    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut worker = Attack::new(data, zi_index);
                loop {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if is_cancelled() {
                        cancelled.store(true, Ordering::Relaxed);
                        stop.store(true, Ordering::Relaxed);
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    if worker.carryout(candidates[i]) {
                        *result.lock().expect("attack result mutex") = worker.found.take();
                        stop.store(true, Ordering::Relaxed);
                        break;
                    }
                    let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if d.is_multiple_of(64) {
                        on_progress(lo + (hi - lo) * (d as f32 / n as f32));
                    }
                }
            });
        }
    });

    if cancelled.load(Ordering::Relaxed) {
        return Err(AttackError::Cancelled);
    }
    Ok(result.into_inner().expect("attack result mutex"))
}

/// Recover the internal ZipCrypto keys from ciphertext (including the 12-byte
/// encryption header) and a run of known plaintext at `offset` (relative to the
/// ciphertext without the header, may be negative).
///
/// `on_progress` receives a 0..1 fraction; `is_cancelled` is polled to abort;
/// `on_log` receives short status strings (e.g. the candidate-set size). All three
/// may be called from multiple threads, so they must be `Sync`.
pub fn recover_keys<P, C, L>(
    ciphertext: &[u8],
    plaintext: Vec<u8>,
    offset: i32,
    on_progress: P,
    is_cancelled: C,
    on_log: L,
) -> Result<Keys, AttackError>
where
    P: Fn(f32) + Sync,
    C: Fn() -> bool + Sync,
    L: Fn(&str) + Sync,
{
    let data = Data::new(ciphertext, plaintext, offset).map_err(AttackError::Data)?;

    // Z reduction (progress 0..0.5).
    let (mut zi_vector, mut index) = zreduction_init(&data.keystream);
    if data.keystream.len() > CONTIGUOUS_SIZE {
        zreduction_reduce(
            &data.keystream,
            &mut zi_vector,
            &mut index,
            &on_progress,
            &is_cancelled,
            0.0,
            0.5,
        )?;
    }
    zreduction_generate(&data.keystream, &mut zi_vector, index);
    on_log(&format!(
        "Z 约简完成：候选 {} 组，开始多线程搜索…",
        zi_vector.len()
    ));

    // Attack (progress 0.5..1.0), parallelized across cores.
    match attack_over_candidates(&data, &zi_vector, index, &on_progress, &is_cancelled, 0.5, 1.0)? {
        Some(keys) => Ok(keys),
        None => Err(AttackError::NoSolution),
    }
}

/// Decipher an encrypted entry (ciphertext incl. 12-byte header) with recovered
/// keys, returning the plaintext data **without** the encryption header.
pub fn decipher(ciphertext_incl_header: &[u8], keys: Keys) -> Vec<u8> {
    let mut k = keys;
    let mut out = Vec::with_capacity(ciphertext_incl_header.len().saturating_sub(ENCRYPTION_HEADER_SIZE));
    for (i, &c) in ciphertext_incl_header.iter().enumerate() {
        let p = c ^ k.get_k();
        k.update(p);
        if i >= ENCRYPTION_HEADER_SIZE {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden vectors from bkcrack's own tests (password "password" over
    // "Hello World!" ciphertext), which validate the tables + Keys + attack.
    const PLAINTEXT: &[u8] = b"Hello World!";
    const CIPHERTEXT: [u8; 24] = [
        0x3e, 0xb4, 0xc5, 0x92, 0x58, 0x40, 0x9a, 0x6c, 0xed, 0x99, 0x65, 0x81, 0x66, 0x1b, 0x1d,
        0xda, 0x5d, 0x8a, 0x8c, 0x30, 0x07, 0x76, 0x50, 0xbb,
    ];
    const EXPECTED: Keys = Keys { x: 0xea9b_4e4d, y: 0xba78_9085, z: 0x5ff8_707d };

    #[test]
    fn keys_from_password_matches() {
        assert_eq!(Keys::from_password(b"password"), EXPECTED);
    }

    #[test]
    fn keystream_table_vectors() {
        let k = [b't' ^ 0x9a, b'e' ^ 0x6b, b's' ^ 0x40, b't' ^ 0x2c];
        assert_eq!(keystream_byte(0x5ff8_707d), k[0]);
        assert_eq!(keystream_byte(0x868c_2aa4), k[1]);
        assert_eq!(get_zi_2_16(k[0], 0x7000), &[0x707c]);
        assert_eq!(get_zi_2_16(k[1], 0x2800), &[0x29a8, 0x2aa4, 0x2ab0, 0x2b3c]);
        assert_eq!(get_zi_2_16(k[0], 0x6000), &[] as &[u32]);
    }

    #[test]
    fn keys_update_backward_roundtrip() {
        // From Keys.test.cpp: update "test" then rewind reaches password keys.
        let mut k = Keys::from_password(b"password");
        for &p in b"test" {
            k.update(p);
        }
        assert_eq!(k, Keys { x: 0x382b_d98d, y: 0x5ad5_5f3b, z: 0x04f8_d2f6 });
        for &c in [0x2cu8, 0x40, 0x6b, 0x9a].iter() {
            k.update_backward(c);
        }
        assert_eq!(k, EXPECTED);
    }

    #[test]
    fn direct_attack_recovers_keys() {
        // Mirror Attack.test.cpp "simple case": attack the z7 candidate list at
        // index 7 and check the recovered initial keys.
        let z7: u32 = 0x7493_0e66;
        let candidates: Vec<u32> = vec![
            0x0000_0000, 0x1000_0000, 0x2000_0000, 0x3000_0000, 0x4000_0000, 0x5000_0000,
            0x6000_0000, z7 & mask(2, 32), 0x8000_0000, 0x9000_0000, 0xa000_0000, 0xb000_0000,
            0xc000_0000, 0xd000_0000, 0xe000_0000, 0xf000_0000,
        ];
        let data = Data::new(&CIPHERTEXT, PLAINTEXT.to_vec(), 0).unwrap();
        let found = attack_over_candidates(&data, &candidates, 7, &|_| {}, &|| false, 0.0, 1.0)
            .unwrap()
            .expect("should find keys");
        assert_eq!(found, EXPECTED);
    }

    #[test]
    #[ignore = "slow in debug: 12 plaintext bytes → little Z reduction, huge \
                candidate set. Run in release: `cargo test -p misclab-core --release -- --ignored`."]
    fn full_pipeline_finds_valid_keys_min_plaintext() {
        // With only 12 bytes (the minimum), the solution is NOT unique — the attack
        // may return a different valid key set than the original. So assert the
        // recovered keys are *a* valid solution (they decipher the ciphertext to the
        // known plaintext), not that they equal the original keys. Uniqueness with
        // more plaintext is covered by `end_to_end_recovers_and_deciphers`.
        let keys = recover_keys(&CIPHERTEXT, PLAINTEXT.to_vec(), 0, |_| {}, || false, |_| {})
            .expect("full pipeline should find valid keys");
        assert_eq!(decipher(&CIPHERTEXT, keys), PLAINTEXT);
    }

    #[test]
    fn decipher_roundtrip() {
        // Deciphering the ciphertext with the keys yields the plaintext.
        let out = decipher(&CIPHERTEXT, EXPECTED);
        assert_eq!(&out, PLAINTEXT);
    }

    /// Encrypt `data` (after a 12-byte header) with a password, using the
    /// validated forward cipher, producing ciphertext incl. header.
    fn encrypt(password: &[u8], header: &[u8; 12], data: &[u8]) -> Vec<u8> {
        let mut k = Keys::from_password(password);
        let mut ct = Vec::with_capacity(12 + data.len());
        for &h in header {
            ct.push(h ^ k.get_k());
            k.update(h);
        }
        for &p in data {
            ct.push(p ^ k.get_k());
            k.update(p);
        }
        ct
    }

    #[test]
    #[ignore = "full pipeline: the 2^22 candidate init + first Z-reduction step are \
                slow in debug (fast in release). Run: \
                `cargo test -p misclab-core --release -- --include-ignored bkcrack`."]
    fn end_to_end_recovers_and_deciphers() {
        // Full engine path (Data → Zreduction → generate → Attack) on a
        // self-made ZipCrypto stream.
        let password = b"S3cr3tPass";
        let data: &[u8] = b"flag{bkcrack_native_rust_port_end_to_end!}";
        let ciphertext = encrypt(password, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], data);

        let expected = Keys::from_password(password);
        let keys = recover_keys(&ciphertext, data.to_vec(), 0, |_| {}, || false, |_| {})
            .expect("should recover keys end-to-end");
        assert_eq!(keys, expected);
        assert_eq!(decipher(&ciphertext, keys), data);
    }
}
