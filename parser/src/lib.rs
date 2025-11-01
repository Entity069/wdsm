mod config;
mod ast_analyzer;
mod wit_generator;

pub use config::{Config, parse_cfg};
pub use ast_analyzer::FnSign;

use anyhow::Result;
use std::path::Path;

pub fn gen_wit(js_file: &Path, config: &Config) -> Result<String> {
    let signature = ast_analyzer::anal_fn(js_file, &config.entrypoint_function)?;
    
    let wit = wit_generator::generate(config, &signature)?;
    
    Ok(wit)
}