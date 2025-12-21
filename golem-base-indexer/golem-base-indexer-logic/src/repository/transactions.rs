use anyhow::{Context, Result};
use golem_base_indexer_entity::transactions::{
    self, Entity as TransactionsEntity, Model as TransactionsModel,
};
use sea_orm::{entity::prelude::*, Condition, QueryOrder, Statement};
use std::str::FromStr;
use tracing::instrument;

use crate::{
    pagination::paginate_try_from,
    types::{CurrencyAmount, PaginationMetadata, PaginationParams, Transaction, TxHash},
    well_known::{
        DEPOSIT_CONTRACT_ADDRESS, GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS, L1_BLOCK_CONTRACT_ADDRESS,
        L1_BLOCK_CONTRACT_SENDER_ADDRESS,
    },
};

impl TryFrom<TransactionsModel> for Transaction {
    type Error = anyhow::Error;

    fn try_from(value: TransactionsModel) -> Result<Self> {
        Ok(Self {
            hash: value.hash.as_slice().try_into()?,
            block_number: value.block_number.map(|v| v as u64),
            index: value.index.map(|v| v as u64),
            from_address_hash: value.from_address_hash.as_slice().try_into()?,
            to_address_hash: value
                .to_address_hash
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            value: CurrencyAmount::from_str(&value.value.to_plain_string())
                .context("Failed to convert value to CurrencyAmount")?,
            input: value.input.into(),
            gas_price: value
                .gas_price
                .map(|v| CurrencyAmount::from_str(&v.to_plain_string()))
                .transpose()?,
            status: value.status.map(|v| v as u8),
            block_hash: value
                .block_hash
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            block_consensus: value.block_consensus,
            block_timestamp: value.block_timestamp.map(|v| v.and_utc()),
            cumulative_gas_used: value
                .cumulative_gas_used
                .map(|v| CurrencyAmount::from_str(&v.to_plain_string()))
                .transpose()?,
            created_contract_address_hash: value
                .created_contract_address_hash
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            error: value.error,
            r#type: value.r#type,
            l1_transaction_origin: value
                .l1_transaction_origin
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            l1_block_number: value.l1_block_number.map(|v| v as u64),
        })
    }
}

#[instrument(skip(db))]
pub async fn finish_tx_processing<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_transaction_operations where hash = $1",
        [tx_hash.into()],
    ))
    .await
    .context("Failed to finish tx processing")?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn finish_tx_cleanup<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_logs_operations where transaction_hash = $1",
        [tx_hash.clone().into()],
    ))
    .await
    .context("Failed to finish tx cleanup - logs")?;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_transaction_cleanups where hash = $1",
        [tx_hash.into()],
    ))
    .await
    .context("Failed to finish tx cleanup")?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn list_custom_contract_transactions<T: ConnectionTrait>(
    db: &T,
    pagination: PaginationParams,
) -> Result<(Vec<Transaction>, PaginationMetadata)> {
    let not_storage_tx =
        transactions::Column::ToAddressHash.ne(GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS.to_vec());
    let not_deposit_tx = transactions::Column::ToAddressHash.ne(DEPOSIT_CONTRACT_ADDRESS.to_vec());
    let housekeeping_tx = Condition::all()
        .add(transactions::Column::FromAddressHash.eq(L1_BLOCK_CONTRACT_SENDER_ADDRESS.to_vec()))
        .add(transactions::Column::ToAddressHash.eq(L1_BLOCK_CONTRACT_ADDRESS.to_vec()));

    let filter = Condition::all()
        .add(not_storage_tx)
        .add(not_deposit_tx)
        .add(Condition::not(housekeeping_tx));

    let paginator = TransactionsEntity::find()
        .filter(filter)
        .order_by_desc(transactions::Column::BlockNumber)
        .order_by_desc(transactions::Column::Index)
        .paginate(db, pagination.page_size);

    paginate_try_from(paginator, pagination)
        .await
        .context("Failed to fetch custom contract transactions")
}
