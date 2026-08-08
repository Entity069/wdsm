pub mod jco;
pub mod python;

use anyhow::Result;
use std::path::Path;

/// Componentize a source file into a WASM component.
/// Dispatches to the appropriate backend based on language.
pub fn componentize(language: &str, source: &Path, wit: &Path, output: &Path) -> Result<()> {
    match language {
        "typescript" => jco::componentize(source, wit, output),
        "python" => python::componentize(source, wit, output),
        lang => anyhow::bail!("unsupported language for componentization: {}", lang),
    }
}
