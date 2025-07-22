use blockscout_service_launcher::{database, launcher::ConfigSettings};
use golem_base_indexer_server::{run_indexer, run_server, Settings};
use migration::Migrator;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::build().expect("failed to read config");
    tracing_subscriber::fmt::init();

    let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
    run_indexer(db_connection.into(), settings.clone()).await?;

    let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
    run_server(db_connection.into(), settings).await?;

    Ok(())
}
