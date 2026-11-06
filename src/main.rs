use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use check_host_worker::{
    app::{self, AppState},
    config::Config,
    gateway, telemetry,
};
use tokio::net::TcpListener;
use tracing::info;

const CONFIG_PATH: &str = "config.toml";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();

    let config = Config::load(CONFIG_PATH)?;
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!(
        config = CONFIG_PATH,
        city = %config.location.city,
        country = %config.location.country,
        "configuration loaded"
    );

    let state = Arc::new(AppState::new(config)?);
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;
    info!(address = %bind_addr, "server listening");

    gateway::spawn(state.clone())?;

    let service = app::router(state).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service)
        .await
        .context("server terminated unexpectedly")
}
