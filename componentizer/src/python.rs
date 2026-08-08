use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Componentize a Python source file into a WASM component using componentize-py.
/// Requires `componentize-py` to be installed: `pip install componentize-py`
///
/// The `world` argument is the WIT world name (e.g. "hello-py"). The Python module
/// name (file stem) must differ from the world name — componentize-py generates a
/// module with the world name, so app module and generated bindings can't share it.
pub fn componentize(
    py_file: &Path,
    wit_file: &Path,
    output_wasm: &Path,
    world: &str,
) -> Result<()> {
    // Check if componentize-py is installed
    Command::new("componentize-py")
        .arg("--version")
        .output()
        .map_err(|_| {
            anyhow::anyhow!(
                "componentize-py is not installed. Install it with: pip install componentize-py"
            )
        })?;

    // Module name = file stem (e.g. "hello" from hello.py)
    let module_name = py_file
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid Python file name")?;

    // WIT directory containing interface.wit — must be absolute since we change CWD
    let wit_dir = wit_file
        .parent()
        .context("WIT file has no parent directory")?;
    let wit_dir_abs = wit_dir
        .canonicalize()
        .with_context(|| format!("WIT directory not found: {}", wit_dir.display()))?;

    // Absolute output path — also needs to be absolute after CWD change
    let output_abs = output_wasm
        .canonicalize()
        .unwrap_or_else(|_| {
            // File doesn't exist yet; make it absolute relative to current working dir
            std::env::current_dir()
                .unwrap_or_default()
                .join(output_wasm)
        });

    // Python source directory (componentize-py searches here for the module)
    let py_dir = py_file
        .parent()
        .context("Python file has no parent directory")?;

    // componentize-py -d <wit_dir_abs> -w <world> componentize <module> -o <output_abs>
    let output = Command::new("componentize-py")
        .arg("-d")
        .arg(&wit_dir_abs)
        .arg("-w")
        .arg(world)
        .arg("componentize")
        .arg(module_name)
        .arg("-o")
        .arg(&output_abs)
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
