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
    #[serde(default)]
    pub capabilities: CapabilitiesConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilitiesConfig {
    #[serde(default = "default_stdio")]
    pub stdio: bool,
    #[serde(default)]
    pub env: EnvConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

fn default_stdio() -> bool {
    true
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            stdio: true,
            env: EnvConfig::default(),
            network: NetworkConfig::default(),
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnvConfig {
    #[serde(default)]
    pub inherit: bool,
    #[serde(default)]
    pub vars: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NetworkConfig {
    #[serde(default)]
    pub http: bool,
    #[serde(default)]
    pub tcp: bool,
    #[serde(default)]
    pub udp: bool,
    #[serde(default)]
    pub dns: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cfg_default_capabilities() {
        let yaml = r#"
name: test
language: javascript
entrypoint: test.js
entrypoint_function: test
port: 3000
endpoint: /test
method: GET
payload: []
return_type: string
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.capabilities.stdio);
        assert!(!cfg.capabilities.env.inherit);
        assert!(!cfg.capabilities.network.http);
    }

    #[test]
    fn test_parse_cfg_full_capabilities() {
        let yaml = r#"
name: net_test
language: python
entrypoint: net.py
entrypoint_function: net
port: 3015
endpoint: /net
method: POST
payload: []
return_type: string
capabilities:
  stdio: true
  env:
    inherit: true
    vars:
      - API_KEY
  network:
    http: true
    tcp: true
    udp: false
    dns: true
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.capabilities.stdio);
        assert!(cfg.capabilities.env.inherit);
        assert_eq!(cfg.capabilities.env.vars, vec!["API_KEY"]);
        assert!(cfg.capabilities.network.http);
        assert!(cfg.capabilities.network.tcp);
        assert!(!cfg.capabilities.network.udp);
        assert!(cfg.capabilities.network.dns);
    }

    #[test]
    fn test_parse_cfg_custom_capabilities() {
        let yaml = r#"
name: nn_test
language: python
entrypoint: nn.py
entrypoint_function: infer
port: 3020
endpoint: /infer
method: POST
payload: []
return_type: string
capabilities:
  stdio: true
  nn:
    enabled: true
    backend: onnx
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.capabilities.stdio);
        assert!(cfg.capabilities.custom.contains_key("nn"));
        let nn_val = &cfg.capabilities.custom["nn"];
        assert_eq!(nn_val["backend"], "onnx");
    }

    #[test]
    fn test_parse_all_example_configs() {
        let example_paths = vec![
            "../examples/ts/hello/config.yml",
            "../examples/ts/add/config.yml",
            "../examples/ts/user/config.yml",
            "../examples/ts/sudoku/config.yml",
            "../examples/ts/complex/config.yml",
            "../examples/ts/net/config.yml",
            "../examples/python/hello/config.yml",
            "../examples/python/add/config.yml",
            "../examples/python/user/config.yml",
            "../examples/python/sudoku/config.yml",
            "../examples/python/complex/config.yml",
            "../examples/python/net/config.yml",
        ];

        for path in example_paths {
            let res = parse_cfg(path);
            assert!(
                res.is_ok(),
                "Failed to parse example config at {}: {:?}",
                path,
                res.err()
            );
        }
    }
}