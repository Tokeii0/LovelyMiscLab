use super::prelude::*;
use base64::Engine as _;

/// Turn the input (http/https/data URL, or a local file path) into an image URL.
fn to_image_url(input: &str) -> Result<String, CoreError> {
    let s = input.trim();
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("data:") {
        return Ok(s.to_string());
    }
    let bytes =
        std::fs::read(s).map_err(|e| CoreError::Other(format!("读取图片失败: {e}")))?;
    let mime = if s.ends_with(".jpg") || s.ends_with(".jpeg") {
        "image/jpeg"
    } else if s.ends_with(".gif") {
        "image/gif"
    } else if s.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

/// Ask the configured vision model about an image.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let image = in_text(inputs, "image")?;
        let prompt = pstr(params, "prompt", "识别图片中的文字或 flag，只输出结果。");
        let url = to_image_url(image)?;
        let answer = ai::vision(&ctx.env.ai.vision, prompt, &url)?;
        Ok(out_text(answer))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "ai_vision",
            AI,
            "AI 识图",
            EMERALD,
            vec![req("image", "图片(路径/URL)", PortType::Text)],
            vec![req("text", "识别结果", PortType::Text)],
            vec![ParamSpec::text(
                "prompt",
                "提示",
                "识别图片中的文字或 flag，只输出结果。",
                true,
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
