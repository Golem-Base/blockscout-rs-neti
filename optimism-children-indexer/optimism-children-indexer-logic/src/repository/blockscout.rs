use crate::repository::sql::GET_TX_BY_HASH;
use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use optimism_children_indexer_entity::optimism_children_pending_logs;
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement, StreamTrait};
use tracing::instrument;

use super::sql;
use crate::types::{LogIndex, Tx, TxHash};

#[derive(FromQueryResult)]
struct DbTx {
    pub hash: Vec<u8>,
    pub from_address_hash: Vec<u8>,
    pub to_address_hash: Vec<u8>,
    pub block_number: Option<i32>,
    pub block_hash: Option<Vec<u8>>,
    pub block_timestamp: Option<chrono::NaiveDateTime>,
    pub input: Vec<u8>,
    pub index: Option<i32>,
}

impl TryFrom<DbTx> for Tx {
    type Error = anyhow::Error;
    fn try_from(tx: DbTx) -> Result<Self> {
        Ok(Self {
            input: tx.input.into(),
            block_hash: tx.block_hash.map(|v| v.as_slice().try_into()).transpose()?,
            block_number: tx.block_number.map(|v| v.try_into()).transpose()?,
            block_timestamp: tx.block_timestamp.map(|v| v.and_utc()),
            from_address_hash: tx.from_address_hash.as_slice().try_into()?,
            to_address_hash: tx.to_address_hash.as_slice().try_into()?,
            hash: tx.hash.as_slice().try_into()?,
            index: tx.index.map(|v| v.try_into()).transpose()?,
        })
    }
}

#[derive(FromQueryResult)]
struct DbLogIndex {
    pub transaction_hash: Vec<u8>,
    pub block_hash: Vec<u8>,
    pub index: i32,
}

impl TryFrom<DbLogIndex> for LogIndex {
    type Error = anyhow::Error;

    fn try_from(value: DbLogIndex) -> Result<Self> {
        Ok(Self {
            transaction_hash: value.transaction_hash.as_slice().try_into()?,
            block_hash: value.block_hash.as_slice().try_into()?,
            index: value.index.try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn stream_unprocessed_logs<T: StreamTrait + ConnectionTrait>(
    db: &T,
) -> Result<impl Stream<Item = LogIndex> + '_> {
    Ok(
        DbLogIndex::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql::GET_UNPROCESSED_LOGS,
            [],
        ))
        .stream(db)
        .await
        .context("Failed to get unprocessed logs")?
        .filter_map(|log| async {
            match log {
                Ok(log) => Some(log.try_into().ok()?),
                Err(err) => {
                    tracing::error!(error = ?err, "error during unprocessed log retrieval");
                    None
                }
            }
        }),
    )
}

#[instrument(skip(db))]
pub async fn get_tx<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<Option<Tx>> {
    DbTx::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        GET_TX_BY_HASH,
        [tx_hash.as_slice().into()],
    ))
    .one(db)
    .await
    .context("Failed to get tx by hash")?
    .map(TryInto::try_into)
    .transpose()
}

#[instrument(skip(db))]
pub async fn count_unprocessed_logs<T: StreamTrait + ConnectionTrait>(db: &T) -> Result<u64> {
    optimism_children_pending_logs::Entity::find()
        .count(db)
        .await
        .context("Failed to count pending logs")
}
