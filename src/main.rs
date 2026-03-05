mod ping;

use axum::{
    extract::Path,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};

// GET /ping/8.8.8.8
async fn ping_handler(Path(ip): Path<String>) -> Json<Value> {
    let result = tokio::task::spawn_blocking(move || {
        ping::ping(&ip)
    }).await.unwrap();

    Json(json!({
        "ip":      result.ip,
        "success": result.success,
        "rtt_ms":  result.rtt_ms,
        "message": result.message,
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/ping/{ip}", get(ping_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
