use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Json, Response},
};
use serde_json::{json, Value};

pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let cfg = crate::config::get();

    // Extract ?key=... from query string
    let query = request.uri().query().unwrap_or("");
    let api_key = query
        .split('&')
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("key"), Some(val)) => Some(val),
                _ => None,
            }
        });

    match api_key {
        Some(k) if k == cfg.api.key => Ok(next.run(request).await),
        Some(_) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid API key" })),
        )),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing API key" })),
        )),
    }
}
