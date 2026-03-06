use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]  // ✅ Add Debug
pub struct Config {
    pub server: Server,
    pub api: Api,
    pub location: Location,
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub port: u16,
}

#[derive(Deserialize, Debug)]  
pub struct Api {
    pub key: String,
}

#[derive(Deserialize, Debug)]
pub struct Location {
    pub city: String,
    pub country: String,
}

pub fn load(path: &str) -> Config {
    let content = fs::read_to_string(path).expect("Failed to read config file");
    toml::from_str(&content).expect("Failed to parse config file")
}
