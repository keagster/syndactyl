use std::fs;
use serde::{Deserialize, Serialize};
use dirs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObserverConfig {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub port: String,
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
