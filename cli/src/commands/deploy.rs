use anyhow::{anyhow, Context, Result};
use std::path::Path;

pub async fn execute(config_path: &str) -> Result<()> {
    let raw_path = Path::new(config_path);
    let resolved_path = if raw_path.is_dir() {
        raw_path.join("config.yml")
    } else {
        raw_path.to_path_buf()
    };
    let resolved_str = resolved_path.to_string_lossy();

    let config = parser::parse_cfg(&resolved_str).context("Failed to parse config")?;

    let project_dir = resolved_path.parent().context("Invalid config path")?;

    let wdsm_dir = project_dir.join(".wdsm");
    std::fs::create_dir_all(&wdsm_dir).context("Failed to create .wdsm directory")?;

    let source_file = project_dir.join(&config.entrypoint);
    let wit_content =
        parser::gen_wit(&config, project_dir).context("Failed to generate WIT file")?;

    let wit_file = wdsm_dir.join("interface.wit");
    std::fs::write(&wit_file, &wit_content).context("Failed to write WIT file")?;

    let wasm_file = wdsm_dir.join("function.wasm");
    componentizer::componentize(&config.language, &source_file, &wit_file, &wasm_file, &config.name)
        .context("Failed to compile to WASM")?;

    let deployment = server::deploy(config.clone(), wasm_file)
        .await
        .context("Failed to start server")?;

    // before registering, wait if the server is actually running
    // need to make deployment atomic in order to avoid undeployed entries in registry
    if let Err(e) = health_check(config.port, 3_000)
        .await
        .with_context(|| {
            format!(
                "[i] Server on port {} did not become ready in time",
                config.port
            )
        })
    {
        let _ = server::stop(&deployment.id).await;
        return Err(e);
    }

    registry::register(deployment.clone()).context("[!] Failed to register deployment")?;

    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    server::stop(&deployment.id)
        .await
        .context("Failed to stop server")?;

    registry::unregister(&deployment.id).context("Failed to unregister deployment")?;

    Ok(())
}

async fn health_check(port: u16, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .context("Failed to build HTTP client")?;

    let url = format!("http://127.0.0.1:{}/__health", port);
    let start = Instant::now();
    let mut last_err: Option<anyhow::Error> = None;

    while start.elapsed().as_millis() < timeout_ms as u128 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            Ok(resp) => {
                last_err = Some(anyhow!("health check {}", resp.status()));
            }
            Err(e) => {
                last_err = Some(anyhow!(e));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(last_err.unwrap_or_else(|| anyhow!("time out")))
}