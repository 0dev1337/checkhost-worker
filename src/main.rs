mod ping;
mod config;
mod middleware;

use axum::{
    extract::Path,
    middleware as axum_middleware,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};

async fn ping_handler(Path(ip): Path<String>) -> Json<Value> {
    let cfg = config::get();

    let result = tokio::task::spawn_blocking(move || {
        ping::ping(&ip)
    }).await.unwrap();

    Json(json!({
        "ip":       result.ip,
        "success":  result.success,
        "rtt_ms":   result.rtt_ms,
        "message":  result.message,
        "location": format!("{}, {}", cfg.location.city, cfg.location.country),
    }))
}

#[tokio::main]
async fn main() {
    config::init("config.toml");

    let cfg = config::get();

    let app = Router::new()
        .route("/ping/{ip}", get(ping_handler))
        .layer(axum_middleware::from_fn(middleware::auth::auth_middleware));

    let addr = format!("0.0.0.0:{}", cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on http://localhost:{}", cfg.server.port);
    axum::serve(listener, app).await.unwrap();
}
