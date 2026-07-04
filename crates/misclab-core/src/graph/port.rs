//! Typed port values — the dataflow currency — and port types for connection
//! validation. The `PortType` string encoding is shared verbatim with the
//! frontend so palette wiring and backend validation never drift.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::{ArtifactId, Fingerprint};

/// A scored candidate string (e.g. from an auto-decoder or text-scoring node).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredString {
    pub text: String,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The typed value that flows along an edge. Large binaries should prefer
/// [`PortValue::Artifact`] (out-of-band handle) over inline [`PortValue::Bytes`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
// Variants intentionally kept unboxed; PortValue is moved, not stored en masse.
#[allow(clippy::large_enum_variant)]
pub enum PortValue {
    None,
    Text(String),
    Number(f64),
    Bool(bool),
    Json(serde_json::Value),
    StringList(Vec<String>),
    Candidates(Vec<ScoredString>),
    Bytes(Arc<[u8]>),
    Artifact(ArtifactId),
    /// An image as a data URL (`data:image/png;base64,…`) for inline display.
    Image(String),
    Fingerprint(Fingerprint),
}

/// The type of a port, for connection validation. `Any` matches anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortType {
    Any,
    Text,
    Number,
    Bool,
    Json,
    StringList,
    Candidates,
    Bytes,
    Artifact,
    Image,
    Fingerprint,
}

impl PortValue {
    pub fn port_type(&self) -> PortType {
        match self {
            PortValue::None => PortType::Any,
            PortValue::Text(_) => PortType::Text,
            PortValue::Number(_) => PortType::Number,
            PortValue::Bool(_) => PortType::Bool,
            PortValue::Json(_) => PortType::Json,
            PortValue::StringList(_) => PortType::StringList,
            PortValue::Candidates(_) => PortType::Candidates,
            PortValue::Bytes(_) => PortType::Bytes,
            PortValue::Artifact(_) => PortType::Artifact,
            PortValue::Image(_) => PortType::Image,
            PortValue::Fingerprint(_) => PortType::Fingerprint,
        }
    }

    pub fn as_text(&self) -> Result<&str, CoreError> {
        match self {
            PortValue::Text(s) => Ok(s),
            other => Err(CoreError::Type(format!(
                "expected Text, got {:?}",
                other.port_type()
            ))),
        }
    }

    pub fn as_bytes(&self) -> Result<Arc<[u8]>, CoreError> {
        match self {
            PortValue::Bytes(b) => Ok(b.clone()),
            other => Err(CoreError::Type(format!(
                "expected Bytes, got {:?}",
                other.port_type()
            ))),
        }
    }
}

impl PortType {
    /// Can a source of type `src` connect into an input of type `self`?
    ///
    /// Besides `Any` and exact matches, a `Text` input accepts scalar/list
    /// sources (`Number` / `Bool` / `StringList`); the value is coerced to its
    /// string form at the node boundary (see the executor's input coercion), so
    /// e.g. a width/height number can drive a text field or 文本输出.
    pub fn accepts(self, src: PortType) -> bool {
        if self == PortType::Any || src == PortType::Any || self == src {
            return true;
        }
        matches!(
            (self, src),
            (PortType::Text, PortType::Number)
                | (PortType::Text, PortType::Bool)
                | (PortType::Text, PortType::StringList)
        )
    }

    pub fn validate(self, value: &PortValue) -> bool {
        self.accepts(value.port_type())
    }
}
