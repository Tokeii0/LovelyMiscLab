//! The serializable node descriptor — the single source of truth for a node's
//! signature and UI. The frontend renders the palette entry, the node body
//! (ports + inline param widgets), and connection validation entirely from this.

use serde::{Deserialize, Serialize};

use crate::graph::port::PortType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cost {
    Cheap,
    Medium,
    Heavy,
}

/// One input or output port on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortSpec {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub port_type: PortType,
    pub required: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

impl PortSpec {
    pub fn new(name: &str, label: &str, port_type: PortType, required: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            port_type,
            required,
            description: String::new(),
        }
    }
}

/// The UI control for a parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ParamWidget {
    Text { multiline: bool },
    Number { min: f64, max: f64, step: f64 },
    Slider { min: f64, max: f64, step: f64 },
    Select { options: Vec<String> },
    Toggle,
    /// A file picker; the param value is the chosen path.
    File,
}

/// One configurable parameter (rendered in the node body and properties panel).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamSpec {
    pub name: String,
    pub label: String,
    pub widget: ParamWidget,
    pub default: serde_json::Value,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

impl ParamSpec {
    pub fn text(name: &str, label: &str, default: &str, multiline: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            widget: ParamWidget::Text { multiline },
            default: serde_json::Value::String(default.into()),
            description: String::new(),
        }
    }

    pub fn toggle(name: &str, label: &str, default: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            widget: ParamWidget::Toggle,
            default: serde_json::Value::Bool(default),
            description: String::new(),
        }
    }

    pub fn select(name: &str, label: &str, options: &[&str], default: &str) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            widget: ParamWidget::Select {
                options: options.iter().map(|s| s.to_string()).collect(),
            },
            default: serde_json::Value::String(default.into()),
            description: String::new(),
        }
    }

    pub fn number(name: &str, label: &str, min: f64, max: f64, step: f64, default: f64) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            widget: ParamWidget::Number { min, max, step },
            default: serde_json::json!(default),
            description: String::new(),
        }
    }

    pub fn file(name: &str, label: &str) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            widget: ParamWidget::File,
            default: serde_json::Value::String(String::new()),
            description: String::new(),
        }
    }
}

/// Everything the frontend needs to render and wire a node type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDescriptor {
    pub id: String,
    pub category: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub color: String,
    pub inputs: Vec<PortSpec>,
    pub outputs: Vec<PortSpec>,
    #[serde(default)]
    pub params: Vec<ParamSpec>,
    pub cost: Cost,
}
