//Periodically reports this worker's address and location to the gateway.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, ensure};
use reqwest::Url;
use serde::Serialize;
use tokio::{
    net::{UdpSocket, lookup_host},
    time::{MissedTickBehavior, interval},
};
use tracing::{debug, info, warn};

use crate::app::AppState;

#[derive(Serialize)]
struct Heartbeat<'a> {
    ip: IpAddr,
    city: &'a str,
    country: &'a str,
}

/// Validates the gateway settings and starts sending heartbeats in the background.
/// Failed heartbeats are logged and retried on the next tick.
pub fn spawn(state: Arc<AppState>) -> anyhow::Result<()> {
    let gateway = &state.config.gateway;
    ensure!(gateway.cooldown > 0, "gateway.cooldown must be at least 1");

    let url = Url::parse(&gateway.host)
        .with_context(|| format!("invalid gateway.host {:?}", gateway.host))?;
    let port = url
        .port_or_known_default()
        .context("gateway.host has no known port")?;
    let host = url
        .host_str()
        .context("gateway.host has no host")?
        .trim_matches(['[', ']'])
        .to_owned();
    let period = Duration::from_secs(gateway.cooldown);

    info!(url = %url, cooldown_secs = gateway.cooldown, "starting gateway heartbeat");
    tokio::spawn(async move {
        let mut ticker = interval(period);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            match send_heartbeat(&state, url.clone(), &host, port).await {
                Ok(ip) => debug!(%ip, "gateway heartbeat sent"),
                Err(err) => warn!(error = format!("{err:#}"), "gateway heartbeat failed"),
            }
        }
    });

    Ok(())
}

async fn send_heartbeat(
    state: &AppState,
    url: Url,
    host: &str,
    port: u16,
) -> anyhow::Result<IpAddr> {
    let ip = outbound_ip(host, port)
        .await
        .context("failed to determine worker IP")?;
    let location = &state.config.location;

    state
        .http_client
        .post(url)
        .json(&Heartbeat {
            ip,
            city: &location.city,
            country: &location.country,
        })
        .send()
        .await?
        .error_for_status()?;

    Ok(ip)
}

/// Returns the local IP address used to reach `host`.
/// Connecting a UDP socket only selects a route, so no packets are sent.
async fn outbound_ip(host: &str, port: u16) -> io::Result<IpAddr> {
    let target = lookup_host((host, port))
        .await?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "host has no addresses"))?;
    let bind_addr = match target {
        SocketAddr::V4(_) => "0.0.0.0:0",
        SocketAddr::V6(_) => "[::]:0",
    };

    let socket = UdpSocket::bind(bind_addr).await?;
    socket.connect(target).await?;
    Ok(socket.local_addr()?.ip())
}
