//! Extract a watermark hidden by **guofei9987/blind_watermark** (the DWT-DCT-SVD
//! scheme, "提取水印无须原图" — only the watermarked image is needed).
//!
//! Per channel: BGR→YUV, Haar DWT → the LL sub-band, split into 4×4 blocks. For
//! each block: DCT → permute the 16 coefficients by a per-block order derived from
//! `np.random.RandomState(password_img)` → SVD → read a bit from the largest
//! singular value (`s0 % 36 > 18`, refined with `s1 % 20`). Bits are averaged over
//! the 3 channels and their tiled repeats, thresholded (1-D k-means for text),
//! un-shuffled by `password_wm`, and turned back into text or a watermark image.
//! Algorithm ported from bwm_core.py; the numpy RNG is byte-exact (see
//! `cpython_random`). You must supply the watermark size (bits for text, or W×H
//! for an image) and the two passwords.
use image::{Rgba, RgbaImage};
use num_bigint::BigUint;

use super::cpython_random::mt_numpy;
use super::image_util::{data_url, input_bytes, to_png};
use super::prelude::*;

const D1: f64 = 36.0;
const D2: f64 = 20.0;

/// OpenCV `COLOR_BGR2YUV` on float32 (BT.601, 0.5 delta).
fn bgr2yuv(b: f32, g: f32, r: f32) -> (f32, f32, f32) {
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = -0.14713 * r - 0.28886 * g + 0.436 * b + 0.5;
    let v = 0.615 * r - 0.51499 * g - 0.10001 * b + 0.5;
    (y, u, v)
}

/// Haar DWT LL sub-band of an even-sized channel: `ca[i][j] = (Σ 2×2 block)/2`.
fn dwt_haar_ca(chan: &[f32], h: usize, w: usize) -> (Vec<f32>, usize, usize) {
    let (ch, cw) = (h / 2, w / 2);
    let mut ca = vec![0f32; ch * cw];
    for i in 0..ch {
        for j in 0..cw {
            let a = chan[(2 * i) * w + 2 * j];
            let b = chan[(2 * i) * w + 2 * j + 1];
            let c = chan[(2 * i + 1) * w + 2 * j];
            let d = chan[(2 * i + 1) * w + 2 * j + 1];
            ca[i * cw + j] = (a + b + c + d) / 2.0;
        }
    }
    (ca, ch, cw)
}

/// Orthonormal 2-D DCT-II of a 4×4 block (matches `cv2.dct`).
fn dct4(block: &[f32; 16]) -> [f32; 16] {
    let n = 4usize;
    let cf = |k: usize| if k == 0 { (1.0f32 / n as f32).sqrt() } else { (2.0f32 / n as f32).sqrt() };
    let mut cos = [[0f32; 4]; 4];
    for (k, row) in cos.iter_mut().enumerate() {
        for (i, cell) in row.iter_mut().enumerate() {
            *cell = (std::f32::consts::PI * (2 * i + 1) as f32 * k as f32 / (2.0 * n as f32)).cos();
        }
    }
    let mut tmp = [0f32; 16];
    for r in 0..4 {
        for k in 0..4 {
            let mut s = 0f32;
            for i in 0..4 {
                s += block[r * 4 + i] * cos[k][i];
            }
            tmp[r * 4 + k] = cf(k) * s;
        }
    }
    let mut out = [0f32; 16];
    for col in 0..4 {
        for k in 0..4 {
            let mut s = 0f32;
            for i in 0..4 {
                s += tmp[i * 4 + col] * cos[k][i];
            }
            out[k * 4 + col] = cf(k) * s;
        }
    }
    out
}

