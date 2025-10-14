use anyhow::{Context, Result};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use sea_query::{Expr, Iden, PostgresQueryBuilder, Query, SelectStatement};
use tracing::instrument;

use crate::types::{BlockTransactionPoint, ChartInfo};

#[derive(Iden)]
pub enum Transactions {
    Table,
    BlockNumber,
}

#[derive(Debug, FromQueryResult)]
struct DbChartBlockTransactions {
    pub block_number: i32,
    pub tx_count: i64,
}

const DEFAULT_BLOCK_LIMIT: u64 = 100;

#[instrument(skip(db))]
pub async fn timeseries_block_transactions<T: ConnectionTrait>(
    db: &T,
) -> Result<(Vec<BlockTransactionPoint>, ChartInfo)> {
    let query = build_query_block_transactions();
    let results = DbChartBlockTransactions::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        query.to_string(PostgresQueryBuilder),
    ))
    .all(db)
    .await
    .context("Failed to get block transactions timeseries")?;

    let chart = generate_points_block_transactions(results)?;

    let info = ChartInfo {
        id: "blockTransactions".to_string(),
        title: "Block Transactions".to_string(),
        description: "Number of transactions for recent blocks".to_string(),
    };

    Ok((chart, info))
}

fn build_query_block_transactions() -> SelectStatement {
    Query::select()
        .column(Transactions::BlockNumber)
        .expr_as(Expr::cust("COUNT(*)"), "tx_count")
        .from(Transactions::Table)
        .and_where(Expr::col(Transactions::BlockNumber).is_not_null())
        .and_where(Expr::col("block_consensus").eq(true))
        .group_by_col(Transactions::BlockNumber)
        .order_by(Transactions::BlockNumber, sea_query::Order::Desc)
        .limit(DEFAULT_BLOCK_LIMIT)
        .to_owned()
}

fn generate_points_block_transactions(
    mut db_results: Vec<DbChartBlockTransactions>,
) -> Result<Vec<BlockTransactionPoint>> {
    // Get ASC order for the chart
    db_results.reverse();

    let points: Vec<BlockTransactionPoint> = db_results
        .into_iter()
        .map(|row| BlockTransactionPoint {
            block_number: row.block_number as u64,
            tx_count: row.tx_count as u64,
        })
        .collect();

    Ok(points)
}
