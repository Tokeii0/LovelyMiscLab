//! Extract least-significant-bit steganography payload from an image.
//! Reads the chosen bit-plane of the chosen channels in row-major pixel order
//! and packs the bits into bytes.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let img = image::load_from_memory(&data)
            .map_err(|e| CoreError::Other(format!("图片解码失败: {e}")))?
            .to_rgba8();
        let bit = pnum(p, "bit", 0.0).clamp(0.0, 7.0) as u8;
        let chans: Vec<usize> = pstr(p, "channels", "RGB")
            .chars()
            .filter_map(|c| match c.to_ascii_uppercase() {
                'R' => Some(0),
                'G' => Some(1),
                'B' => Some(2),
                'A' => Some(3),
                _ => None,
            })
            .collect();
        if chans.is_empty() {
            return Err(CoreError::Parse("通道至少选一个 (R/G/B/A)".into()));
        }
        let msb_first = pbool(p, "msbFirst", true);

        let mut bits: Vec<u8> = Vec::with_capacity(img.pixels().len() * chans.len());
        for px in img.pixels() {
            for &ch in &chans {
                bits.push((px.0[ch] >> bit) & 1);
            }
        }
        let mut out = Vec::with_capacity(bits.len() / 8);
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                let shift = if msb_first { 7 - i } else { i };
                byte |= b << shift;
            }
            out.push(byte);
        }

        let mut m = PortMap::new();
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(out.clone().into_boxed_slice())),
        );
        m.insert(
            "text".to_string(),
            PortValue::Text(String::from_utf8_lossy(&out).into_owned()),
        );
        m.insert("hex".to_string(), PortValue::Text(hex::encode(&out)));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "lsb_extract",
            STEG,
            "LSB 提取",
            PURPLE,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
                opt("hex", "hex", PortType::Text),
            ],
            vec![
                ParamSpec::text("channels", "通道顺序 (R/G/B/A)", "RGB", false),
                ParamSpec::number("bit", "位平面 (0=最低位)", 0.0, 7.0, 1.0, 0.0),
                ParamSpec::toggle("msbFirst", "高位在前打包", true),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
