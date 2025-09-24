use anyhow::Result;
use bytes::Bytes;
use golem_base_indexer_logic::updater_timeseries::TimeseriesUpdaterService;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub fn bytes_to_hex(bytes: &Bytes) -> String {
    let slice = bytes.as_ref();
    let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
    format!("0x{hex}")
}

pub async fn refresh_timeseries(db: Arc<DatabaseConnection>) -> Result<()> {
    let update_service = TimeseriesUpdaterService::new(db);
    update_service.refresh_views().await
}
