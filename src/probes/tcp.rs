use std::{
    io,
    time::{Duration, Instant},
};

use tokio::{net::TcpStream, time::timeout};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Measures how long a TCP connection to `address` (`host:port`) takes to establish.
pub async fn ping(address: &str) -> io::Result<Duration> {
    let start = Instant::now();
    timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await??;
    Ok(start.elapsed())
}
