//! The data model — this is the IPC contract with the frontend. Every type is
//! `serde`-serializable; `specta::Type` derives are layered on in M2 when these
//! begin crossing the Tauri command boundary.
//!
//! `Finding` is the universal currency: suspicion scoring, the results
//! dashboard, and the right-panel hints are all projections of findings.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ArtifactId = Uuid;

/// The five pipeline stages. Analyzers declare which stage they belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    TypeId = 1,
    Structure = 2,
    Feature = 3,
    Extract = 4,
    Correlate = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// A file's identity + basic forensic metadata. Populates the right-hand panel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fingerprint {
    pub name: String,
    pub size: u64,
    pub mime: Option<String>,
    pub file_type: Option<String>,
    /// 0.0..=1.0 confidence in the type identification.
    pub type_confidence: f32,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub ssdeep: Option<String>,
    pub crc32: Option<u32>,
    /// Unix-epoch milliseconds.
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
    /// First 64 bytes, for the magic-byte hex viewer.
    pub magic_head: Vec<u8>,
    pub overall_entropy: f32,
}

/// Typed evidence attached to a finding. Byte samples are capped (≤256B); bulk
/// data lives out-of-band in an [`Artifact`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "data")]
pub enum Evidence {
    Bytes { offset: u64, len: u64, sample: Vec<u8> },
    Text { value: String },
    Kv { pairs: Vec<(String, String)> },
    Histogram { channel: String, bins: Vec<u32> },
    EntropyRegion { start: u64, end: u64, value: f32 },
    Image { artifact: ArtifactId },
    None,
}

/// The universal unit of analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: Uuid,
    /// The emitting analyzer's stable id.
    pub source: String,
    pub stage: Stage,
    pub title: String,
    pub detail: String,
    pub severity: Severity,
    /// 0.0..=1.0.
    pub confidence: f32,
    /// Raw contribution to the suspicion score before confidence/severity scaling.
    pub score_weight: f32,
    /// Free-form tags, e.g. ["steg","png","lsb"] — drive filtering + module routing.
    pub tags: Vec<String>,
    pub evidence: Evidence,
    pub artifact_refs: Vec<ArtifactId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    ExtractedFile,
    StringsDump,
    CarvedChild,
    DecodedBlob,
    Spectrogram,
    BitPlane,
    Preview,
    Other,
}

/// Where an artifact's bytes actually live — a *locator*, never the bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum ArtifactStore {
    InMemory,
    TempFile(String),
    DbBlob(i64),
}

/// Derived / extracted data (carved child, decoded blob, spectrogram image, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub origin_analyzer: String,
    pub label: String,
    pub store: ArtifactStore,
    pub size: u64,
    pub mime: Option<String>,
    /// If this artifact was recursively re-analyzed (matryoshka), its report.
    pub child_report: Option<Box<AnalysisReport>>,
}

/// Per-chunk entropy series driving the right-panel entropy chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntropySeries {
    pub window: u32,
    pub points: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeRef {
    File(String),
    Artifact(ArtifactId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Relation {
    ChildOf,
    DecodesTo,
    SharesString,
    ReferencesHash,
    EmbeddedIn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationEdge {
    pub from: NodeRef,
    pub to: NodeRef,
    pub relation: Relation,
    pub weight: f32,
}

/// Aggregate suspicion, bounded 0..=100 and explainable via `top_reasons`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspicionScore {
    pub value: u8,
    pub clue_count: u32,
    pub top_reasons: Vec<Uuid>,
}

/// The full result of analyzing one file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReport {
    pub fingerprint: Fingerprint,
    pub findings: Vec<Finding>,
    pub artifacts: Vec<Artifact>,
    pub entropy: Option<EntropySeries>,
    pub edges: Vec<CorrelationEdge>,
    pub suspicion: SuspicionScore,
    pub elapsed_ms: u64,
}
