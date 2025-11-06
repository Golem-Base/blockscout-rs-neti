use anyhow::Result;
use clap::{Parser, Subcommand};
use optimism_children_indexer_logic::Indexer;

#[derive(Parser)]
struct Cli {
    #[arg(long, env = "DATABASE_URL")]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    L2Tick,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();
    let cli = Cli::parse();

    let db = sea_orm::Database::connect(cli.db).await?;
    match &cli.command {
        Commands::L2Tick => {
            Indexer::new(db.into(), Default::default())
                .tick()
                .await
                .unwrap();
        }
    };

    Ok(())
}
