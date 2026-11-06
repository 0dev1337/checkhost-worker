use std::{net::SocketAddr, time::Instant};

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument, error, info, info_span, warn};

/// Logs the outcome of every request and opens a span shared by logs emitted while handling it.
///
/// Only the path is recorded: the query string carries the API key and must never be logged.
pub async fn log_requests(request: Request, next: Next) -> Response {
    let client = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.to_string());
    let span = info_span!(
        "request",
        method = %request.method(),
        path = %request.uri().path(),
        client = client.as_deref(),
    );

    async move {
        let start = Instant::now();
        let response = next.run(request).await;
        let latency_ms = start.elapsed().as_millis();
        let status = response.status();

        if status.is_server_error() {
            error!(status = status.as_u16(), latency_ms, "request failed");
        } else if status.is_client_error() {
            warn!(status = status.as_u16(), latency_ms, "request rejected");
        } else {
            info!(status = status.as_u16(), latency_ms, "request completed");
        }

        response
    }
    .instrument(span)
    .await
}
