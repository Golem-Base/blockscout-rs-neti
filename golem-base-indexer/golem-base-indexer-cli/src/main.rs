use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use golem_base_indexer_logic::{
    Indexer, repository,
    types::{EntitiesFilter, EntityKey, ListEntitiesFilter, PaginationParams},
};
use sea_orm::DatabaseConnection;

#[derive(Parser)]
struct Cli {
    #[arg(long, env = "DATABASE_URL")]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Tick,
    ListEntityKeys,
    ReindexEntity { entity_key: String },
}

async fn list_entity_keys(db: DatabaseConnection) -> Result<()> {
    let (entities, _) = repository::entities::list_entities(
        &db,
        ListEntitiesFilter {
            pagination: PaginationParams {
                page: 0,
                page_size: i64::MAX as u64,
            },
            entities_filter: EntitiesFilter {
                status: None,
                string_attribute: None,
                numeric_attribute: None,
                owner: None,
            },
        },
    )
    .await?;

    for i in entities {
        println!("{}", i.key);
    }

    Ok(())
}

async fn reindex_entity(db: DatabaseConnection, key: EntityKey) -> Result<()> {
    let db = Arc::new(db);
    let indexer = Indexer::new(db.clone(), Default::default());
    indexer.reindex_entity(key).await
}

async fn tick(db: DatabaseConnection) -> Result<()> {
    Indexer::new(db.into(), Default::default()).tick().await
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let db = sea_orm::Database::connect(cli.db).await?;
    match &cli.command {
        Commands::Tick => tick(db).await?,
        Commands::ListEntityKeys => list_entity_keys(db).await?,
        Commands::ReindexEntity { entity_key } => reindex_entity(db, entity_key.parse()?).await?,
    };

    Ok(())
}
