use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::FnService;

fn registry_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".wdsm").join("registry")
}

fn registry_file() -> PathBuf {
    registry_dir().join("deployments.json")
}

fn fregistry_dir() -> Result<()> {
    let dir = registry_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

fn load_registry() -> Result<HashMap<String, FnService>> {
    fregistry_dir()?;
    
    let file = registry_file();
    if !file.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&file)?;
    let deployments: HashMap<String, FnService> = serde_json::from_str(&content)?;

    Ok(deployments)
}

fn save_registry(deployments: &HashMap<String, FnService>) -> Result<()> {
    fregistry_dir()?;

    let content = serde_json::to_string_pretty(deployments)?;
    let target = registry_file();
    let tmp = target.with_extension("json.tmp");

    fs::write(&tmp, content)?;
    fs::rename(&tmp, &target)?;

    Ok(())
}

pub fn save(deployment: FnService) -> Result<()> {
    let mut registry = load_registry()?;
    registry.insert(deployment.id.clone(), deployment);

    save_registry(&registry)
}

pub fn remove(id: &str) -> Result<()> {
    let mut registry = load_registry()?;
    registry.remove(id);

    save_registry(&registry)
}

pub fn find_by_name(name: &str) -> Result<FnService> {
    let registry = load_registry()?;
    registry
        .values()
        .find(|d| d.name == name)
        .cloned()
        .context(format!("FnService '{}' not found", name))
}

pub fn list_all() -> Result<Vec<FnService>> {
    let registry = load_registry()?;
    
    Ok(registry.values().cloned().collect())
}