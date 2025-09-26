use anyhow::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use std::{sync::Arc, time::Duration};
use tokio::time;
use tracing::error;

const LEADERBOARDS_MATERIALIZED_VIEWS: &[&str] = &[
    "golem_base_leaderboard_biggest_spenders",
    "golem_base_leaderboard_data_owned",
    "golem_base_leaderboard_effectively_largest_entities",
    "golem_base_leaderboard_entities_created",
    "golem_base_leaderboard_entities_owned",
    "golem_base_leaderboard_largest_entities",
    "golem_base_leaderboard_top_accounts",
];

#[derive(Clone)]
pub struct LeaderboardsUpdaterService {
    db: Arc<DatabaseConnection>,
}

impl LeaderboardsUpdaterService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn refresh_views(&self) -> Result<()> {
        for mview in LEADERBOARDS_MATERIALIZED_VIEWS {
            self.db
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!("REFRESH MATERIALIZED VIEW {}", mview),
                ))
                .await?;
        }

        Ok(())
    }

    pub fn spawn_periodic_task(&self, interval_seconds: u64) {
        let refresher = self.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                if let Err(e) = refresher.refresh_views().await {
                    error!("Leaderboards refresh failed: {}", e);
                }
            }
        });
    }
}
