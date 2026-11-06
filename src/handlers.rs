use std::{fmt::Display, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{app::AppState, probes};

#[derive(Serialize)]
pub struct PingResponse {
    ip: String,
    success: bool,
    rtt_ms: u128,
    message: String,
    location: String,
}

#[derive(Serialize)]
pub struct ProbeResponse {
    success: bool,
    address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct HttpParams {
    addr: String,
}

pub async fn ping(
    State(state): State<Arc<AppState>>,
    Path(ip): Path<String>,
) -> Json<PingResponse> {
    let location = &state.config.location;
    let location = format!("{}, {}", location.city, location.country);

    Json(match probes::icmp::ping(ip.clone()).await {
        Ok(reply) => {
            let rtt_ms = reply.rtt.as_millis();
            info!(probe = "icmp", target = %ip, rtt_ms, "probe succeeded");
            PingResponse {
                ip: reply.from,
                success: true,
                rtt_ms,
                message: "OK".to_owned(),
                location,
            }
        }
        Err(err) => {
            warn!(probe = "icmp", target = %ip, error = %err, "probe failed");
            PingResponse {
                ip,
                success: false,
                rtt_ms: 0,
                message: err.to_string(),
                location,
            }
        }
    })
}

pub async fn tcp(Path(addr): Path<String>) -> Json<ProbeResponse> {
    let result = probes::tcp::ping(&addr).await;
    Json(probe_response("tcp", addr, result))
}

pub async fn udp(Path(addr): Path<String>) -> Json<ProbeResponse> {
    let result = probes::udp::ping(&addr).await;
    Json(probe_response("udp", addr, result))
}

pub async fn http(
    State(state): State<Arc<AppState>>,
    Query(HttpParams { addr }): Query<HttpParams>,
) -> Json<ProbeResponse> {
    let result = probes::http::get(&state.http_client, &addr).await;
    Json(probe_response("http", addr, result))
}

fn probe_response<E: Display>(
    probe: &str,
    address: String,
    result: Result<Duration, E>,
) -> ProbeResponse {
    match result {
        Ok(elapsed) => {
            let elapsed_ms = elapsed.as_millis();
            info!(probe, target = %address, elapsed_ms, "probe succeeded");
            ProbeResponse {
                success: true,
                address,
                elapsed_ms: Some(elapsed_ms),
                error: None,
            }
        }
        Err(err) => {
            warn!(probe, target = %address, error = %err, "probe failed");
            ProbeResponse {
                success: false,
                address,
                elapsed_ms: None,
                error: Some(err.to_string()),
            }
        }
    }
}
