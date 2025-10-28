use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[arg(long, env = "DATABASE_URL")]
    db: String,

    #[arg(long)]
    chain_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Tick,
    Reindex { since_block: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let _db = sea_orm::Database::connect(cli.db).await?;
    match &cli.command {
        Commands::Tick => {
            // FIXME TODO
        }
        Commands::Reindex { .. } => {
            // FIXME TODO
        }
    };

    Ok(())
}
