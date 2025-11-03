use std::sync::Arc;

use anyhow::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use tokio::time::{sleep, Duration};
use tracing::instrument;

const MINUTE: Duration = Duration::from_secs(60);
const HALF_HOUR: Duration = Duration::from_secs(60 * 30);

pub struct MatViewSettings {
    pub name: String,
    pub delay: Duration,
}

#[derive(Clone)]
pub struct MatViewScheduler {
    db: Arc<DatabaseConnection>,
}

impl MatViewScheduler {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn get_mat_view_settings(&self) -> Vec<MatViewSettings> {
        vec![
            // charts
            MatViewSettings {
                name: "golem_base_entity_data_size_histogram".to_string(),
                delay: MINUTE,
            },
            // Leaderboards
            MatViewSettings {
                name: "golem_base_leaderboard_biggest_spenders".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_leaderboard_data_owned".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_leaderboard_effectively_largest_entities".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_leaderboard_entities_created".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_leaderboard_entities_owned".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_leaderboard_largest_entities".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_leaderboard_top_accounts".to_string(),
                delay: HALF_HOUR,
            },
            // timeseries
            MatViewSettings {
                name: "golem_base_timeseries_data_usage".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_timeseries_storage_forecast".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_timeseries_operation_count".to_string(),
                delay: HALF_HOUR,
            },
            MatViewSettings {
                name: "golem_base_timeseries_entity_count".to_string(),
                delay: HALF_HOUR,
            },
        ]
    }

    #[instrument(skip_all)]
    pub async fn run(&self) -> Result<()> {
        let views = self.get_mat_view_settings();

        for view in views {
            let scheduler = self.clone();

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

        let sql = format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {view}");
        let _ = self
            .db
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .inspect_err(|e| tracing::error!(?e, "Failed to refresh materialized view"));
    }
}
