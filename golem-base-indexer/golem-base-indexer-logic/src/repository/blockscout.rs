use crate::{repository::sql::GET_TX_BY_HASH, types::Block};
use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use golem_base_indexer_entity::{
    golem_base_pending_transaction_cleanups, golem_base_pending_transaction_operations,
};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, QuerySelect, Statement, StreamTrait};
use tracing::instrument;

use super::sql;
use crate::{
    types::{BlockHash, BlockNumber, LogIndex, Tx, TxHash},
    well_known,
};

#[derive(FromQueryResult)]
struct DbTxHash {
    pub hash: Vec<u8>,
}

#[derive(FromQueryResult)]
struct DbBlock {
    pub hash: Vec<u8>,
    pub number: i64,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(FromQueryResult)]
struct DbBlockNumber {
    pub number: i64,
}

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

impl TryFrom<DbBlockNumber> for BlockNumber {
    type Error = anyhow::Error;

    fn try_from(value: DbBlockNumber) -> Result<Self> {
        value
            .number
            .try_into()
            .context("Failed to convert block number")
    }
}

impl TryFrom<DbBlock> for Block {
    type Error = anyhow::Error;

    fn try_from(value: DbBlock) -> Result<Self> {
        Ok(Self {
            hash: value.hash.as_slice().try_into()?,
            number: value.number.try_into()?,
            timestamp: value.timestamp.and_utc(),
        })
    }
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
pub async fn stream_unprocessed_tx_hashes<T: StreamTrait + ConnectionTrait>(
    db: &T,
) -> Result<impl Stream<Item = TxHash> + '_> {
    Ok(DbTxHash::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::GET_UNPROCESSED_TX_HASHES,
        [
            well_known::L1_BLOCK_CONTRACT_ADDRESS.as_slice().into(),
            well_known::GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS
                .as_slice()
                .into(),
        ],
    ))
    .stream(db)
    .await
    .context("Failed to get unprocessed tx hashes")?
    .filter_map(|tx| async {
        match tx {
            Ok(tx) => Some(TxHash::from_slice(&tx.hash)),
            Err(err) => {
                tracing::error!(error = ?err, "error during unprocessed tx hash retrieval");
                None
            }
        }
    }))
}

#[instrument(skip(db))]
pub async fn stream_tx_hashes_for_cleanup<T: StreamTrait + ConnectionTrait>(
    db: &T,
) -> Result<impl Stream<Item = TxHash> + '_> {
    Ok(golem_base_pending_transaction_cleanups::Entity::find()
        .limit(100)
        .stream(db)
        .await
        .context("Failed to get tx hashes for cleanup")?
        .filter_map(|tx| async {
            match tx {
                Ok(tx) => Some(TxHash::from_slice(&tx.hash)),
                Err(err) => {
                    tracing::error!(error = ?err, "error during tx hashes for cleanup retrieval");
                    None
                }
            }
        }))
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
pub async fn get_current_block<T: ConnectionTrait>(db: &T) -> Result<Option<Block>> {
    DbBlock::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        "select hash, number, timestamp from blocks order by number desc limit 1",
    ))
    .one(db)
    .await
    .context("Failed to get current block")?
    .map(TryInto::try_into)
    .transpose()
}

#[instrument(skip(db))]
pub(super) async fn get_block<T: ConnectionTrait>(
    db: &T,
    hash: BlockHash,
) -> Result<Option<Block>> {
    let hash: Vec<u8> = hash.as_slice().into();
    DbBlock::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "select hash, number, timestamp from blocks where hash = $1",
        [hash.into()],
    ))
    .one(db)
    .await
    .context("Failed to get block by hash")?
    .map(TryInto::try_into)
    .transpose()
}

#[instrument(skip(db))]
pub async fn count_unprocessed_txs<T: StreamTrait + ConnectionTrait>(db: &T) -> Result<u64> {
    golem_base_pending_transaction_operations::Entity::find()
        .count(db)
        .await
        .context("Failed to count pending txs")
}

#[instrument(skip(db))]
pub async fn count_txs_for_cleanup<T: StreamTrait + ConnectionTrait>(db: &T) -> Result<u64> {
    golem_base_pending_transaction_cleanups::Entity::find()
        .count(db)
        .await
        .context("Failed to count pending tx cleanups")
}
