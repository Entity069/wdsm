pub mod jco;
pub mod python;

use anyhow::Result;
use std::path::Path;

/// Componentize a source file into a WASM component.
/// Dispatches to the appropriate backend based on language.
///
/// - `language`: "typescript" | "python"
/// - `source`:   path to the source file (.ts / .py)
/// - `wit`:      path to the generated .wit file
/// - `output`:   path for the output .wasm file
/// - `world`:    WIT world name (e.g. "hello-py")
pub fn componentize(
    language: &str,
    source: &Path,
    wit: &Path,
    output: &Path,
    world: &str,
) -> Result<()> {
    match language {
        "typescript" => jco::componentize(source, wit, output),
        "python" => python::componentize(source, wit, output, world),
        lang => anyhow::bail!("unsupported language for componentization: {}", lang),
    }
}
