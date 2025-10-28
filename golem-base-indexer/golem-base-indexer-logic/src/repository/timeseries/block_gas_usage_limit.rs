use anyhow::{Context, Result};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use sea_query::{Iden, PostgresQueryBuilder, Query};
use tracing::instrument;

use crate::types::{BlockGasUsageLimitPoint, ChartInfo};

#[derive(Iden)]
pub enum GolemBaseTimeseriesBlockGasUsedAndLimit {
    Table,
    BlockNumber,
    GasUsed,
    GasLimit,
    GasUsagePercentage,
}

#[derive(Debug, FromQueryResult)]
struct DbChartBlockGasUsageLimit {
    pub block_number: i64,
    pub gas_used: i64,
    pub gas_limit: i64,
    pub gas_usage_percentage: f64,
}

impl TryFrom<DbChartBlockGasUsageLimit> for BlockGasUsageLimitPoint {
    type Error = anyhow::Error;

    fn try_from(value: DbChartBlockGasUsageLimit) -> Result<Self> {
        Ok(Self {
            block_number: value.block_number.try_into()?,
            gas_used: value.gas_used.try_into()?,
            gas_limit: value.gas_limit.try_into()?,
            gas_usage_percentage: value.gas_usage_percentage,
        })
    }
}

#[instrument(skip(db))]
pub async fn timeseries_block_gas_usage_limit<T: ConnectionTrait>(
    db: &T,
) -> Result<(Vec<BlockGasUsageLimitPoint>, ChartInfo)> {
    let query = Query::select()
        .columns([
            GolemBaseTimeseriesBlockGasUsedAndLimit::BlockNumber,
            GolemBaseTimeseriesBlockGasUsedAndLimit::GasUsed,
            GolemBaseTimeseriesBlockGasUsedAndLimit::GasLimit,
            GolemBaseTimeseriesBlockGasUsedAndLimit::GasUsagePercentage,
        ])
        .from(GolemBaseTimeseriesBlockGasUsedAndLimit::Table)
        .order_by(
            GolemBaseTimeseriesBlockGasUsedAndLimit::BlockNumber,
            sea_query::Order::Asc,
        )
        .to_owned();

    let points = DbChartBlockGasUsageLimit::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        query.to_string(PostgresQueryBuilder),
    ))
    .all(db)
    .await
    .context("Failed to get block gas usage and limit timeseries")?
    .into_iter()
    .map(|db_point| db_point.try_into())
    .collect::<Result<Vec<BlockGasUsageLimitPoint>>>()
    .context("Failed to convert chart data for block gas usage and limit timeseries")?;

    let info = ChartInfo {
        id: "golemBaseBlockGasUsageLimit".to_string(),
        title: "Gas usage over time".to_string(),
        description: "Per block gas used and gas limit".to_string(),
    };

    Ok((points, info))
}
