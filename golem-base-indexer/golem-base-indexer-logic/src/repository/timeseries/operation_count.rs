use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use sea_query::{ExprTrait, Iden, PostgresQueryBuilder, Query, SelectStatement};
use std::collections::HashMap;
use tracing::instrument;

use crate::types::{ChartInfo, ChartPoint, OperationType};

use super::common::*;

#[derive(Iden)]
pub enum GolemBaseTimeseriesOperationCount {
    Table,
    Timestamp,
    Operation,
    OperationCount,
}

#[derive(Debug, FromQueryResult)]
struct DbChartOperationCountDaily {
    pub timestamp: NaiveDate,
    pub operation_count: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbChartOperationCountHourly {
    pub timestamp: NaiveDateTime,
    pub operation_count: i64,
}

impl OperationType {
    fn as_sql_string(&self) -> &str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Extend => "extend",
        }
    }
}

#[instrument(skip(db))]
pub async fn timeseries_operation_count<T: ConnectionTrait>(
    db: &T,
    from: Option<String>,
    to: Option<String>,
    resolution: ChartResolution,
    operation: Option<OperationType>,
) -> Result<(Vec<ChartPoint>, ChartInfo)> {
    let chart = match resolution {
        ChartResolution::Day => {
            let (from_date, to_date) = parse_date_range(from, to)?;
            let query = build_query_operation_count_daily(from_date, to_date, operation);
            let results = DbChartOperationCountDaily::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get operation count timeseries daily")?;

            let initial_value = if let Some(from_date) = from_date {
                let lookback_query = build_query_operation_count_daily_last_value(from_date);
                let lookback_result =
                    DbChartOperationCountDaily::find_by_statement(Statement::from_string(
                        DbBackend::Postgres,
                        lookback_query.to_string(PostgresQueryBuilder),
                    ))
                    .one(db)
                    .await
                    .context("Failed to get last known daily value")?;

                lookback_result.map(|row| row.operation_count)
            } else {
                None
            };

            generate_points_operation_count_daily(results, from_date, to_date, initial_value)?
        }
        ChartResolution::Hour => {
            let (from_datetime, to_datetime) = parse_datetime_range(from, to)?;
            let query = build_query_operation_count_hourly(from_datetime, to_datetime, operation);
            let results = DbChartOperationCountHourly::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get operation count timeseries hourly")?;

            let initial_value = if let Some(from_dt) = from_datetime {
                let lookback_query =
                    build_query_operation_count_hourly_last_value(from_dt, operation);
                let lookback_result =
                    DbChartOperationCountHourly::find_by_statement(Statement::from_string(
                        DbBackend::Postgres,
                        lookback_query.to_string(PostgresQueryBuilder),
                    ))
                    .one(db)
                    .await
                    .context("Failed to get last known hourly value")?;

                lookback_result.map(|row| row.operation_count)
            } else {
                None
            };

            generate_points_operation_count_hourly(
                results,
                from_datetime,
                to_datetime,
                initial_value,
            )?
        }
        _ => return Err(anyhow!("Unsupported chart resolution")),
    };

    let info = ChartInfo {
        id: "golemBaseOperationCount".to_string(),
        title: "Operations over time".to_string(),
        description: "Operations over time".to_string(),
    };

    Ok((chart, info))
}

fn build_query_operation_count_daily_last_value(before_date: NaiveDate) -> SelectStatement {
    Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseriesOperationCount::Timestamp).cast_as("date"),
            "timestamp",
        )
        .expr_as(
            Expr::max(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)),
            GolemBaseTimeseriesOperationCount::OperationCount,
        )
        .from(GolemBaseTimeseriesOperationCount::Table)
        .and_where(
            Expr::col(GolemBaseTimeseriesOperationCount::Timestamp)
                .cast_as("date")
                .lt(before_date),
        )
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Desc)
        .limit(1)
        .to_owned()
}

fn build_query_operation_count_daily(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    operation: Option<OperationType>,
) -> SelectStatement {
    let mut query = Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseriesOperationCount::Timestamp).cast_as("date"),
            "timestamp",
        )
        .from(GolemBaseTimeseriesOperationCount::Table)
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Asc)
        .to_owned();

    match (from, to) {
        (Some(from_date), Some(to_date)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Timestamp)
                    .cast_as("date")
                    .between(from_date, to_date),
            );
        }
        (Some(from_date), None) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Timestamp)
                    .cast_as("date")
                    .gte(from_date),
            );
        }
        (None, Some(to_date)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Timestamp)
                    .cast_as("date")
                    .lte(to_date),
            );
        }
        (None, None) => {}
    };

    match operation {
        Some(op) => query
            .expr_as(
                Expr::max(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)),
                GolemBaseTimeseriesOperationCount::OperationCount,
            )
            .and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Operation).eq(op.as_sql_string()),
            ),
        None => query.expr_as(
            Expr::sum(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)).cast_as("int8"),
            GolemBaseTimeseriesOperationCount::OperationCount,
        ),
    };

    query
}

