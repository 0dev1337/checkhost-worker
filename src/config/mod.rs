
mod parser;
pub use parser::{Config, load};

use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init(path: &str) {
    CONFIG.set(load(path)).expect("Config already initialized");
}

pub fn get() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}
