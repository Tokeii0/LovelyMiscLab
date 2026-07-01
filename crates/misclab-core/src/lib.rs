//! LovelyMiscLab analysis engine — a **typed node-graph engine** for CTF misc work.
//!
//! Tauri-agnostic on purpose: it can be unit-tested headlessly
//! (`cargo test -p misclab-core`) and reused from a CLI. The Tauri app
//! (`src-tauri`) is a thin adapter.
//!
//! - [`graph`]    — typed ports ([`graph::port`]), the serialized graph model,
//!   the petgraph-backed compute graph, and the [`graph::executor`].
//! - [`node`]     — the [`node::Node`] trait, the serializable
//!   [`node::descriptor::NodeDescriptor`] (data-driven UI), and the registry.
//! - [`nodes`]    — concrete built-in nodes; adding a node happens here.
//! - [`model`]    — shared value types (e.g. [`model::Fingerprint`]) used as node outputs.
//! - [`progress`] / [`cancel`] / [`input`] — per-node progress, cancellation, file access.
//!
//! **Adding a node = implement [`node::Node`] + write a [`node::descriptor::NodeDescriptor`]
//! + register it.** It then appears in the frontend palette automatically.

pub mod ai;
pub mod cancel;
pub mod error;
pub mod graph;
pub mod input;
pub mod model;
pub mod node;
pub mod nodes;
pub mod progress;

pub use error::{CoreError, CoreResult};

/// The engine version (mirrors the crate version).
pub fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
