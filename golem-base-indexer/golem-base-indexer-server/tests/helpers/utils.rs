use anyhow::Result;
use bytes::Bytes;
use golem_base_indexer_logic::mat_view_scheduler::MatViewScheduler;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub fn bytes_to_hex(bytes: &Bytes) -> String {
    let slice = bytes.as_ref();
    let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
    format!("0x{hex}")
}

pub async fn refresh_leaderboards(db: Arc<DatabaseConnection>) -> Result<()> {
    let scheduler = MatViewScheduler::new(db);
    let views = scheduler
        .get_mat_view_settings()
        .into_iter()
        .filter(|v| v.name.contains("leaderboard"));
    for view in views {
        scheduler.refresh_named_view(&view.name).await;
    }
    Ok(())
}

pub async fn refresh_timeseries(db: Arc<DatabaseConnection>) -> Result<()> {
    let scheduler = MatViewScheduler::new(db);
    let views = scheduler
        .get_mat_view_settings()
        .into_iter()
        .filter(|v| v.name.contains("timeseries"));
    for view in views {
        scheduler.refresh_named_view(&view.name).await;
    }
    Ok(())
}

pub fn iso_to_ts_sec(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .unwrap()
        .timestamp()
        .to_string()
}
