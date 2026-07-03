//! Shared WAV decoding for the audio nodes. Uses `hound` (pure-Rust WAV I/O).
//! Only uncompressed WAV/PCM is supported — for MP3/OGG/FLAC convert to WAV first.
use std::io::Cursor;

use super::prelude::*;

/// Decoded PCM audio: interleaved integer samples (for LSB) + normalized floats
/// (for DSP/rendering), plus format metadata.
pub struct Audio {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
    pub is_float: bool,
    /// Interleaved integer samples (empty for float WAV).
    pub ints: Vec<i32>,
    /// Interleaved samples normalized to roughly [-1, 1].
    pub floats: Vec<f32>,
}

impl Audio {
    /// Frames = samples per channel.
    pub fn frames(&self) -> usize {
        let ch = self.channels.max(1) as usize;
        self.floats.len() / ch
    }

    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.frames() as f64 / self.sample_rate as f64
        }
    }

    /// Mono downmix (average of all channels).
    pub fn mono(&self) -> Vec<f32> {
        let ch = self.channels.max(1) as usize;
        if ch == 1 {
            return self.floats.clone();
        }
        self.floats
            .chunks(ch)
            .map(|f| f.iter().sum::<f32>() / ch as f32)
            .collect()
    }

    /// One channel's float samples (index clamped to the available channels).
    pub fn channel(&self, idx: usize) -> Vec<f32> {
        let ch = self.channels.max(1) as usize;
        let idx = idx.min(ch - 1);
        self.floats.iter().skip(idx).step_by(ch).copied().collect()
    }
}

pub fn decode_wav(bytes: &[u8]) -> Result<Audio, CoreError> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(|e| {
        CoreError::Parse(format!(
            "WAV 解析失败（仅支持 WAV/PCM，MP3/OGG/FLAC 请先转成 WAV）：{e}"
        ))
    })?;
    let spec = reader.spec();
    let bits = spec.bits_per_sample;
    let mut ints = Vec::new();
    let mut floats;
    match spec.sample_format {
        hound::SampleFormat::Int => {
            ints = reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| CoreError::Parse(format!("读取采样失败：{e}")))?;
            let scale = if bits >= 1 { (1i64 << (bits - 1)) as f32 } else { 1.0 };
            floats = Vec::with_capacity(ints.len());
            floats.extend(ints.iter().map(|&s| (s as f32 / scale).clamp(-1.0, 1.0)));
        }
        hound::SampleFormat::Float => {
            floats = reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| CoreError::Parse(format!("读取采样失败：{e}")))?;
        }
    }
    Ok(Audio {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        bits,
        is_float: matches!(spec.sample_format, hound::SampleFormat::Float),
        ints,
        floats,
    })
}

#[cfg(test)]
pub fn encode_wav_i16(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = hound::WavWriter::new(&mut buf, spec).unwrap();
        for &s in samples {
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
    }
    buf.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_wav() {
        // 1s of stereo 16-bit at 48 kHz.
        let wav = encode_wav_i16(&[1000i16; 48000 * 2], 48000, 2);
        let a = decode_wav(&wav).unwrap();
        assert_eq!(a.sample_rate, 48000);
        assert_eq!(a.channels, 2);
        assert_eq!(a.bits, 16);
        assert_eq!(a.frames(), 48000);
        assert!((a.duration_secs() - 1.0).abs() < 1e-6);
        assert_eq!(a.ints.len(), 48000 * 2);
        // 1000 / 32768 ≈ 0.0305
        assert!((a.floats[0] - 1000.0 / 32768.0).abs() < 1e-4);
    }
}
