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

    let mut js_to_compile = js_file.to_path_buf();
    let mut is_temp = false;
    
    if js_file.extension().and_then(|e| e.to_str()) == Some("ts") {
        let temp_js = std::env::temp_dir().join(format!("{}.js", uuid::Uuid::new_v4()));
        
        let esbuild_check = Command::new("npx")
            .arg("-y")
            .arg("esbuild")
            .arg(js_file)
            .arg("--format=esm")
            .arg(&format!("--outfile={}", temp_js.display()))
            .output()
            .context("Failed to run esbuild to transpile TypeScript")?;

        if !esbuild_check.status.success() {
            let stderr = String::from_utf8_lossy(&esbuild_check.stderr);
            anyhow::bail!("esbuild compilation failed:\n{}", stderr);
        }

        js_to_compile = temp_js;
        is_temp = true;
    }

    // jco componentize --wit <wit-file> --out <output-wasm> <js-file>
    // TODO: option to include WASI standard library
    let output = Command::new("jco")
        .arg("componentize")
        .arg("--wit")
        .arg(wit_file)
        .arg("--out")
        .arg(output_wasm)
        .arg(&js_to_compile)
        .output();

    if is_temp {
        let _ = std::fs::remove_file(&js_to_compile);
    }

    let output = output.context("Failed to execute jco")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jco compilation failed:\n{}", stderr);
    }

    Ok(())
}