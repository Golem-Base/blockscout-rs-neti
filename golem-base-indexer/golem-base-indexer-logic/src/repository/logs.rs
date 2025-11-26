use super::sql::GET_LOGS;
use crate::types::{BlockHash, Log, LogIndex, TxHash};
use alloy_primitives::B256;
use anyhow::{Context, Result};
use golem_base_indexer_entity::logs as EntityLogs;
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use tracing::instrument;

#[derive(FromQueryResult)]
#[allow(dead_code)]
struct DbLog {
    pub data: Vec<u8>,
    pub index: i32,
    pub first_topic: Option<Vec<u8>>,
    pub second_topic: Option<Vec<u8>>,
    pub third_topic: Option<Vec<u8>>,
    pub fourth_topic: Option<Vec<u8>>,
    pub transaction_hash: Vec<u8>,
}

impl TryFrom<LogIndex> for (i32, Vec<u8>, Vec<u8>) {
    type Error = anyhow::Error;

    fn try_from(value: LogIndex) -> Result<Self> {
        Ok((
            value.index.try_into()?,
            value.transaction_hash.as_slice().into(),
            value.block_hash.as_slice().into(),
        ))
    }
}

impl TryFrom<DbLog> for Log {
    type Error = anyhow::Error;

    fn try_from(v: DbLog) -> Result<Self> {
        Ok(Self {
            data: v.data.into(),
            index: v.index.try_into()?,
            first_topic: v.first_topic.map(|v| v.as_slice().try_into()).transpose()?,
            second_topic: v
                .second_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            third_topic: v.third_topic.map(|v| v.as_slice().try_into()).transpose()?,
            fourth_topic: v
                .fourth_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            tx_hash: v.transaction_hash.as_slice().try_into()?,
        })
    }
}

impl TryFrom<EntityLogs::Model> for Log {
    type Error = anyhow::Error;

    fn try_from(value: EntityLogs::Model) -> Result<Self> {
        Ok(Self {
            data: value.data.into(),
            index: value.index.try_into()?,
            first_topic: value
                .first_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            second_topic: value
                .second_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            third_topic: value
                .third_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            fourth_topic: value
                .fourth_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            tx_hash: value.transaction_hash.as_slice().try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn get_tx_logs<T: ConnectionTrait>(
    db: &T,
    tx_hash: TxHash,
    signature: B256,
) -> Result<Vec<Log>> {
    DbLog::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        GET_LOGS,
        [tx_hash.as_slice().into(), signature.as_slice().into()],
    ))
    .all(db)
    .await
    .with_context(|| format!("Failed to get tx logs - tx={tx_hash}, signature={signature}"))?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

#[instrument(skip(db))]
pub async fn get_log<T: ConnectionTrait>(db: &T, log: LogIndex) -> Result<Option<Log>> {
    let id: (i32, Vec<u8>, Vec<u8>) = log.try_into()?;
    EntityLogs::Entity::find_by_id(id)
        .one(db)
        .await?
        .map(TryInto::try_into)
        .transpose()
}

#[instrument(skip(db))]
pub async fn finish_log_processing<T: ConnectionTrait>(
    db: &T,
    tx_hash: TxHash,
    block_hash: BlockHash,
    index: u64,
) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    let block_hash: Vec<u8> = block_hash.as_slice().into();
    let index: i64 = index.try_into()?;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_logs_operations where transaction_hash = $1 and block_hash = $2 and index = $3",
        [tx_hash.into(), block_hash.into(), index.into()],
    ))
    .await
    .context("Failed to finish tx cleanup - logs")?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn finish_log_event_processing<T: ConnectionTrait>(
    db: &T,
    tx_hash: TxHash,
    block_hash: BlockHash,
    index: u64,
) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    let block_hash: Vec<u8> = block_hash.as_slice().into();
    let index: i64 = index.try_into()?;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_logs_events where transaction_hash = $1 and block_hash = $2 and index = $3",
        [tx_hash.into(), block_hash.into(), index.into()],
    ))
    .await
    .context("Failed to finish tx cleanup - event log")?;
    Ok(())
}
