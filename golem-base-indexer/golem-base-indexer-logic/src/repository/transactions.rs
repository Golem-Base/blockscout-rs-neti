use anyhow::{Context, Result};
use golem_base_indexer_entity::transactions::{
    self, Entity as TransactionsEntity, Model as TransactionsModel,
};
use sea_orm::{entity::prelude::*, Condition, FromQueryResult, QueryOrder, Statement};
use tracing::instrument;

use crate::{
    pagination::paginate_try_from,
    repository::sql,
    types::{
        BiggestSpenders, CurrencyAmount, PaginationMetadata, PaginationParams, Transaction, TxHash,
    },
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
            value: value
                .value
                .to_string()
                .parse::<CurrencyAmount>()
                .context("Failed to convert value to CurrencyAmount")?,
            input: value.input.clone().try_into()?,
            gas_price: value
                .gas_price
                .map(|v| {
                    v.to_string()
                        .parse::<u64>()
                        .context("Failed to convert gas_price to u64")
                })
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
                .map(|v| {
                    v.to_string()
                        .parse::<u64>()
                        .context("Failed to convert cumulative_gas_used to u64")
                })
                .transpose()?,
            created_contract_address_hash: value
                .created_contract_address_hash
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            error: value.error,
            r#type: value.r#type.map(|v| v as i32),
            l1_transaction_origin: value
                .l1_transaction_origin
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            l1_block_number: value.l1_block_number.map(|v| v as u64),
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbBiggestSpenders {
    rank: i64,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    address: Vec<u8>,
    total_fees: String,
}

impl TryFrom<DbBiggestSpenders> for BiggestSpenders {
    type Error = anyhow::Error;

    fn try_from(value: DbBiggestSpenders) -> Result<Self> {
        Ok(Self {
            rank: value.rank as u64,
            address: value.address.as_slice().try_into()?,
            total_fees: value
                .total_fees
                .parse::<CurrencyAmount>()
                .context("Failed to convert transaction_fees to CurrencyAmount")?,
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
        "delete from golem_base_pending_transaction_cleanups where hash = $1",
        [tx_hash.into()],
    ))
    .await
    .context("Failed to finish tx cleanup")?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn list_biggest_spenders<T: ConnectionTrait>(
    db: &T,
    pagination: PaginationParams,
) -> Result<(Vec<BiggestSpenders>, PaginationMetadata)> {
    let stmt = Statement::from_string(db.get_database_backend(), sql::FIND_TX_FEE_BIGGEST_SPENDERS);

    let paginator = DbBiggestSpenders::find_by_statement(stmt).paginate(db, pagination.page_size);

    paginate_try_from(paginator, pagination)
        .await
        .context("Failed to fetch biggest spenders")
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
