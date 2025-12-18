use blockscout_service_launcher::{database, launcher::ConfigSettings};
use golem_base_indexer_server::{run_indexer, run_mat_view_scheduler, run_server, Settings};
use migration::Migrator;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::build().expect("failed to read config");
    tracing_subscriber::fmt::init();

    if !settings.indexer.api_only {
        let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
        run_indexer(db_connection.into(), settings.clone()).await?;

        let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
        run_mat_view_scheduler(db_connection.into()).await?;
    }

    let db_connection = database::initialize_postgres::<Migrator>(&settings.database).await?;
    run_server(db_connection.into(), settings.clone()).await?;

    Ok(())
}
