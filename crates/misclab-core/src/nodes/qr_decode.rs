use super::prelude::*;

/// Decode 2D barcodes (QR, DataMatrix, Aztec, …) from image bytes via rxing.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bytes = inputs
            .get("image")
            .ok_or_else(|| CoreError::MissingInput("image".into()))?
            .as_bytes()?;
        let dynimg = image::load_from_memory(bytes.as_ref())
            .map_err(|e| CoreError::Parse(format!("图片解析失败: {e}")))?;
        let luma = dynimg.to_luma8();
        let (w, h) = luma.dimensions();

        let results =
            rxing::helpers::detect_multiple_in_luma(luma.into_raw(), w, h).unwrap_or_default();
        let texts: Vec<String> = results.iter().map(|r| r.getText().to_string()).collect();
        let format = results
            .first()
            .map(|r| format!("{:?}", r.getBarcodeFormat()))
            .unwrap_or_default();

        let mut out = PortMap::new();
        out.insert(
            "text".to_string(),
            PortValue::Text(texts.first().cloned().unwrap_or_default()),
        );
        out.insert("all".to_string(), PortValue::StringList(texts));
        out.insert("format".to_string(), PortValue::Text(format));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "qr_decode",
            ENC,
            "二维码解码",
            TEAL,
            vec![req("image", "图片字节", PortType::Bytes)],
            vec![
                req("text", "内容", PortType::Text),
                opt("all", "全部", PortType::StringList),
                opt("format", "格式", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
