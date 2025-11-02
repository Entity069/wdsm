mod config;
mod wit_generator;
mod type_converter;

pub use config::{Config, parse_cfg};
pub use type_converter::{parse_value, format_result};

use anyhow::Result;

pub fn gen_wit(config: &Config) -> Result<String> {
    let wit = wit_generator::generate(config)?;
    
    Ok(wit)
}