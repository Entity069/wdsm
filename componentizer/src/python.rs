use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Componentize a Python source file into a WASM component using componentize-py.
/// Requires `componentize-py` to be installed: `pip install componentize-py`
///
/// The world name is derived from the Python file stem by convention — e.g.,
/// `hello.py` uses world `hello`. This matches how the WIT generator names worlds.
pub fn componentize(py_file: &Path, wit_file: &Path, output_wasm: &Path) -> Result<()> {
    // Check if componentize-py is installed
    Command::new("componentize-py")
        .arg("--version")
        .output()
        .map_err(|_| {
            anyhow::anyhow!(
                "componentize-py is not installed. Install it with: pip install componentize-py"
            )
        })?;

    // Derive module name from file stem (e.g. hello.py -> "hello")
    let module_name = py_file
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid Python file name")?;

    // WIT world name is the module name (kebab-cased by convention from our generator)
    // componentize-py -d <wit_dir> -w <world> componentize <module> -o <output>
    let wit_dir = wit_file
        .parent()
        .context("WIT file has no parent directory")?;

    let py_dir = py_file
        .parent()
        .context("Python file has no parent directory")?;

    let output = Command::new("componentize-py")
        .arg("-d")
        .arg(wit_dir)
        .arg("-w")
        .arg(module_name)
        .arg("componentize")
        .arg(module_name)
        .arg("-o")
        .arg(output_wasm)
        .current_dir(py_dir)
        .output()
        .context("Failed to execute componentize-py")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "componentize-py failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        );
    }

    Ok(())
}
