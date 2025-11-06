use anyhow::{Context, Result};
use golem_base_indexer_entity::blocks;
use sea_orm::{prelude::*, FromQueryResult, QueryOrder, QuerySelect};
use sea_query::{Alias, ExprTrait, Func};
use tracing::instrument;

use crate::types::{BlockGasUsageLimitPoint, ChartInfo};

#[derive(Debug, FromQueryResult)]
struct DbChartBlockGasUsageLimit {
    pub block_number: i64,
    pub gas_used: Decimal,
    pub gas_limit: Decimal,
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
    limit: u64,
) -> Result<(Vec<BlockGasUsageLimitPoint>, ChartInfo)> {
    let points = blocks::Entity::find()
        .select_only()
        .column_as(blocks::Column::Number, "block_number")
        .column(blocks::Column::GasUsed)
        .column(blocks::Column::GasLimit)
        .column_as(
            Expr::case(
                Expr::col(blocks::Column::GasLimit).gt(0),
                Func::round_with_precision(
                    Expr::col(blocks::Column::GasUsed)
                        .div(Expr::col(blocks::Column::GasLimit))
                        .mul(100),
                    2,
                ),
            )
            .finally(0)
            .cast_as(Alias::new("DOUBLE PRECISION")),
            "gas_usage_percentage",
        )
        .order_by_desc(blocks::Column::Number)
        .limit(limit)
        .into_model::<DbChartBlockGasUsageLimit>()
        .all(db)
        .await
        .context("Failed to get block gas usage and limit timeseries")?
        .into_iter()
        .rev()
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
