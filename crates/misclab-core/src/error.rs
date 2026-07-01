//! Core error type. Individual node failures are generally non-fatal at the graph
//! level (the executor records them and continues); this type is for the errors a
//! node or the engine itself can raise.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("type error: {0}")]
    Type(String),

    #[error("graph error: {0}")]
    Graph(String),

    #[error("missing input: {0}")]
    MissingInput(String),

    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
