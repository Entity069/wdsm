pub mod runtime;
pub mod handler;
pub mod context;

use anyhow::Result;
use parser::Config;
use registry::FnService;
use std::path::PathBuf;

pub async fn deploy(config: Config, wasm_file: PathBuf) -> Result<FnService> {
    runtime::start_server(config, wasm_file).await
}

pub async fn stop(deployment_id: &str) -> Result<()> {
    runtime::stop_server(deployment_id).await
}