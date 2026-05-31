pub mod ir;
pub mod errors;
pub mod frontends;
pub mod validation;
pub mod codegen;
mod config;
mod type_converter;

pub use ir::{WitIR, IRType, TypeDef, RecordDef, FieldDef, FunctionDef, ParamDef, ReturnType};
pub use ir::{VariantDef, VariantCase, EnumDef, FlagsDef, AliasDef, SourceSpan, Confidence};
pub use errors::WdsmError;
pub use frontends::{LanguageFrontend, FrontendConfig, get_frontend};
pub use codegen::generate_wit;
pub use validation::validate;

pub use config::{Config, parse_cfg};
pub use type_converter::{parse_value, format_result};

use anyhow::{Context, Result};
use std::path::Path;

pub fn gen_wit(config: &Config, project_dir: &Path) -> Result<String> {
    let ts_file = project_dir.join(&config.entrypoint);

    let (wit, warnings) = gen_wit_ir(
        &ts_file,
        &config.language,
        &format!("wdsm:{}", config.name),
        &config.name,
        Some(vec![config.entrypoint_function.clone()]),
    )?;

    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    Ok(wit)
}

pub fn gen_wit_ir(
    source: &Path,
    language: &str,
    package_name: &str,
    world_name: &str,
    target_functions: Option<Vec<String>>,
) -> Result<(String, Vec<String>)> {
    let frontend = get_frontend(language)
        .ok_or_else(|| anyhow::anyhow!("unsupported language: {}", language))?;

    let config = FrontendConfig {
        package_name: package_name.to_string(),
        world_name: world_name.to_string(),
        target_functions,
    };

    let mut ir = frontend
        .extract(source, &config)
        .context("Failed to extract IR from source file")?;

    let warnings = validate(&mut ir)
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    let wit = generate_wit(&ir);

    Ok((wit, warnings))
}