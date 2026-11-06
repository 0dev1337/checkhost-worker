use std::{
    io,
    time::{Duration, Instant},
};

use tokio::{net::UdpSocket, time::timeout};
use tracing::debug;

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// DNS query for the A record of www.google.com, so that DNS servers answer the probe.
const DNS_QUERY: &[u8] = b"\x24\x1a\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\
    \x03www\x06google\x03com\x00\x00\x01\x00\x01";

/// Measures the round trip of a datagram sent to `address` (`host:port`).
pub async fn ping(address: &str) -> io::Result<Duration> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let mut buffer = [0; 1024];

    let start = Instant::now();
    socket.send_to(DNS_QUERY, address).await?;
    let (size, source) = timeout(REPLY_TIMEOUT, socket.recv_from(&mut buffer)).await??;
    let elapsed = start.elapsed();

    debug!(%source, size, "UDP reply received");
    Ok(elapsed)
}
