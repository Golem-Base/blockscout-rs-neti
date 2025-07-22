use crate::{
    repository::sql::GET_TX_BY_HASH,
    types::{Tx, TxHash},
};
use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement, StreamTrait};
use tracing::instrument;

use super::sql;
use crate::well_known;

#[derive(FromQueryResult)]
pub struct DbTxHash {
    pub hash: Vec<u8>,
}

#[derive(FromQueryResult)]
pub struct DbTx {
    pub hash: Vec<u8>,
    pub from_address_hash: Vec<u8>,
    pub to_address_hash: Vec<u8>,
    pub block_number: i32,
    pub block_hash: Vec<u8>,
    pub input: Vec<u8>,
    pub index: i32,
}

impl TryFrom<DbTx> for Tx {
    type Error = anyhow::Error;
    fn try_from(tx: DbTx) -> Result<Self> {
        Ok(Self {
            input: tx.input.into(),
            block_hash: tx.block_hash.as_slice().try_into()?,
            block_number: tx.block_number.try_into()?,
            from_address_hash: tx.from_address_hash.as_slice().try_into()?,
            to_address_hash: tx.to_address_hash.as_slice().try_into()?,
            hash: tx.hash.as_slice().try_into()?,
            index: tx.index.try_into()?,
        })
    }
}

#[instrument(
    name = "repository::transactions::stream_unprocessed_tx_hashes",
    skip(db)
)]
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

#[instrument(name = "repository::transactions::get_tx", skip(db))]
pub async fn get_tx<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<Option<Tx>> {
    DbTx::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        GET_TX_BY_HASH,
        [tx_hash.as_slice().into()],
    ))
    .one(db)
    .await?
    .map(TryInto::try_into)
    .transpose()
}

#[instrument(skip(db))]
pub async fn finish_tx_processing<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_transaction_operations where hash = $1",
        [tx_hash.into()],
    ))
    .await?;
    Ok(())
}
