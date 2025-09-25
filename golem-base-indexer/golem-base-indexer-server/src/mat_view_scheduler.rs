use std::sync::Arc;

use golem_base_indexer_logic::mat_view_scheduler::MatViewScheduler;
use sea_orm::DatabaseConnection;

pub async fn run(db_connection: Arc<DatabaseConnection>) -> Result<(), anyhow::Error> {
    let scheduler = MatViewScheduler::new(db_connection);

    tokio::spawn(async move {
        scheduler
            .run()
            .await
            .inspect_err(|e| {
                tracing::error!(?e, "MatViewScheduler exited with error");
            })
            .unwrap();
    });

    Ok(())
}
