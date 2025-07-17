use alloy_primitives::{Address, BlockHash, BlockNumber, TxHash, B256};
use anyhow::Result;
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
    pub block_number: i32,
    pub block_hash: Vec<u8>,
    pub input: Vec<u8>,
    pub index: i32,
}

#[derive(Debug)]
pub struct Tx {
    pub hash: TxHash,
    pub from_address_hash: Address,
    pub block_number: BlockNumber,
    pub block_hash: BlockHash,
    pub input: Vec<u8>,
    pub index: i32,
}

#[instrument(
    name = "repository::transactions::stream_unprocessed_tx_hashes",
    skip(db),
    level = "info"
)]
pub async fn stream_unprocessed_tx_hashes<T: StreamTrait + ConnectionTrait>(
    db: &T,
) -> Result<impl Stream<Item = B256> + '_> {
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
    .await?
    .filter_map(|tx| async {
        match tx {
            Ok(tx) => Some(B256::from_slice(&tx.hash)),
            Err(err) => {
                tracing::error!(error = ?err, "error during unprocessed tx hash retrieval");
                None
            }
        }
    }))
}

#[instrument(name = "repository::transactions::get_tx", skip(db), level = "info")]
pub async fn get_tx<T: ConnectionTrait>(db: &T, tx_hash: B256) -> Result<Option<Tx>> {
    DbTx::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
select 
    from_address_hash,
    hash,
    block_number,
    block_hash,
    index,
    input
from transactions
where hash = $1
        "#,
        [tx_hash.as_slice().into()],
    ))
    .one(db)
    .await?
    .map(|tx| {
        Ok(Tx {
            input: tx.input,
            block_hash: tx.block_hash.as_slice().try_into()?,
            block_number: tx.block_number.try_into()?,
            from_address_hash: tx.from_address_hash.as_slice().try_into()?,
            hash: tx.hash.as_slice().try_into()?,
            index: tx.index,
        })
    })
    .transpose()
}
