use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
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
    pub payload: Vec<HashMap<String, String>>,
    pub return_type: String,
}

pub fn parse_cfg(path: &str) -> Result<Config> {
    let p = std::path::Path::new(path);
    let actual_path = if p.is_dir() {
        p.join("config.yml")
    } else {
        p.to_path_buf()
    };

    let content = fs::read_to_string(&actual_path)
        .with_context(|| format!("Failed to read config file: {}", actual_path.display()))?;
    let cfg: Config = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML config: {}", actual_path.display()))?;

    Ok(cfg)
}