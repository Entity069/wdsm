use anyhow::{Context, Result};

pub async fn execute(name: &str) -> Result<()> {
    println!("[i] Stopping function: {}", name);

    let deployment = registry::find_by_name(name).context(format!("Function '{}' not found", name))?;

    server::stop(&deployment.id).await.context("Failed to stop server")?;

    registry::unregister(&deployment.id).context("Failed to unregister deployment")?;

    println!("[i] Stopped function: {}", name);

    Ok(())
}