/// Two largest singular values of a 4×4 matrix, via Jacobi eigenvalues of AᵀA.
fn svd_top2(m: &[f32; 16]) -> (f64, f64) {
    let a: Vec<f64> = m.iter().map(|&x| x as f64).collect();
    // AᵀA (4×4 symmetric)
    let mut s = [[0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let mut acc = 0.0;
            for k in 0..4 {
                acc += a[k * 4 + i] * a[k * 4 + j];
            }
            s[i][j] = acc;
        }
    }
    // Jacobi eigenvalue iteration.
    for _ in 0..60 {
        // largest off-diagonal
        let (mut p, mut q, mut max) = (0, 1, 0.0f64);
        for (i, row) in s.iter().enumerate() {
            for (j, &val) in row.iter().enumerate().skip(i + 1) {
                if val.abs() > max {
                    max = val.abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max < 1e-12 {
            break;
        }
        let theta = (s[q][q] - s[p][p]) / (2.0 * s[p][q]);
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
        let c = 1.0 / (t * t + 1.0).sqrt();
        let sn = t * c;
        for row in &mut s {
            let skp = row[p];
            let skq = row[q];
            row[p] = c * skp - sn * skq;
            row[q] = sn * skp + c * skq;
        }
        let [row_p, row_q] = s.get_disjoint_mut([p, q]).unwrap();
        for (spk, sqk) in row_p.iter_mut().zip(row_q.iter_mut()) {
            let old_spk = *spk;
            let old_sqk = *sqk;
            *spk = c * old_spk - sn * old_sqk;
            *sqk = sn * old_spk + c * old_sqk;
        }
    }
    let mut eig = [s[0][0], s[1][1], s[2][2], s[3][3]];
    eig.sort_by(|a, b| b.partial_cmp(a).unwrap());
    (eig[0].max(0.0).sqrt(), eig[1].max(0.0).sqrt())
}

/// Per-block coefficient permutation: `RandomState(pw).random((n,16)).argsort(axis=1)`.
fn idx_shuffle(pw: u32, block_num: usize) -> Vec<[usize; 16]> {
    let mut mt = mt_numpy(pw);
    let mut out = Vec::with_capacity(block_num);
    for _ in 0..block_num {
        let vals: [f64; 16] = std::array::from_fn(|_| mt.random_f64());
        let mut order: [usize; 16] = std::array::from_fn(|i| i);
        order.sort_by(|&a, &b| vals[a].partial_cmp(&vals[b]).unwrap());
        out.push(order);
    }
    out
}

/// Averaged watermark bits (0..1), one per watermark position.
fn extract_avg(img: &RgbaImage, pw_img: u32, wm_size: usize) -> Vec<f64> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    // pad to even (bottom/right), like cv2.copyMakeBorder with value 0.
    let (hp, wp) = (h + h % 2, w + w % 2);
    let mut yuv = vec![[0f32; 3]; hp * wp];
    for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel(x as u32, y as u32).0;
            let (yy, uu, vv) = bgr2yuv(px[2] as f32, px[1] as f32, px[0] as f32);
            yuv[y * wp + x] = [yy, uu, vv];
        }
    }
    let cbh = (hp / 2) / 4;
    let cbw = (wp / 2) / 4;
    let block_num = cbh * cbw;
    if block_num == 0 || wm_size == 0 {
        return vec![0.0; wm_size];
    }
    let shufflers = idx_shuffle(pw_img, block_num);

    // wm_block_bit[channel][block]
    let mut bits = vec![vec![0f64; block_num]; 3];
    let mut chan = vec![0f32; hp * wp];
    for c in 0..3 {
        for (k, cell) in chan.iter_mut().enumerate() {
            *cell = yuv[k][c];
        }
        let (ca, _ch, cw) = dwt_haar_ca(&chan, hp, wp);
        for (bi, sh) in shufflers.iter().enumerate() {
            let (br, bc) = (bi / cbw, bi % cbw);
            let mut block = [0f32; 16];
            for r in 0..4 {
                for col in 0..4 {
                    block[r * 4 + col] = ca[(br * 4 + r) * cw + (bc * 4 + col)];
                }
            }
            let d = dct4(&block);
            // permute the flattened 16 coefficients by the block's shuffler
            let mut shuffled = [0f32; 16];
            for (pos, &src) in sh.iter().enumerate() {
                shuffled[pos] = d[src];
            }
            let (s0, s1) = svd_top2(&shuffled);
            let b0 = if s0 % D1 > D1 / 2.0 { 1.0 } else { 0.0 };
            let b1 = if s1 % D2 > D2 / 2.0 { 1.0 } else { 0.0 };
            bits[c][bi] = (b0 * 3.0 + b1) / 4.0;
        }
    }

    // average over channels + tiled repeats
    let mut avg = vec![0.0f64; wm_size];
    for (i, a) in avg.iter_mut().enumerate() {
        let (mut sum, mut n) = (0.0, 0u32);
        for chan in bits.iter().take(3) {
            let mut j = i;
            while j < block_num {
                sum += chan[j];
                n += 1;
                j += wm_size;
            }
        }
        *a = if n > 0 { sum / n as f64 } else { 0.0 };
    }
    avg
}

/// 1-D k-means threshold → bools (guofei9987 `one_dim_kmeans`).
fn kmeans(inputs: &[f64]) -> Vec<bool> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &x in inputs {
        lo = lo.min(x);
        hi = hi.max(x);
    }
    let mut center = [lo, hi];
    let mut threshold = 0.0;
    for _ in 0..300 {
        threshold = (center[0] + center[1]) / 2.0;
        let (mut s0, mut n0, mut s1, mut n1) = (0.0, 0u32, 0.0, 0u32);
        for &x in inputs {
            if x > threshold {
                s1 += x;
                n1 += 1;
            } else {
                s0 += x;
                n0 += 1;
            }
        }
        let new = [
            if n0 > 0 { s0 / n0 as f64 } else { center[0] },
            if n1 > 0 { s1 / n1 as f64 } else { center[1] },
        ];
        if ((new[0] + new[1]) / 2.0 - threshold).abs() < 1e-6 {
            threshold = (new[0] + new[1]) / 2.0;
            break;
        }
        center = new;
    }
    inputs.iter().map(|&x| x > threshold).collect()
}

