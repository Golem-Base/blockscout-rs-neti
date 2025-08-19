use anyhow::{Context, Result};
use sea_orm::{entity::prelude::*, FromQueryResult, Statement};
use tracing::instrument;

use crate::{
    pagination::paginate_try_from,
    types::{BiggestSpenders, BiggestSpendersFilter, CurrencyAmount, PaginationMetadata, TxHash},
};

#[derive(Debug, FromQueryResult)]
pub struct BiggestSpendersQueryResult {
    rank: i64,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    address: Vec<u8>,
    total_fees: String,
}

impl TryFrom<BiggestSpendersQueryResult> for BiggestSpenders {
    type Error = anyhow::Error;

    fn try_from(value: BiggestSpendersQueryResult) -> Result<Self> {
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
    filter: BiggestSpendersFilter,
) -> Result<(Vec<BiggestSpenders>, PaginationMetadata)> {
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
            SELECT 
                ROW_NUMBER() OVER(ORDER BY SUM(cumulative_gas_used * gas_price) DESC) as rank,
                from_address_hash as address, 
                CAST(SUM(cumulative_gas_used * gas_price) AS TEXT) as total_fees
                -- SUM(cumulative_gas_used * gas_price) as total_fees
            FROM transactions
            GROUP BY from_address_hash
            ORDER BY SUM(cumulative_gas_used * gas_price) DESC
        "#,
        [],
    );

    let paginator =
        BiggestSpendersQueryResult::find_by_statement(stmt).paginate(db, filter.page_size);

    paginate_try_from(paginator, filter.page, filter.page_size)
        .await
        .context("Failed to fetch biggest spenders")
}
