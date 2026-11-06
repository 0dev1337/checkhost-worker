use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    Router,
    middleware::{from_fn, from_fn_with_state},
    routing::get,
};

use crate::{config::Config, handlers, middleware};

const HTTP_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct AppState {
    pub config: Config,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(HTTP_PROBE_TIMEOUT)
            // Reusing connections would leave connection setup out of later measurements.
            .pool_max_idle_per_host(0)
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            config,
            http_client,
        })
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ping/{ip}", get(handlers::ping))
        .route("/tcp/{addr}", get(handlers::tcp))
        .route("/udp/{addr}", get(handlers::udp))
        .route("/http", get(handlers::http))
        .layer(from_fn_with_state(
            state.clone(),
            middleware::auth::require_api_key,
        ))
        // Added last so it runs first and also records requests rejected by auth.
        .layer(from_fn(middleware::logging::log_requests))
        .with_state(state)
}
