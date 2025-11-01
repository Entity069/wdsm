use anyhow::Result;

pub async fn execute() -> Result<()> {
    let deployments = registry::list_all()?;

    if deployments.is_empty() {
        println!("No functions deployed.");
        return Ok(());
    }

    println!("Deployed Functions:\n");
    println!("{:<20} {:<10} {:<15} {:<30}", "NAME", "METHOD", "PORT", "ENDPOINT");
    println!("{}", "-".repeat(75));

    for d in deployments {
        println!(
            "{:<20} {:<10} {:<15} {:<30}",
            d.name, d.method, d.port, d.endpoint
        );
    }

    println!();
    Ok(())
}