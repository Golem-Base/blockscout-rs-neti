use anyhow::{Context, Result};
use sea_orm::{prelude::*, FromQueryResult, Statement};
use tracing::instrument;

use crate::{
    repository::sql::BLOCK_OPERATIONS_TIMESERIES,
    types::{BlockOperationPoint, ChartInfo},
};

#[derive(Debug, FromQueryResult)]
struct DbChartBlockOperations {
    pub block_number: i64,
    pub create_count: i64,
    pub update_count: i64,
    pub delete_count: i64,
    pub extend_count: i64,
}

#[instrument(skip(db))]
pub async fn timeseries_block_operations<T: ConnectionTrait>(
    db: &T,
    limit: u64,
) -> Result<(Vec<BlockOperationPoint>, ChartInfo)> {
    let results = DbChartBlockOperations::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        BLOCK_OPERATIONS_TIMESERIES,
        [limit.into()],
    ))
    .all(db)
    .await
    .context("Failed to get block operations timeseries")?;

    let chart = generate_points_block_operations(results)?;

    let info = ChartInfo {
        id: "blockOperations".to_string(),
        title: "Block Operations".to_string(),
        description: "Number of operations per block by type".to_string(),
    };

    Ok((chart, info))
}

fn generate_points_block_operations(
    mut db_results: Vec<DbChartBlockOperations>,
) -> Result<Vec<BlockOperationPoint>> {
    db_results.reverse();

    let points: Vec<BlockOperationPoint> = db_results
        .into_iter()
        .map(|row| BlockOperationPoint {
            block_number: row.block_number as u64,
            create_count: row.create_count as u64,
            update_count: row.update_count as u64,
            delete_count: row.delete_count as u64,
            extend_count: row.extend_count as u64,
        })
        .collect();

    Ok(points)
}
