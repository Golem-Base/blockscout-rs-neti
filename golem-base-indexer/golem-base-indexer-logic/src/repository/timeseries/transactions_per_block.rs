use anyhow::{Context, Result};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use sea_query::{Expr, Iden, PostgresQueryBuilder, Query, SelectStatement};
use tracing::instrument;

use crate::types::{ChartInfo, ChartPoint};

#[derive(Iden)]
pub enum Transactions {
    Table,
    BlockNumber,
}

#[derive(Debug, FromQueryResult)]
struct DbChartTransactionsPerBlock {
    pub block_number: i32,
    pub transaction_count: i64,
}

const DEFAULT_BLOCK_LIMIT: u64 = 100;

#[instrument(skip(db))]
pub async fn timeseries_transactions_per_block<T: ConnectionTrait>(
    db: &T,
) -> Result<(Vec<ChartPoint>, ChartInfo)> {
    let query = build_query_transactions_per_block();
    let results = DbChartTransactionsPerBlock::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        query.to_string(PostgresQueryBuilder),
    ))
    .all(db)
    .await
    .context("Failed to get transactions per block timeseries")?;

    let chart = generate_points_transactions_per_block(results)?;

    let info = ChartInfo {
        id: "transactionsPerBlock".to_string(),
        title: "Transactions per Block".to_string(),
        description: "Number of transactions for recent blocks".to_string(),
    };

    Ok((chart, info))
}

fn build_query_transactions_per_block() -> SelectStatement {
    Query::select()
        .column(Transactions::BlockNumber)
        .expr_as(Expr::cust("COUNT(*)"), "transaction_count")
        .from(Transactions::Table)
        .and_where(Expr::col(Transactions::BlockNumber).is_not_null())
        .and_where(Expr::cust("block_consensus = true"))
        .group_by_col(Transactions::BlockNumber)
        .order_by(Transactions::BlockNumber, sea_query::Order::Desc)
        .limit(DEFAULT_BLOCK_LIMIT)
        .to_owned()
}

fn generate_points_transactions_per_block(
    mut db_results: Vec<DbChartTransactionsPerBlock>,
) -> Result<Vec<ChartPoint>> {
    // Results come in DESC order, reverse them to get ASC order for the chart
    db_results.reverse();

    let points: Vec<ChartPoint> = db_results
        .into_iter()
        .map(|row| ChartPoint {
            date: row.block_number.to_string(),
            date_to: (row.block_number + 1).to_string(),
            value: row.transaction_count.to_string(),
        })
        .collect();

    Ok(points)
}


