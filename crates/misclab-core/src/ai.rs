//! AI model configuration and OpenAI-compatible chat/vision calls used by AI
//! nodes. Kept small and synchronous (nodes run on `spawn_blocking`).

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub model: String,
    pub api_key: String,
    /// Base URL, e.g. `https://api.openai.com/v1`.
    pub base_url: String,
}

/// The two configurable models (text LLM + vision).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub llm: ModelConfig,
    pub vision: ModelConfig,
}

impl ModelConfig {
    pub fn is_configured(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.model.trim().is_empty()
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

fn content_of(v: &serde_json::Value) -> String {
    v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn post(cfg: &ModelConfig, body: serde_json::Value) -> Result<String, CoreError> {
    let resp = ureq::post(&cfg.endpoint())
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| CoreError::Other(format!("AI 请求失败: {e}")))?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| CoreError::Other(format!("AI 响应解析失败: {e}")))?;
    Ok(content_of(&json))
}

/// A single-turn chat completion.
pub fn chat(cfg: &ModelConfig, system: &str, user: &str) -> Result<String, CoreError> {
    if !cfg.is_configured() {
        return Err(CoreError::Other("AI 文本模型未配置（请在设置中填写）".into()));
    }
    let mut messages = Vec::new();
    if !system.trim().is_empty() {
        messages.push(serde_json::json!({ "role": "system", "content": system }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": user }));
    post(
        cfg,
        serde_json::json!({ "model": cfg.model, "messages": messages, "temperature": 0 }),
    )
}

/// A vision completion — `image_url` may be an http(s) URL or a data URL.
pub fn vision(cfg: &ModelConfig, prompt: &str, image_url: &str) -> Result<String, CoreError> {
    if !cfg.is_configured() {
        return Err(CoreError::Other("AI 识图模型未配置（请在设置中填写）".into()));
    }
    let content = serde_json::json!([
        { "type": "text", "text": prompt },
        { "type": "image_url", "image_url": { "url": image_url } },
    ]);
    post(
        cfg,
        serde_json::json!({ "model": cfg.model, "messages": [{ "role": "user", "content": content }] }),
    )
}
