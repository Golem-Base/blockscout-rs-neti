use std::sync::Arc;

use crate::settings::Settings;
use optimism_children_indexer_l3::Layer3Indexer;
use optimism_children_indexer_logic::Indexer;
use sea_orm::DatabaseConnection;
use tokio::time::{sleep, Duration};

pub async fn run(
    db_connection: Arc<DatabaseConnection>,
    settings: Settings,
) -> Result<(), anyhow::Error> {
    let db_conn = db_connection.clone();
    let db_conn2 = Arc::clone(&db_connection);
    let sett = settings.indexer.clone();

    tokio::spawn(async move {
        let indexer = Indexer::new(db_conn, sett);
        indexer.update_gauges().await;
    });

    tokio::spawn(async move {
        let delay = settings.indexer.restart_delay;

        loop {
            let indexer = Indexer::new(db_conn2.clone(), settings.indexer.clone());
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

    // Spawn Layer3 Indexer
    tokio::spawn(async move {
        // TODO: Consider making this configurable via Settings
        let delay = Duration::from_secs(60);

        loop {
            let mut layer3_indexer = Layer3Indexer::new(Arc::clone(&db_connection));
            match layer3_indexer.run().await {
                Err(err) => {
                    tracing::error!(error = ?err, "Layer3 Indexer ended with error, retrying");
                }
                Ok(_) => {
                    tracing::error!("Layer3 Indexer ended unexpectedly, retrying");
                }
            }
            sleep(delay).await;
        }
    });

    Ok(())
}
