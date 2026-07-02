//! Single-image blind-watermark reveal via 2-D FFT magnitude. Watermarks that
//! are embedded symmetrically in the frequency domain (the common "文字盲水印",
//! and what tools like ww23/BlindWatermark produce) surface as bright text in
//! the magnitude spectrum. The scaling options mirror the menu of the popular
//! GUI extractor:
//!   • FFT(Multiplier)            linear |F| × multiplier, clamp, no shift
//!   • FFT(fftshiftMultiplier)    linear |F| × multiplier, clamp, DC centered
//!   • FFT(Normalization)         linear |F| stretched to 0‥255, no shift
//!   • FFT(fftshift_Normalization) linear |F| stretched to 0‥255, DC centered
//!   • Java-BlindWatermark        log(1+|F|) stretched, DC centered (best default)
use image::{Rgba, RgbaImage};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use super::image_util::*;
use super::prelude::*;

fn channel_value(px: &Rgba<u8>, ch: &str) -> f32 {
    match ch {
        "R" => px.0[0] as f32,
        "G" => px.0[1] as f32,
        "B" => px.0[2] as f32,
        _ => luma(px.0[0], px.0[1], px.0[2]) as f32,
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (w, h) = (img.width() as usize, img.height() as usize);
        if w == 0 || h == 0 {
            return Err(CoreError::Other("空图".into()));
        }

        let ch = pstr(p, "channel", "灰度");
        let mut data: Vec<Complex<f32>> = img
            .pixels()
            .map(|px| Complex::new(channel_value(px, ch), 0.0))
            .collect();

        // Row FFTs, then column FFTs → 2-D DFT.
        let mut planner = FftPlanner::<f32>::new();
        let fft_row = planner.plan_fft_forward(w);
        let fft_col = planner.plan_fft_forward(h);
        for y in 0..h {
            fft_row.process(&mut data[y * w..(y + 1) * w]);
        }
        let mut col = vec![Complex::default(); h];
        for x in 0..w {
            for y in 0..h {
                col[y] = data[y * w + x];
            }
            fft_col.process(&mut col);
            for y in 0..h {
                data[y * w + x] = col[y];
            }
        }

        let mode = pstr(p, "mode", "Java-BlindWatermark");
        let shift = mode.contains("fftshift") || mode == "Java-BlindWatermark";
        let mult = pnum(p, "multiplier", 1.0) as f32;
        let log = mode == "Java-BlindWatermark";
        let normalize = !mode.contains("Multiplier"); // Normalization + Java

        let mags: Vec<f32> = if log {
            data.iter().map(|c| (1.0 + c.norm()).ln()).collect()
        } else {
            data.iter().map(|c| c.norm()).collect()
        };
        let maxv = mags.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

        let mut out = RgbaImage::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                // fftshift moves DC (0,0) to the image centre.
                let (sx, sy) = if shift {
                    ((x + w / 2) % w, (y + h / 2) % h)
                } else {
                    (x, y)
                };
                let m = mags[sy * w + sx];
                let v = if normalize { m / maxv * 255.0 } else { m * mult };
                let v = v.clamp(0.0, 255.0) as u8;
                out.put_pixel(x as u32, y as u32, Rgba([v, v, v, 255]));
            }
        }
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "blind_watermark",
            IMG,
            "盲水印 (FFT)",
            INDIGO,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("image", "频谱/水印", PortType::Image),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select(
                    "mode",
                    "模式",
                    &[
                        "Java-BlindWatermark",
                        "FFT(Multiplier)",
                        "FFT(fftshiftMultiplier)",
                        "FFT(Normalization)",
                        "FFT(fftshift_Normalization)",
                    ],
                    "Java-BlindWatermark",
                ),
                ParamSpec::select("channel", "通道", &["灰度", "R", "G", "B"], "灰度"),
                ParamSpec::number("multiplier", "乘数(Multiplier 模式)", 0.0, 100000.0, 0.1, 1.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
