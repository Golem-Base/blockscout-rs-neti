use blockscout_service_launcher::{database, launcher::ConfigSettings, tracing};
use golem_base_indexer_server::{run_indexer, run_server, Settings};
use migration::Migrator;

const SERVICE_NAME: &str = "golem_base_indexer";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::build().expect("failed to read config");
    tracing::init_logs(SERVICE_NAME, &settings.tracing, &settings.jaeger)?;

    let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
    run_indexer(db_connection.into(), settings.clone()).await?;

    let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
    run_server(db_connection.into(), settings).await?;

    Ok(())
}
