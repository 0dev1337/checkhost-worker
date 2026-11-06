use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "info";

/// Installs the global log subscriber. `RUST_LOG` overrides the default filter.
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
