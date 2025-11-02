mod config;
mod wit_generator;
mod type_converter;
mod ast_analyzer;

pub use config::{Config, parse_cfg};
pub use type_converter::{parse_value, format_result};
pub use ast_analyzer::{ModuleAnalysis, FnSign, FnParams, WitType};

use anyhow::{Context, Result};
use std::path::Path;

pub fn gen_wit(config: &Config, project_dir: &Path) -> Result<String> {
    let ts_file = project_dir.join(&config.entrypoint);
    
    let analysis = ast_analyzer::anal_fn(&ts_file, &config.entrypoint_function)
        .context("Failed to analyze TypeScript file")?;
    
    let wit = wit_generator::generate(config, &analysis)?;
    
    Ok(wit)
}