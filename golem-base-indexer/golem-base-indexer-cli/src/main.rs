use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use golem_base_indexer_logic::{
    Indexer, repository,
    types::{EntitiesFilter, EntityKey, ListEntitiesFilter, PaginationParams},
};
use sea_orm::{DatabaseConnection, TransactionTrait};

#[derive(Parser)]
struct Cli {
    #[arg(long)]
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
                string_annotation: None,
                numeric_annotation: None,
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
    db.transaction::<_, (), anyhow::Error>(|txn| {
        Box::pin(async move { indexer.reindex_entity(txn, None, key).await })
    })
    .await
    .map_err(Into::into)
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
