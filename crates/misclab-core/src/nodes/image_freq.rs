//! Frequency-domain analysis: 2-D DFT log-magnitude spectrum (DC centered).
//! Hidden text/patterns often show up here even when invisible in the pixels.
use image::{Rgba, RgbaImage};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use super::image_util::*;
use super::prelude::*;

struct N;
impl Node for N {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (w, h) = (img.width() as usize, img.height() as usize);
        if w == 0 || h == 0 {
            return Err(CoreError::Other("空图".into()));
        }
        // Grayscale → complex plane.
        let mut data: Vec<Complex<f32>> =
            img.pixels().map(|p| Complex::new(luma(p.0[0], p.0[1], p.0[2]) as f32, 0.0)).collect();

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

        let mags: Vec<f32> = data.iter().map(|c| (1.0 + c.norm()).ln()).collect();
        let maxv = mags.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
        let mut out = RgbaImage::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                // fftshift: move DC (0,0) to the image centre.
                let sx = (x + w / 2) % w;
                let sy = (y + h / 2) % h;
                let v = (mags[sy * w + sx] / maxv * 255.0).clamp(0.0, 255.0) as u8;
                out.put_pixel(x as u32, y as u32, Rgba([v, v, v, 255]));
            }
        }
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "dft_spectrum",
            IMG,
            "频谱 (DFT)",
            INDIGO,
            vec![req("data", "图片", PortType::Any)],
            vec![req("image", "频谱图", PortType::Image), opt("bytes", "字节", PortType::Bytes)],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