fn build_query_operation_count_hourly_last_value(
    before_datetime: NaiveDateTime,
    operation: Option<OperationType>,
) -> SelectStatement {
    let mut query = Query::select()
        .columns([GolemBaseTimeseriesOperationCount::Timestamp])
        .from(GolemBaseTimeseriesOperationCount::Table)
        .and_where(Expr::col(GolemBaseTimeseriesOperationCount::Timestamp).lt(before_datetime))
        .group_by_col("timestamp")
        .order_by(
            GolemBaseTimeseriesOperationCount::Timestamp,
            sea_query::Order::Desc,
        )
        .limit(1)
        .to_owned();

    match operation {
        Some(op) => query
            .expr_as(
                Expr::max(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)),
                GolemBaseTimeseriesOperationCount::OperationCount,
            )
            .and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Operation).eq(op.as_sql_string()),
            ),
        None => query.expr_as(
            Expr::sum(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)).cast_as("int8"),
            GolemBaseTimeseriesOperationCount::OperationCount,
        ),
    };

    query
}

fn build_query_operation_count_hourly(
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
    operation: Option<OperationType>,
) -> SelectStatement {
    let mut query = Query::select()
        .columns([GolemBaseTimeseriesOperationCount::Timestamp])
        .group_by_col("timestamp")
        .from(GolemBaseTimeseriesOperationCount::Table)
        .order_by(
            GolemBaseTimeseriesOperationCount::Timestamp,
            sea_query::Order::Asc,
        )
        .to_owned();

    match (from, to) {
        (Some(from_datetime), Some(to_datetime)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Timestamp)
                    .between(from_datetime, to_datetime),
            );
        }
        (Some(from_datetime), None) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Timestamp).gte(from_datetime),
            );
        }
        (None, Some(to_datetime)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Timestamp).lte(to_datetime),
            );
        }
        (None, None) => {}
    }

    match operation {
        Some(op) => query
            .expr_as(
                Expr::max(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)),
                GolemBaseTimeseriesOperationCount::OperationCount,
            )
            .and_where(
                Expr::col(GolemBaseTimeseriesOperationCount::Operation).eq(op.as_sql_string()),
            ),
        None => query.expr_as(
            Expr::sum(Expr::col(GolemBaseTimeseriesOperationCount::OperationCount)).cast_as("int8"),
            GolemBaseTimeseriesOperationCount::OperationCount,
        ),
    };

    query
}

fn generate_points_operation_count_daily(
    db_results: Vec<DbChartOperationCountDaily>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    initial_value: Option<i64>,
) -> Result<Vec<ChartPoint>> {
    let data_map: HashMap<NaiveDate, i64> = db_results
        .into_iter()
        .map(|row| (row.timestamp, row.operation_count))
        .collect();

    let start_date = match from_date {
        Some(date) => date,
        None => data_map
            .keys()
            .min()
            .copied()
            .unwrap_or_else(|| Utc::now().naive_utc().date()),
    };

    let end_date = match to_date {
        Some(date) => date,
        None => Utc::now().naive_utc().date(),
    };

    let mut points = Vec::new();
    let mut current_date = start_date;
    let mut last_known_value = initial_value;

    while current_date < end_date {
        let next_date = current_date + Duration::days(1);

        let value = match data_map.get(&current_date) {
            Some(&actual_value) => {
                last_known_value = Some(actual_value);
                actual_value
            }
            None => last_known_value.unwrap_or(0),
        };

        points.push(ChartPoint {
            date: current_date.format("%Y-%m-%d").to_string(),
            date_to: next_date.format("%Y-%m-%d").to_string(),
            value: value.to_string(),
        });

        current_date = next_date;
    }

    Ok(points)
}

fn generate_points_operation_count_hourly(
    db_results: Vec<DbChartOperationCountHourly>,
    from_datetime: Option<NaiveDateTime>,
    to_datetime: Option<NaiveDateTime>,
    initial_value: Option<i64>,
) -> Result<Vec<ChartPoint>> {
    let data_map: HashMap<NaiveDateTime, i64> = db_results
        .into_iter()
        .map(|row| (row.timestamp, row.operation_count))
        .collect();

    let start_time = match from_datetime {
        Some(dt) => dt,
        None => data_map
            .keys()
            .min()
            .copied()
            .unwrap_or_else(|| Utc::now().naive_utc()),
    };

    let end_time = match to_datetime {
        Some(dt) => dt,
        None => Utc::now().naive_utc(),
    };

    let mut points = Vec::new();
    let mut current_time = start_time;
    let mut last_known_value = initial_value;

    while current_time < end_time {
        let next_hour = current_time + Duration::hours(1);

        let value = match data_map.get(&current_time) {
            Some(&actual_value) => {
                last_known_value = Some(actual_value);
                actual_value
            }
            None => last_known_value.unwrap_or(0),
        };

        points.push(ChartPoint {
            date: current_time.format("%Y-%m-%d %H:%M").to_string(),
            date_to: next_hour.format("%Y-%m-%d %H:%M").to_string(),
            value: value.to_string(),
        });

        current_time = next_hour;
    }

    Ok(points)
}
