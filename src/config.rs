use std::{fs, path::Path};

use anyhow::Context;
use serde::Deserialize;

// `Debug` is deliberately not derived: it would make it easy to log the API key.
#[derive(Deserialize)]
pub struct Config {
    pub server: Server,
    pub api: Api,
    pub location: Location,
}

#[derive(Deserialize)]
pub struct Server {
    pub port: u16,
}

#[derive(Deserialize)]
pub struct Api {
    pub key: String,
}

#[derive(Deserialize)]
pub struct Location {
    pub city: String,
    pub country: String,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse config file {}", path.display()))
    }
}
