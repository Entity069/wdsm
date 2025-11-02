mod store;

use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnService {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub port: u16,
    pub method: String,
    pub wasm_path: String,
}

pub fn register(deployment: FnService) -> Result<()> {
    store::save(deployment)
}

pub fn unregister(id: &str) -> Result<()> {
    store::remove(id)
}

pub fn find_by_name(name: &str) -> Result<FnService> {
    store::find_by_name(name)
}

pub fn list_all() -> Result<Vec<FnService>> {
    store::list_all()
}