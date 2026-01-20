use std::fs;
use serde::{Deserialize, Serialize};
use dirs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObserverConfig {
    pub name: String,
    pub path: String,
    /// Optional shared secret for HMAC authentication
    /// If not provided, observer will not use authentication (insecure)
    pub shared_secret: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BootstrapPeer {
    pub ip: String,
    pub port: String,
    pub peer_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub listen_addr: String,
    pub port: String,
    pub dht_mode: String,
    pub bootstrap_peers: Vec<BootstrapPeer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub observers: Vec<ObserverConfig>,
    pub network: Option<NetworkConfig>,
}

pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let mut config_path = dirs::home_dir().ok_or("Could not find any config")?;
    config_path.push(".config/syndactyl/config.json");
    let contents = fs::read_to_string(config_path)?;
    let configuration: Config = serde_json::from_str(&contents)?;
    Ok(configuration)
}
