use anyhow::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use std::{sync::Arc, time::Duration};
use tokio::time;
use tracing::error;

#[derive(Clone)]
pub struct UpdateTimeseriesService {
    db: Arc<DatabaseConnection>,
}

impl UpdateTimeseriesService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn refresh_timeseries(&self) -> Result<()> {
        self.db
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                "REFRESH MATERIALIZED VIEW golem_base_timeseries",
            ))
            .await?;

        Ok(())
    }

    pub fn spawn_periodic_task(&self, interval_seconds: u64) {
        let refresher = self.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                if let Err(e) = refresher.refresh_timeseries().await {
                    error!("Timeseries refresh failed: {}", e);
                }
            }
        });
    }
}