/// Un-shuffle by password_wm: `wm[idx[k]] = wm[k]`.
fn decrypt<T: Copy + Default>(arr: &[T], pw_wm: u32) -> Vec<T> {
    let mut idx: Vec<usize> = (0..arr.len()).collect();
    mt_numpy(pw_wm).numpy_shuffle(&mut idx);
    let mut out = vec![T::default(); arr.len()];
    for (k, &dst) in idx.iter().enumerate() {
        out[dst] = arr[k];
    }
    out
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bytes = input_bytes(inputs, "data")?;
        let img = image::load_from_memory(&bytes)
            .map_err(|e| CoreError::Parse(format!("图片解码失败: {e}")))?
            .to_rgba8();
        let pw_wm = (pnum(p, "pwWm", 1.0).max(0.0)) as u32;
        let pw_img = (pnum(p, "pwImg", 1.0).max(0.0)) as u32;
        let mode = pstr(p, "mode", "文本");

        let mut m = PortMap::new();
        if mode == "图片" {
            let (ww, wh) = (pnum(p, "wmWidth", 64.0) as usize, pnum(p, "wmHeight", 64.0) as usize);
            let wm_size = ww * wh;
            if wm_size == 0 {
                return Err(CoreError::Other("请设置水印宽高。".into()));
            }
            let avg = extract_avg(&img, pw_img, wm_size);
            let dec = decrypt(&avg, pw_wm);
            let mut out = RgbaImage::new(ww as u32, wh as u32);
            for (i, px) in out.pixels_mut().enumerate() {
                let g = (dec[i] * 255.0).clamp(0.0, 255.0) as u8;
                *px = Rgba([g, g, g, 255]);
            }
            let png = to_png(&out)?;
            m.insert("image".into(), PortValue::Image(data_url(&png, "image/png")));
            m.insert("report".into(), PortValue::Text(format!("图片水印 {ww}×{wh}")));
        } else {
            let wm_size = pnum(p, "wmLength", 0.0) as usize;
            if wm_size == 0 {
                return Err(CoreError::Other(
                    "文本水印需要「水印位数」（编码时 bin(int(text.hex(),16)) 的位长）。".into(),
                ));
            }
            let avg = extract_avg(&img, pw_img, wm_size);
            let bits = kmeans(&avg);
            let dec = decrypt(&bits, pw_wm);
            // bits → big integer → bytes → utf-8
            let mut n = BigUint::ZERO;
            for &bit in &dec {
                n <<= 1;
                if bit {
                    n |= BigUint::from(1u32);
                }
            }
            let text = String::from_utf8_lossy(&n.to_bytes_be()).into_owned();
            m.insert("text".into(), PortValue::Text(text));
            m.insert("report".into(), PortValue::Text(format!("文本水印 {wm_size} 位")));
        }
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "guofei_bwm_extract",
            STEG,
            "盲水印提取(guofei9987)",
            PURPLE,
            vec![req("data", "含水印图", PortType::Any)],
            vec![
                opt("text", "文本水印", PortType::Text),
                opt("image", "图片水印", PortType::Image),
                opt("report", "说明", PortType::Text),
            ],
            vec![
                ParamSpec::select("mode", "水印类型", &["文本", "图片"], "文本"),
                ParamSpec::number("wmLength", "水印位数(文本)", 0.0, 1_000_000.0, 1.0, 0.0),
                ParamSpec::number("wmWidth", "水印宽(图片)", 0.0, 4096.0, 1.0, 64.0),
                ParamSpec::number("wmHeight", "水印高(图片)", 0.0, 4096.0, 1.0, 64.0),
                ParamSpec::number("pwWm", "水印密码 password_wm", 0.0, 4_294_967_295.0, 1.0, 1.0),
                ParamSpec::number("pwImg", "图密码 password_img", 0.0, 4_294_967_295.0, 1.0, 1.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_text_watermark() {
        // g987_embedded.png holds "flag{g987}" (79 bits, pw_wm=pw_img=1),
        // embedded by the real guofei9987/blind_watermark tool.
        let img = image::load_from_memory(include_bytes!("../../tests/fixtures/g987_embedded.png"))
            .unwrap()
            .to_rgba8();
        let avg = extract_avg(&img, 1, 79);
        let bits = kmeans(&avg);
        let dec = decrypt(&bits, 1);
        let mut n = BigUint::ZERO;
        for &b in &dec {
            n <<= 1;
            if b {
                n |= BigUint::from(1u32);
            }
        }
        assert_eq!(String::from_utf8_lossy(&n.to_bytes_be()), "flag{g987}");
    }
}
