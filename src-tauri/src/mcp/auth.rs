//! Bearer-token gate for the local MCP endpoint. The server is a code-execution
//! surface (script nodes spawn processes), so every `/mcp` request must present
//! `Authorization: Bearer <token>`.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Constant-time byte comparison (avoids leaking the token via timing).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// axum middleware: reject requests whose bearer token doesn't match. When no
/// token is configured the gate is open (localhost-only convenience) — the
/// lifecycle layer always generates one on enable, so this is a rare dev path.
pub async fn require_bearer(
    State(token): State<Arc<Option<String>>>,
    req: Request,
    next: Next,
) -> Response {
    let expected = match token.as_ref() {
        Some(t) if !t.is_empty() => t,
        _ => return next.run(req).await,
    };
    let provided = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    if ct_eq(provided.as_bytes(), expected.as_bytes()) {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}
