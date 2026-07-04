//! Render a WAV's spectrogram (STFT magnitude) as an image. The classic audio
//! CTF trick hides text or a picture in the frequency domain — invisible on the
//! waveform, obvious on a spectrogram. Time → X, frequency → Y (low at bottom),
//! magnitude → brightness.
use image::{Rgba, RgbaImage};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use super::audio_util::decode_wav;
use super::image_util::{data_url, input_bytes, to_png};
use super::prelude::*;

const MAX_W: usize = 2400; // cap columns (long files) by widening the hop
const MAX_H: usize = 1024; // cap rows; extra bins are max-pooled

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let s = (std::f32::consts::PI * i as f32 / (n as f32 - 1.0).max(1.0)).sin();
            s * s
        })
        .collect()
}

/// 5-stop approximation of the magma colormap (dark → purple → orange → cream).
fn magma(t: f32) -> [u8; 3] {
    const STOPS: [(f32, [f32; 3]); 5] = [
        (0.0, [0.0, 0.0, 4.0]),
        (0.25, [51.0, 16.0, 88.0]),
        (0.5, [131.0, 39.0, 110.0]),
        (0.75, [222.0, 73.0, 63.0]),
        (1.0, [252.0, 253.0, 191.0]),
    ];
    let t = t.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return [
                (c0[0] + (c1[0] - c0[0]) * f) as u8,
                (c0[1] + (c1[1] - c0[1]) * f) as u8,
                (c0[2] + (c1[2] - c0[2]) * f) as u8,
            ];
        }
    }
    [252, 253, 191]
}

/// STFT → image. `signal` is mono; `hop` is the frame advance in samples.
fn render(signal: &[f32], fft_size: usize, hop: usize, dynamic_range: f32, grayscale: bool) -> Option<RgbaImage> {
    let n = fft_size;
    if signal.len() < n || hop == 0 {
        return None;
    }
    let bins = n / 2;
    let win = hann(n);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    let frames = (signal.len() - n) / hop + 1;

    let mut cols: Vec<Vec<f32>> = Vec::with_capacity(frames);
    let mut buf = vec![Complex::default(); n];
    let mut global_max = f32::MIN;
    for f in 0..frames {
        let base = f * hop;
        for i in 0..n {
            buf[i] = Complex::new(signal[base + i] * win[i], 0.0);
        }
        fft.process(&mut buf);
        let mut col = vec![0f32; bins];
        for (b, slot) in col.iter_mut().enumerate() {
            let db = 20.0 * (buf[b].norm() + 1e-9).log10();
            *slot = db;
            if db > global_max {
                global_max = db;
            }
        }
        cols.push(col);
    }

    let floor = global_max - dynamic_range;
    let span = (global_max - floor).max(1e-6);
    let width = frames;
    let height = bins.min(MAX_H);
    let mut img = RgbaImage::new(width as u32, height as u32);
    for (x, col) in cols.iter().enumerate() {
        for y in 0..height {
            // Bottom rows = low frequency, top rows = high frequency; max-pool the
            // bin range that falls on this row so thin lines survive downsampling.
            let rb = height - 1 - y;
            let lo = (rb * bins / height).min(bins - 1);
            let hi = ((rb + 1) * bins / height).clamp(lo + 1, bins);
            let mut db = f32::MIN;
            for &v in &col[lo..hi] {
                if v > db {
                    db = v;
                }
            }
            let t = ((db - floor) / span).clamp(0.0, 1.0);
            let rgb = if grayscale {
                let v = (t * 255.0) as u8;
                [v, v, v]
            } else {
                magma(t)
            };
            img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], 255]));
        }
    }
    Some(img)
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
        let a = decode_wav(&bytes)?;
        let signal = match pstr(p, "channel", "混合") {
            "左声道" => a.channel(0),
            "右声道" => a.channel(1),
            _ => a.mono(),
        };
        let n = match pstr(p, "fftSize", "1024") {
            "512" => 512,
            "2048" => 2048,
            "4096" => 4096,
            _ => 1024,
        };
        if signal.len() < n {
            return Err(CoreError::Other("音频太短，无法生成频谱图。".into()));
        }
        let overlap = match pstr(p, "overlap", "75%") {
            "50%" => 0.5,
            "87.5%" => 0.875,
            _ => 0.75,
        };
        let mut hop = ((n as f64) * (1.0 - overlap)).round().max(1.0) as usize;
        // Widen the hop for long files so the image stays within MAX_W columns.
        if (signal.len() - n) / hop + 1 > MAX_W {
            hop = ((signal.len() - n) / (MAX_W - 1)).max(1);
        }
        let dr = pnum(p, "dynamicRange", 80.0) as f32;
        let grayscale = pstr(p, "colormap", "彩色") == "灰度";

        let img = render(&signal, n, hop, dr, grayscale)
            .ok_or_else(|| CoreError::Other("生成频谱图失败。".into()))?;
        let png = to_png(&img)?;
        let (w, h) = img.dimensions();
        let freq_res = a.sample_rate as f32 / n as f32;
        let time_res = hop as f32 / a.sample_rate.max(1) as f32 * 1000.0;
        let report = format!(
            "频谱图 {w}×{h}\nFFT: {n}  跳步: {hop} 采样\n频率分辨率: {freq_res:.1} Hz/bin\n时间分辨率: {time_res:.1} ms/列\n纵轴: 0–{} Hz（下低上高）",
            a.sample_rate / 2
        );

        let mut m = PortMap::new();
        m.insert("image".into(), PortValue::Image(data_url(&png, "image/png")));
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(png.into_boxed_slice())),
        );
        m.insert("report".into(), PortValue::Text(report));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "audio_spectrogram",
            AUD,
            "音频频谱图",
            FUCHSIA,
            vec![req("data", "音频", PortType::Any)],
            vec![
                req("image", "频谱图", PortType::Image),
                opt("bytes", "PNG字节", PortType::Bytes),
                opt("report", "参数", PortType::Text),
            ],
            vec![
                ParamSpec::select("channel", "声道", &["混合", "左声道", "右声道"], "混合"),
                ParamSpec::select("fftSize", "FFT 窗口", &["512", "1024", "2048", "4096"], "1024"),
                ParamSpec::select("overlap", "重叠", &["50%", "75%", "87.5%"], "75%"),
                ParamSpec::select("colormap", "配色", &["彩色", "灰度"], "彩色"),
                ParamSpec::number("dynamicRange", "动态范围(dB)", 30.0, 120.0, 5.0, 80.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_row_matches_tone_frequency() {
        // 1 kHz sine at 8 kHz → bin 128 of 512; low freq at the bottom, so the
        // bright row should sit ~1/4 up from the bottom.
        let sr = 8000.0f32;
        let n = 1024;
        let signal: Vec<f32> = (0..4000)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr).sin())
            .collect();
        let img = render(&signal, n, 256, 80.0, true).expect("image");
        let (w, h) = img.dimensions();
        assert!(w > 0 && h == 512);

        // brightest row by total luminance
        let mut best_row = 0u32;
        let mut best_sum = -1i64;
        for y in 0..h {
            let sum: i64 = (0..w).map(|x| img.get_pixel(x, y).0[0] as i64).sum();
            if sum > best_sum {
                best_sum = sum;
                best_row = y;
            }
        }
        // bin 128 → row from bottom 128 → y = 512-1-128 = 383
        assert!((best_row as i32 - 383).abs() <= 4, "peak row {best_row}, expected ~383");
    }
}
