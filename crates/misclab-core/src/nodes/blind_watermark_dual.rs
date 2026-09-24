//! Two-image blind-watermark reveal (双图盲水印). Given the original + the
//! watermarked copy:
//!   • 频率盲水印   the chishaxie/BlindWaterMark scheme — the watermark is added
//!                  to the FFT and its rows/cols shuffled by a seed. Recover with
//!                  `rwm = Re(fft2(B) − fft2(A))/alpha`, then un-shuffle with the
//!                  same seed. (Algorithm ported from chishaxie/BlindWaterMark;
//!                  seed RNG verified byte-exact against CPython.)
//!   • 异或(XOR)    per-channel pixel XOR — reveals LSB / spatial differences.
//!   • 差值(放大)   |A − B| amplified — reveals faint spatial changes.
use image::{Rgba, RgbaImage};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use super::cpython_random::mt_from_u64;
use super::image_util::*;
use super::prelude::*;

/// Real part of `fft2(diff)` over the last two axes (W, C) — matches numpy's
/// `np.fft.fft2` default on an (H, W, C) array.
fn fft2_wc_real(diff: &[f32], h: usize, w: usize, c: usize) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fw = planner.plan_fft_forward(w);
    let fc = planner.plan_fft_forward(c);
    let mut out = vec![0f32; h * w * c];
    let mut buf = vec![Complex::<f32>::default(); w * c];
    let mut colw = vec![Complex::<f32>::default(); w];
    for row in 0..h {
        let base = row * w * c;
        for k in 0..w * c {
            buf[k] = Complex::new(diff[base + k], 0.0);
        }
        for c0 in 0..c {
            for w0 in 0..w {
                colw[w0] = buf[w0 * c + c0];
            }
            fw.process(&mut colw);
            for w0 in 0..w {
                buf[w0 * c + c0] = colw[w0];
            }
        }
        for w0 in 0..w {
            fc.process(&mut buf[w0 * c..w0 * c + c]);
        }
        for k in 0..w * c {
            out[base + k] = buf[k].re;
        }
    }
    out
}

fn u8_wrap(v: f32) -> u8 {
    (v as i64).rem_euclid(256) as u8
}

