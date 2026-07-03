//! Parse a WAV header and report its format — the first thing you check on an
//! audio-stego challenge (sample rate hints at what a spectrogram will show;
//! channel count hints at per-channel hiding).
use super::audio_util::decode_wav;
use super::image_util::input_bytes;
use super::prelude::*;

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
        let report = format!(
            "采样率: {} Hz\n声道: {}\n位深: {} bit{}\n时长: {:.3} s\n帧数(每声道采样): {}\n总采样数: {}\n奈奎斯特频率: {} Hz",
            a.sample_rate,
            a.channels,
            a.bits,
            if a.is_float { " (float)" } else { "" },
            a.duration_secs(),
            a.frames(),
            a.floats.len(),
            a.sample_rate / 2,
        );
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(report));
        m.insert("sampleRate".into(), PortValue::Number(a.sample_rate as f64));
        m.insert("channels".into(), PortValue::Number(a.channels as f64));
        m.insert("duration".into(), PortValue::Number(a.duration_secs()));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "audio_info",
            AUD,
            "音频信息",
            FUCHSIA,
            vec![req("data", "音频", PortType::Any)],
            vec![
                req("text", "信息", PortType::Text),
                opt("sampleRate", "采样率", PortType::Number),
                opt("channels", "声道数", PortType::Number),
                opt("duration", "时长(秒)", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
