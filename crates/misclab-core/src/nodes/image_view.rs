//! 图片查看：把上游的图片（Image 数据URL / 原始字节 / data:URL 文本）直接在节点上
//! 展示，并原样输出为 Image 端口以便继续连接。纯展示 + 透传，不改动像素。
use base64::Engine as _;

use super::prelude::*;

/// 由图片首部魔数推断 MIME 类型（用于拼 data:URL）。
fn mime_from_magic(b: &[u8]) -> &'static str {
    if b.starts_with(&[0x89, 0x50, 0x4e, 0x47]) {
        "image/png"
    } else if b.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if b.starts_with(b"GIF8") {
        "image/gif"
    } else if b.starts_with(b"BM") {
        "image/bmp"
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/png"
    }
}

fn bytes_to_data_url(b: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime_from_magic(b),
        base64::engine::general_purpose::STANDARD.encode(b)
    )
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let url = match i.get("image") {
            // 已经是可直接展示的图片端口（data:/http URL），原样透传。
            Some(PortValue::Image(u)) if !u.trim().is_empty() => u.clone(),
            _ => {
                let bytes = in_bytes(i, "image")?;
                if bytes.is_empty() {
                    return Err(CoreError::Other("图片为空".into()));
                }
                bytes_to_data_url(&bytes)
            }
        };
        Ok(one("image", PortValue::Image(url)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        {
            let mut d = desc(
                "image_view",
                IO,
                "图片查看",
                SLATE,
                vec![req("image", "图片", PortType::Any)],
                vec![req("image", "图片", PortType::Image)],
                vec![],
            );
            d.description = "把上游图片直接在节点上展示（接受图片/字节/data:URL），并原样输出以便继续连接。".into();
            d
        },
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    #[test]
    fn bytes_become_image_data_url() {
        // Minimal PNG signature → detected as image/png and wrapped as a data URL.
        let png = [0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4];
        let mut inputs = PortMap::new();
        inputs.insert("image".into(), PortValue::Bytes(Arc::from(png.to_vec().into_boxed_slice())));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "image_view",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("image") {
            Some(PortValue::Image(u)) => assert!(u.starts_with("data:image/png;base64,"), "{u}"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn image_url_passes_through() {
        let mut inputs = PortMap::new();
        inputs.insert("image".into(), PortValue::Image("data:image/gif;base64,AAAA".into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "image_view",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("image"), Some(PortValue::Image(u)) if u == "data:image/gif;base64,AAAA"));
    }
}