fn chishaxie_decode(
    a: &RgbaImage,
    b: &RgbaImage,
    seed: u64,
    alpha: f32,
    oldseed: bool,
) -> RgbaImage {
    let (w, h) = a.dimensions();
    let (wu, hu, cu) = (w as usize, h as usize, 3usize);

    // Difference in cv2's BGR order (the FFT mixes channels, so order matters).
    let mut diff = vec![0f32; hu * wu * cu];
    for (idx, (pa, pb)) in a.pixels().zip(b.pixels()).enumerate() {
        diff[idx * 3] = pb.0[2] as f32 - pa.0[2] as f32; // B
        diff[idx * 3 + 1] = pb.0[1] as f32 - pa.0[1] as f32; // G
        diff[idx * 3 + 2] = pb.0[0] as f32 - pa.0[0] as f32; // R
    }
    let rwm = fft2_wc_real(&diff, hu, wu, cu);

    // Same seeded permutation of the top-half rows and all columns.
    let mut mt = mt_from_u64(seed);
    let half = hu / 2;
    let mut m: Vec<usize> = (0..half).collect();
    let mut n: Vec<usize> = (0..wu).collect();
    if oldseed {
        mt.old_shuffle(&mut m);
        mt.old_shuffle(&mut n);
    } else {
        mt.shuffle(&mut m);
        mt.shuffle(&mut n);
    }

    let mut out = vec![0u8; hu * wu * cu];
    for i in 0..half {
        for j in 0..wu {
            for c in 0..cu {
                let v = rwm[(i * wu + j) * cu + c] / alpha;
                out[(m[i] * wu + n[j]) * cu + c] = u8_wrap(v);
            }
        }
    }
    // Mirror the recovered top half into the bottom (point symmetry).
    for i in 0..half {
        for j in 0..wu {
            for c in 0..cu {
                out[((hu - 1 - i) * wu + (wu - 1 - j)) * cu + c] = out[(i * wu + j) * cu + c];
            }
        }
    }

    let mut img = RgbaImage::new(w, h);
    for (idx, px) in img.pixels_mut().enumerate() {
        *px = Rgba([out[idx * 3 + 2], out[idx * 3 + 1], out[idx * 3], 255]); // R,G,B,A
    }
    img
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = load_image(i, "a")?;
        let b = load_image(i, "b")?;
        let (a, b) = align(a, b, false); // crop to shared size (never resize)
        let (w, h) = a.dimensions();
        if w == 0 || h == 0 {
            return Err(CoreError::Other("空图".into()));
        }

        let out = match pstr(p, "mode", "频率盲水印") {
            "异或(XOR)" => {
                let mut out = RgbaImage::new(w, h);
                for (px, (pa, pb)) in out.pixels_mut().zip(a.pixels().zip(b.pixels())) {
                    *px = Rgba([pa.0[0] ^ pb.0[0], pa.0[1] ^ pb.0[1], pa.0[2] ^ pb.0[2], 255]);
                }
                out
            }
            "差值(放大)" => {
                let amp = pnum(p, "amplify", 8.0) as f32;
                let mut out = RgbaImage::new(w, h);
                for (px, (pa, pb)) in out.pixels_mut().zip(a.pixels().zip(b.pixels())) {
                    let d = |x: u8, y: u8| {
                        (((x as i32 - y as i32).abs() as f32) * amp).clamp(0.0, 255.0) as u8
                    };
                    *px = Rgba([
                        d(pa.0[0], pb.0[0]),
                        d(pa.0[1], pb.0[1]),
                        d(pa.0[2], pb.0[2]),
                        255,
                    ]);
                }
                out
            }
            // chishaxie/BlindWaterMark frequency scheme.
            _ => {
                let seed = pnum(p, "seed", 20160930.0).max(0.0) as u64;
                let alpha = (pnum(p, "alpha", 3.0) as f32).max(1e-3);
                let oldseed = pbool(p, "oldseed", false);
                chishaxie_decode(&a, &b, seed, alpha, oldseed)
            }
        };
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "blind_watermark_dual",
            IMG,
            "两图盲水印",
            INDIGO,
            vec![
                req("a", "原图", PortType::Any),
                req("b", "含水印图", PortType::Any),
            ],
            vec![
                req("image", "水印", PortType::Image),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select(
                    "mode",
                    "模式",
                    &["频率盲水印", "异或(XOR)", "差值(放大)"],
                    "频率盲水印",
                ),
                ParamSpec::number(
                    "seed",
                    "随机种子(频率)",
                    0.0,
                    4_294_967_295.0,
                    1.0,
                    20160930.0,
                ),
                ParamSpec::number("alpha", "alpha(频率)", 0.1, 100.0, 0.1, 3.0),
                ParamSpec::toggle("oldseed", "Python2 随机(oldseed)", false),
                ParamSpec::number("amplify", "放大倍数(差值)", 1.0, 64.0, 1.0, 8.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chishaxie_recovers_watermark_block() {
        // Golden pair made by the real chishaxie/BlindWaterMark tool (seed 12345):
        // a 24×10 white block was embedded; decode must recover it bright, with a
        // black background — a wrong seed would spread noise everywhere instead.
        let a = image::load_from_memory(include_bytes!("../../tests/fixtures/bwm_carrier.png"))
            .unwrap()
            .to_rgba8();
        let b = image::load_from_memory(include_bytes!("../../tests/fixtures/bwm_encoded.png"))
            .unwrap()
            .to_rgba8();
        let out = chishaxie_decode(&a, &b, 12345, 3.0, false);
        let mean = |x0: u32, y0: u32, x1: u32, y1: u32| {
            let (mut s, mut n) = (0u64, 0u64);
            for y in y0..y1 {
                for x in x0..x1 {
                    s += out.get_pixel(x, y).0[0] as u64;
                    n += 1;
                }
            }
            s as f64 / n as f64
        };
        assert!(mean(4, 4, 28, 14) > 100.0, "block region should be bright");
        assert!(mean(30, 30, 60, 40) < 30.0, "background should be dark");
        // wrong seed → the block scatters, so its region is no longer cleanly bright-on-black
        let noise = chishaxie_decode(&a, &b, 99999, 3.0, false);
        let nmean = |x0: u32, y0: u32, x1: u32, y1: u32| {
            let (mut s, mut n) = (0u64, 0u64);
            for y in y0..y1 {
                for x in x0..x1 {
                    s += noise.get_pixel(x, y).0[0] as u64;
                    n += 1;
                }
            }
            s as f64 / n as f64
        };
        assert!(
            nmean(30, 30, 60, 40) > 40.0,
            "wrong seed should not give a clean black background"
        );
    }

    #[test]
    fn xor_reveals_differing_pixels() {
        let a = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
        let mut b = a.clone();
        b.put_pixel(3, 3, Rgba([10 ^ 0xFF, 20, 30, 255]));
        let mut out = RgbaImage::new(8, 8);
        for (px, (pa, pb)) in out.pixels_mut().zip(a.pixels().zip(b.pixels())) {
            *px = Rgba([pa.0[0] ^ pb.0[0], pa.0[1] ^ pb.0[1], pa.0[2] ^ pb.0[2], 255]);
        }
        assert_eq!(out.get_pixel(3, 3).0[0], 0xFF);
        assert_eq!(out.get_pixel(0, 0).0[0], 0);
    }

    #[test]
    fn u8_wrap_matches_numpy_uint8() {
        assert_eq!(u8_wrap(254.7), 254);
        assert_eq!(u8_wrap(-1.0), 255);
        assert_eq!(u8_wrap(256.5), 0);
        assert_eq!(u8_wrap(0.3), 0);
    }
}
