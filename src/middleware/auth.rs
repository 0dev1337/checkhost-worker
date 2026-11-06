use std::sync::Arc;

use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde_json::{Value, json};
use tracing::warn;

use crate::app::AppState;

type Rejection = (StatusCode, Json<Value>);

/// Rejects requests whose `key` query parameter does not match the configured API key.
pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, Rejection> {
    match key_param(request.uri().query().unwrap_or_default()) {
        Some(key) if key == state.config.api.key => Ok(next.run(request).await),
        Some(_) => {
            warn!("rejected request: invalid API key");
            Err(unauthorized("Invalid API key"))
        }
        None => {
            warn!("rejected request: missing API key");
            Err(unauthorized("Missing API key"))
        }
    }
}

fn key_param(query: &str) -> Option<&str> {
    query.split('&').find_map(|pair| pair.strip_prefix("key="))
}

fn unauthorized(message: &str) -> Rejection {
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": message })))
}
