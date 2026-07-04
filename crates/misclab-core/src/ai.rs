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

// ---- multi-turn tool-calling (agent loop) ----------------------------------

/// One function the model may call, as an OpenAI-compatible tool. `parameters`
/// is a JSON-Schema object describing the arguments.
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A single tool invocation the model asked for. `arguments` is already parsed
/// to a JSON value (models emit it as a JSON *string*; we decode it here).
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// The outcome of one turn: either the model called tools, or it answered in
/// plain text (which also happens when an endpoint ignores `tools` entirely).
pub enum AssistantTurn {
    ToolCalls {
        /// The assistant message verbatim (carries `tool_calls` + ids) — push it
        /// back into the transcript so the provider can match tool results.
        raw_assistant_msg: serde_json::Value,
        calls: Vec<ToolCall>,
    },
    Content(String),
}

/// Token accounting from a response's `usage` field (0 when the provider omits
/// it). `total_tokens` ≈ the running transcript size, so the agent loop can stop
/// before it outgrows the model's context window.
#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

fn parse_tool_call(tc: &serde_json::Value) -> Option<ToolCall> {
    let name = tc["function"]["name"].as_str()?.to_string();
    let id = tc["id"].as_str().unwrap_or_default().to_string();
    // Providers send `arguments` as a JSON string; some send a bare object.
    // On malformed JSON, yield Null so the caller can answer with a validation
    // error (self-correction) instead of aborting the whole run.
    let arguments = match &tc["function"]["arguments"] {
        serde_json::Value::String(s) => serde_json::from_str(s).unwrap_or(serde_json::Value::Null),
        obj @ serde_json::Value::Object(_) => obj.clone(),
        _ => serde_json::Value::Null,
    };
    Some(ToolCall { id, name, arguments })
}

/// One step of a tool-calling conversation. `messages` is the running transcript
/// (system + user + prior assistant/tool turns). Returns the model's next move.
pub fn chat_step(
    cfg: &ModelConfig,
    messages: &[serde_json::Value],
    tools: &[ToolDef],
) -> Result<(AssistantTurn, Usage), CoreError> {
    if !cfg.is_configured() {
        return Err(CoreError::Other("AI 文本模型未配置（请在设置中填写）".into()));
    }
    let tools_json: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                },
            })
        })
        .collect();
    let body = serde_json::json!({
        "model": cfg.model,
        "temperature": 0,
        "tool_choice": "auto",
        "messages": messages,
        "tools": tools_json,
    });
    // We need the full message object (not just `content`), so we can't reuse
    // `post()` here.
    let resp = ureq::post(&cfg.endpoint())
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| CoreError::Other(format!("AI 请求失败: {e}")))?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| CoreError::Other(format!("AI 响应解析失败: {e}")))?;
    let usage = Usage {
        prompt_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
        completion_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0),
        total_tokens: json["usage"]["total_tokens"].as_u64().unwrap_or(0),
    };
    let msg = &json["choices"][0]["message"];
    if let Some(tcs) = msg.get("tool_calls").and_then(|v| v.as_array()) {
        if !tcs.is_empty() {
            let calls: Vec<ToolCall> = tcs.iter().filter_map(parse_tool_call).collect();
            if !calls.is_empty() {
                return Ok((
                    AssistantTurn::ToolCalls {
                        raw_assistant_msg: msg.clone(),
                        calls,
                    },
                    usage,
                ));
            }
        }
    }
    Ok((
        AssistantTurn::Content(msg["content"].as_str().unwrap_or_default().to_string()),
        usage,
    ))
}
