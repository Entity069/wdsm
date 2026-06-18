pub mod typescript;

use crate::ir::WitIR;
use std::path::Path;

pub trait LanguageFrontend: Send + Sync {
    fn extract(&self, source: &Path, config: &FrontendConfig) -> anyhow::Result<WitIR>;
    fn language(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct FrontendConfig {
    pub package_name: String,
    pub world_name: String,
    pub target_functions: Option<Vec<String>>,
}

pub fn get_frontend(language: &str) -> Option<Box<dyn LanguageFrontend>> {
    match language {
        "typescript" => Some(Box::new(typescript::TypeScriptFrontend)),
        _ => None,
    }
}
