use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn componentize(js_file: &Path, wit_file: &Path, output_wasm: &Path) -> Result<()> {
    // check if jco is installed
    let jco_check = Command::new("jco")
        .arg("--version")
        .output();

    if jco_check.is_err() {
        anyhow::bail!(
            "jco is not installed. Install it with: npm install -g @bytecodealliance/jco"
        );
    }

    // jco componentize --wit <wit-file> --out <output-wasm> <js-file>
    // TODO: option to include WASI standard library
    let output = Command::new("jco")
        .arg("componentize")
        .arg("--wit")
        .arg(wit_file)
        .arg("--out")
        .arg(output_wasm)
        .arg(js_file)
        .output()
        .context("Failed to execute jco")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jco compilation failed:\n{}", stderr);
    }

    Ok(())
}