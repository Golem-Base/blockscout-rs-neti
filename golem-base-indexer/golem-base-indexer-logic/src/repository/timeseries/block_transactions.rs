use anyhow::{Context, Result};
use sea_orm::{prelude::*, FromQueryResult, QueryOrder, QuerySelect};
use tracing::instrument;

use crate::types::{BlockTransactionPoint, ChartInfo};
use golem_base_indexer_entity::transactions;

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
    let results = transactions::Entity::find()
        .select_only()
        .column(transactions::Column::BlockNumber)
        .column_as(transactions::Column::BlockNumber.count(), "tx_count")
        .filter(transactions::Column::BlockNumber.is_not_null())
        .filter(transactions::Column::BlockConsensus.eq(true))
        .group_by(transactions::Column::BlockNumber)
        .order_by_desc(transactions::Column::BlockNumber)
        .limit(DEFAULT_BLOCK_LIMIT)
        .into_model::<DbChartBlockTransactions>()
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
