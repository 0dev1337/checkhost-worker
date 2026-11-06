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

#[cfg(test)]
mod tests {
    use super::key_param;

    #[test]
    fn finds_key_among_other_parameters() {
        assert_eq!(key_param("addr=x&key=secret"), Some("secret"));
        assert_eq!(key_param("key=a=b"), Some("a=b"));
    }

    #[test]
    fn ignores_other_parameters_and_bare_key() {
        assert_eq!(key_param(""), None);
        assert_eq!(key_param("key"), None);
        assert_eq!(key_param("monkey=secret"), None);
    }
}
