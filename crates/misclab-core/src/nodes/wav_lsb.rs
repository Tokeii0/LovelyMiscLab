//! Extract data hidden in the least-significant bits of WAV PCM samples — the
//! audio counterpart of image LSB stego (as produced by tools like stegolsb /
//! wavsteg). Reads the low N bits of each sample and packs them MSB-first into
//! bytes.
use super::audio_util::{decode_wav, Audio};
use super::image_util::input_bytes;
use super::prelude::*;

/// Pull `nbits` LSBs from each selected sample and pack them into bytes.
fn extract_lsb(a: &Audio, nbits: u32, channel: &str, msb_first: bool) -> Vec<u8> {
    let ch = a.channels.max(1) as usize;
    let start = match channel {
        "左声道" => 0,
        "右声道" => 1.min(ch - 1),
        _ => 0,
    };
    let step = if channel == "全部(交错)" { 1 } else { ch };

    let mut bits: Vec<u8> = Vec::new();
    let mut i = start;
    while i < a.ints.len() {
        let s = a.ints[i] as u32;
        if msb_first {
            for k in (0..nbits).rev() {
                bits.push(((s >> k) & 1) as u8);
            }
        } else {
            for k in 0..nbits {
                bits.push(((s >> k) & 1) as u8);
            }
        }
        i += step;
    }

    let mut out = Vec::with_capacity(bits.len() / 8);
    for chunk in bits.chunks_exact(8) {
        let mut b = 0u8;
        for &bit in chunk {
            b = (b << 1) | bit;
        }
        out.push(b);
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
        let a = decode_wav(&bytes)?;
        if a.ints.is_empty() {
            return Err(CoreError::Other(
                "这是浮点 WAV，没有可用于 LSB 提取的整数采样。".into(),
            ));
        }
        let nbits = (pnum(p, "bits", 1.0) as u32).clamp(1, 8);
        let channel = pstr(p, "channel", "全部(交错)");
        let msb_first = pstr(p, "bitOrder", "MSB优先") == "MSB优先";

        let out = extract_lsb(&a, nbits, channel, msb_first);
        let preview = String::from_utf8_lossy(&out).into_owned();

        let mut m = PortMap::new();
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(out.clone().into_boxed_slice())),
        );
        m.insert("text".into(), PortValue::Text(preview));
        m.insert("hex".into(), PortValue::Text(hex::encode(&out[..out.len().min(4096)])));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "wav_lsb",
            AUD,
            "WAV LSB 提取",
            FUCHSIA,
            vec![req("data", "音频", PortType::Any)],
            vec![
                req("bytes", "字节", PortType::Bytes),
                opt("text", "文本", PortType::Text),
                opt("hex", "hex预览", PortType::Text),
            ],
            vec![
                ParamSpec::number("bits", "每采样取LSB位数", 1.0, 8.0, 1.0, 1.0),
                ParamSpec::select(
                    "channel",
                    "声道",
                    &["全部(交错)", "左声道", "右声道"],
                    "全部(交错)",
                ),
                ParamSpec::select("bitOrder", "位序", &["MSB优先", "LSB优先"], "MSB优先"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::super::audio_util::{decode_wav, encode_wav_i16};
    use super::*;

    #[test]
    fn lsb_roundtrip() {
        let msg = b"flag{audio_lsb}";
        // message bits, MSB-first per byte
        let mut bits = Vec::new();
        for &byte in msg {
            for k in (0..8).rev() {
                bits.push((byte >> k) & 1);
            }
        }
        // one mono sample per bit, LSB carries the bit
        let samples: Vec<i16> = bits
            .iter()
            .enumerate()
            .map(|(i, &bit)| (((i as i16) % 100) & !1) | bit as i16)
            .collect();
        let wav = encode_wav_i16(&samples, 8000, 1);
        let a = decode_wav(&wav).unwrap();
        let out = extract_lsb(&a, 1, "全部(交错)", true);
        assert!(out.starts_with(msg), "got: {:?}", &out[..out.len().min(20)]);
    }
}
