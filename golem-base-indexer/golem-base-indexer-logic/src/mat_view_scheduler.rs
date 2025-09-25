use std::sync::Arc;

use anyhow::Result;
use key_mutex::tokio::KeyMutex;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use tokio::time::{self, sleep};
use tracing::instrument;

pub struct MatViewSettings {
    pub name: String,
    pub delay: time::Duration,
}

pub struct MatViewScheduler {
    db: Arc<DatabaseConnection>,
    lock: KeyMutex<String, ()>,
}

impl MatViewScheduler {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let lock = KeyMutex::new();
        Self { db, lock }
    }

    fn get_mat_view_settings(&self) -> Vec<MatViewSettings> {
        vec![MatViewSettings {
            name: "golem_base_entity_data_size_histogram".to_string(),
            delay: time::Duration::from_secs(60),
        }]
    }

    #[instrument(skip_all)]
    pub async fn run(&self) -> Result<()> {
        let views = self.get_mat_view_settings();

        for view in views {
            let scheduler = Self {
                db: self.db.clone(),
                lock: self.lock.clone(),
            };

            tokio::spawn(async move {
                loop {
                    scheduler.refresh_named_view(&view.name).await;
                    sleep(view.delay).await;
                }
            });
        }

        Ok(())
    }

    pub async fn refresh_named_view(&self, view: &str) {
        tracing::info!("Running refresh named view {view}");

        let _guard = self.lock.lock(view.to_string()).await;
        let sql = format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {view}");
        let _ = self
            .db
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .inspect_err(|e| tracing::error!(?e, "Failed to refresh materialized view"));
    }
}
