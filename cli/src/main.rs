mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "wdsm")]
#[command(about = "wdsm", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Deploy {
        #[arg(long)]
        config: String,
    },
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init();
    
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy { config } => {
            commands::deploy::execute(&config,).await?;
        }
        Commands::List => {
            commands::list::execute().await?;
        }
    }

    Ok(())
}