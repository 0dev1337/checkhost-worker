use std::time::{Duration, Instant};

use reqwest::Client;
use tracing::debug;

/// Measures the time until the response headers of a GET request arrive.
///
/// Any HTTP status counts as a response; only transport-level failures are errors.
pub async fn get(client: &Client, url: &str) -> reqwest::Result<Duration> {
    let start = Instant::now();
    let response = client.get(url).send().await?;
    let elapsed = start.elapsed();

    debug!(
        status = response.status().as_u16(),
        "HTTP response received"
    );
    Ok(elapsed)
}
