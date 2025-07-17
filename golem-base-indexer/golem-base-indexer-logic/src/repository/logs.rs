use super::sql::GET_LOGS;
use anyhow::Result;
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use tracing::instrument;

#[derive(FromQueryResult)]
#[allow(dead_code)]
pub struct Log {
    pub data: Vec<u8>,
    pub index: i32,
    pub first_topic: Vec<u8>,
    pub second_topic: Vec<u8>,
    pub third_topic: Vec<u8>,
    pub fourth_topic: Vec<u8>,
}

#[instrument(name = "repository::logs::get_tx_logs", skip(db), level = "info")]
pub async fn get_tx_logs<T: ConnectionTrait>(
    db: &T,
    tx_hash: Vec<u8>,
    first_topic: Vec<u8>,
) -> Result<Vec<Log>> {
    Ok(Log::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        GET_LOGS,
        [tx_hash.as_slice().into(), first_topic.as_slice().into()],
    ))
    .all(db)
    .await?)
}
