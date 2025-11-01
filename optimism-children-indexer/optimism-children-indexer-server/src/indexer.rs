use std::sync::Arc;

use crate::settings::Settings;
use optimism_children_indexer_logic::Indexer;
use sea_orm::DatabaseConnection;
use tokio::time::sleep;

pub async fn run(
    db_connection: Arc<DatabaseConnection>,
    settings: Settings,
) -> Result<(), anyhow::Error> {
    let db_conn = db_connection.clone();
    let sett = settings.indexer.clone();

    tokio::spawn(async move {
        let indexer = Indexer::new(db_conn, sett);
        indexer.update_gauges().await;
    });

    tokio::spawn(async move {
        let delay = settings.indexer.restart_delay;

        loop {
            let indexer = Indexer::new(db_connection.clone(), settings.indexer.clone());
            match indexer.run().await {
                Err(err) => {
                    tracing::error!(
                        error = ?err,
                        ?delay,
                        "indexer stream ended with error, retrying"
                    );
                }
                Ok(_) => {
                    tracing::error!(?delay, "indexer stream ended unexpectedly, retrying");
                }
            };
            sleep(delay).await;
        }
    });

    Ok(())
}
