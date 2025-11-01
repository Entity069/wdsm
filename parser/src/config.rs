use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub name: String,
    pub language: String,
    pub entrypoint: String,
    pub entrypoint_function: String,
    pub port: u16,
    pub endpoint: String,
    pub method: String,
    pub payload: String,
}

pub fn parse_cfg(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path).with_context(|| format!("Failed to read config file: {}", path))?;
    let cfg: Config = serde_yaml::from_str(&content).with_context(|| format!("Failed to parse YAML config: {}", path))?;
    
    Ok(cfg)
}