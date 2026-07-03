//! Decode DTMF (touch-tone) dialing tones from a WAV into digits. Each key is a
//! sum of one low + one high frequency; a per-frame Goertzel filter at the eight
//! DTMF frequencies recovers the pressed keys. A staple of phone-audio CTFs.
use super::audio_util::decode_wav;
use super::image_util::input_bytes;
use super::prelude::*;

const LOW: [f32; 4] = [697.0, 770.0, 852.0, 941.0];
const HIGH: [f32; 4] = [1209.0, 1336.0, 1477.0, 1633.0];
const KEYS: [[char; 4]; 4] = [
    ['1', '2', '3', 'A'],
    ['4', '5', '6', 'B'],
    ['7', '8', '9', 'C'],
    ['*', '0', '#', 'D'],
];

/// Goertzel magnitude of `frame` at `freq` (works for any frequency).
fn goertzel(frame: &[f32], sr: f32, freq: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * freq / sr;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in frame {
        let s0 = x + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0).sqrt()
}

fn argmax(v: &[f32]) -> (usize, f32) {
    let mut idx = 0;
    let mut m = v[0];
    for (i, &x) in v.iter().enumerate() {
        if x > m {
            m = x;
            idx = i;
        }
    }
    (idx, m)
}

fn second_max(v: &[f32], skip: usize) -> f32 {
    v.iter()
        .enumerate()
        .filter(|(i, _)| *i != skip)
        .map(|(_, &x)| x)
        .fold(0.0f32, f32::max)
}

fn decode_dtmf(signal: &[f32], sr: u32) -> String {
    let sr = sr as f32;
    let frame_len = (sr * 0.04) as usize; // 40 ms analysis window
    if frame_len < 8 || signal.len() < frame_len {
        return String::new();
    }
    let hop = (frame_len / 2).max(1);

    let mut seq: Vec<Option<char>> = Vec::new();
    let mut pos = 0;
    while pos + frame_len <= signal.len() {
        let frame = &signal[pos..pos + frame_len];
        let energy: f32 = frame.iter().map(|x| x * x).sum::<f32>() / frame_len as f32;
        let low: Vec<f32> = LOW.iter().map(|&f| goertzel(frame, sr, f)).collect();
        let high: Vec<f32> = HIGH.iter().map(|&f| goertzel(frame, sr, f)).collect();
        let (li, lmax) = argmax(&low);
        let (hi, hmax) = argmax(&high);
        let l2 = second_max(&low, li);
        let h2 = second_max(&high, hi);
        // A valid tone: a clear winner in each group, both tones roughly balanced,
        // and enough signal energy to not be silence/noise.
        let ok = energy > 1e-4
            && lmax > 3.0 * l2
            && hmax > 3.0 * h2
            && lmax.max(hmax) < 8.0 * lmax.min(hmax).max(1e-9);
        seq.push(ok.then(|| KEYS[li][hi]));
        pos += hop;
    }

    // One character per held tone; a silence gap separates repeated digits.
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for cell in seq {
        match cell {
            Some(c) => {
                if prev != Some(c) {
                    out.push(c);
                }
                prev = Some(c);
            }
            None => prev = None,
        }
    }
    out
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bytes = input_bytes(inputs, "data")?;
        let a = decode_wav(&bytes)?;
        let digits = decode_dtmf(&a.mono(), a.sample_rate);
        let report = if digits.is_empty() {
            "未检测到 DTMF 音（可能不是拨号音、噪声过大或已加窗）。".to_string()
        } else {
            format!("检测到 {} 位：{digits}", digits.chars().count())
        };
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(digits));
        m.insert("report".into(), PortValue::Text(report));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "dtmf_decode",
            AUD,
            "DTMF 拨号音解码",
            FUCHSIA,
            vec![req("data", "音频", PortType::Any)],
            vec![
                req("text", "数字", PortType::Text),
                opt("report", "说明", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(f: [f32; 2], sr: f32, secs: f32) -> Vec<f32> {
        let n = (sr * secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr;
                0.5 * (2.0 * std::f32::consts::PI * f[0] * t).sin()
                    + 0.5 * (2.0 * std::f32::consts::PI * f[1] * t).sin()
            })
            .collect()
    }

    #[test]
    fn decodes_synthetic_dtmf() {
        let sr = 8000.0f32;
        // '1'=(697,1209) '5'=(770,1336) '9'=(852,1477)
        let digits = [[697.0, 1209.0], [770.0, 1336.0], [852.0, 1477.0]];
        let mut signal = Vec::new();
        for d in digits {
            signal.extend(tone(d, sr, 0.15)); // 150 ms tone
            signal.extend(vec![0.0f32; (sr * 0.08) as usize]); // 80 ms silence
        }
        assert_eq!(decode_dtmf(&signal, 8000), "159");
    }
}
