use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use sea_query::{ExprTrait, Iden, PostgresQueryBuilder, Query, SelectStatement};
use std::collections::HashMap;
use tracing::instrument;

use crate::types::{ChartInfo, ChartPoint};

#[derive(Debug)]
pub enum ChartResolution {
    Day,
    Hour,
}

impl TryFrom<i32> for ChartResolution {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self> {
        match value {
            0 => Ok(ChartResolution::Day),
            1 => Ok(ChartResolution::Hour),
            _ => Err(anyhow!("Error converting chart resolution")),
        }
    }
}

#[derive(Iden)]
pub enum GolemBaseTimeseries {
    Table,
    Timestamp,
    ActiveDataBytes,
}

#[derive(Debug, FromQueryResult)]
struct DbChartDataUsageDaily {
    pub timestamp: NaiveDate,
    pub active_data_bytes: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbChartDataUsageHourly {
    pub timestamp: NaiveDateTime,
    pub active_data_bytes: i64,
}

#[instrument(skip(db))]
pub async fn chart_data_usage<T: ConnectionTrait>(
    db: &T,
    from: Option<String>,
    to: Option<String>,
    resolution: ChartResolution,
) -> Result<(Vec<ChartPoint>, ChartInfo)> {
    let chart = match resolution {
        ChartResolution::Day => {
            let (from_date, to_date) = parse_date_range(from, to)?;
            let query = build_daily_data_usage_query(from_date, to_date);
            let results = DbChartDataUsageDaily::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get data usage timeseries")?;

            let initial_value = if let Some(from_date) = from_date {
                let lookback_query = build_daily_value_before_query(from_date);
                let lookback_result =
                    DbChartDataUsageDaily::find_by_statement(Statement::from_string(
                        DbBackend::Postgres,
                        lookback_query.to_string(PostgresQueryBuilder),
                    ))
                    .one(db)
                    .await
                    .context("Failed to get last known daily value")?;

                lookback_result.map(|row| row.active_data_bytes)
            } else {
                None
            };

            generate_daily_points(results, from_date, to_date, initial_value)?
        }
        ChartResolution::Hour => {
            let (from_datetime, to_datetime) = parse_datetime_range(from, to)?;
            let query = build_hourly_data_usage_query(from_datetime, to_datetime);
            let results = DbChartDataUsageHourly::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get data usage timeseries")?;

            let initial_value = if let Some(from_dt) = from_datetime {
                let lookback_query = build_hourly_last_value_before_query(from_dt);
                let lookback_result =
                    DbChartDataUsageHourly::find_by_statement(Statement::from_string(
                        DbBackend::Postgres,
                        lookback_query.to_string(PostgresQueryBuilder),
                    ))
                    .one(db)
                    .await
                    .context("Failed to get last known hourly value")?;

                lookback_result.map(|row| row.active_data_bytes)
            } else {
                None
            };

            generate_hourly_points(results, from_datetime, to_datetime, initial_value)?
        }
    };

    let info = ChartInfo {
        id: "golemBaseDataUsage".to_string(),
        title: "Data over time".to_string(),
        description: "Data storage over time".to_string(),
    };

    Ok((chart, info))
}

fn parse_date_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>)> {
    let from_date = match from {
        Some(date_str) => Some(
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid from date format: {}", e))?,
        ),
        None => None,
    };

    let to_date = match to {
        Some(date_str) => Some(
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid to date format: {}", e))?,
        ),
        None => None,
    };

    if let (Some(from), Some(to)) = (from_date, to_date) {
        if from > to {
            return Err(anyhow!(
                "From date ({}) cannot be later than to date ({})",
                from.format("%Y-%m-%d"),
                to.format("%Y-%m-%d")
            ));
        }
    }

    Ok((from_date, to_date))
}

fn parse_datetime_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<(Option<NaiveDateTime>, Option<NaiveDateTime>)> {
    let current_datetime = Utc::now().naive_utc();

    let from_datetime = match from {
        Some(datetime_str) => Some(
            NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                .map_err(|e| anyhow!("Invalid from datetime format: {}", e))?,
        ),
        None => None,
    };

    let to_datetime = match to {
        Some(datetime_str) => {
            let parsed = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                .map_err(|e| anyhow!("Invalid to datetime format: {}", e))?;

            Some(if parsed > current_datetime {
                current_datetime
            } else {
                parsed
            })
        }
        None => Some(Utc::now().naive_utc()),
    };

    if let (Some(from), Some(to)) = (from_datetime, to_datetime) {
        if from > to {
            return Err(anyhow!(
                "From datetime ({}) cannot be later than to datetime ({})",
                from.format("%Y-%m-%d %H:%M"),
                to.format("%Y-%m-%d %H:%M")
            ));
        }
    }

    Ok((from_datetime, to_datetime))
}

fn build_daily_value_before_query(before_date: NaiveDate) -> SelectStatement {
    Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseries::Timestamp).cast_as("date"),
            "timestamp",
        )
        .expr_as(
            Expr::max(Expr::col(GolemBaseTimeseries::ActiveDataBytes)),
            GolemBaseTimeseries::ActiveDataBytes,
        )
        .from(GolemBaseTimeseries::Table)
        .and_where(
            Expr::col(GolemBaseTimeseries::Timestamp)
                .cast_as("date")
                .lt(before_date),
        )
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Desc)
        .limit(1)
        .to_owned()
}

fn build_daily_data_usage_query(from: Option<NaiveDate>, to: Option<NaiveDate>) -> SelectStatement {
    let mut query = Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseries::Timestamp).cast_as("date"),
            "timestamp",
        )
        .expr_as(
            Expr::max(Expr::col(GolemBaseTimeseries::ActiveDataBytes)),
            GolemBaseTimeseries::ActiveDataBytes,
        )
        .from(GolemBaseTimeseries::Table)
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Asc)
        .to_owned();

    match (from, to) {
        (Some(from_date), Some(to_date)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseries::Timestamp)
                    .cast_as("date")
                    .between(from_date, to_date),
            );
        }
        (Some(from_date), None) => {
            query.and_where(
                Expr::col(GolemBaseTimeseries::Timestamp)
                    .cast_as("date")
                    .gte(from_date),
            );
        }
        (None, Some(to_date)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseries::Timestamp)
                    .cast_as("date")
                    .lte(to_date),
            );
        }
        (None, None) => {}
    }

    query
}

fn generate_daily_points(
    db_results: Vec<DbChartDataUsageDaily>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    initial_value: Option<i64>,
) -> Result<Vec<ChartPoint>> {
    if db_results.is_empty() && initial_value.is_none() {
        return Err(anyhow!("No data usage available"));
    }

    let data_map: HashMap<NaiveDate, i64> = db_results
        .into_iter()
        .map(|row| (row.timestamp, row.active_data_bytes))
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

    while current_date <= end_date {
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

fn build_hourly_last_value_before_query(before_datetime: NaiveDateTime) -> SelectStatement {
    Query::select()
        .columns([
            GolemBaseTimeseries::Timestamp,
            GolemBaseTimeseries::ActiveDataBytes,
        ])
        .from(GolemBaseTimeseries::Table)
        .and_where(Expr::col(GolemBaseTimeseries::Timestamp).lt(before_datetime))
        .order_by(GolemBaseTimeseries::Timestamp, sea_query::Order::Desc)
        .limit(1)
        .to_owned()
}

fn build_hourly_data_usage_query(
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
) -> SelectStatement {
    let mut query = Query::select()
        .columns([
            GolemBaseTimeseries::Timestamp,
            GolemBaseTimeseries::ActiveDataBytes,
        ])
        .from(GolemBaseTimeseries::Table)
        .order_by(GolemBaseTimeseries::Timestamp, sea_query::Order::Asc)
        .to_owned();

    match (from, to) {
        (Some(from_datetime), Some(to_datetime)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseries::Timestamp).between(from_datetime, to_datetime),
            );
        }
        (Some(from_datetime), None) => {
            query.and_where(Expr::col(GolemBaseTimeseries::Timestamp).gte(from_datetime));
        }
        (None, Some(to_datetime)) => {
            query.and_where(Expr::col(GolemBaseTimeseries::Timestamp).lte(to_datetime));
        }
        (None, None) => {}
    }

    query
}

fn generate_hourly_points(
    db_results: Vec<DbChartDataUsageHourly>,
    from_datetime: Option<NaiveDateTime>,
    to_datetime: Option<NaiveDateTime>,
    initial_value: Option<i64>,
) -> Result<Vec<ChartPoint>> {
    if db_results.is_empty() && initial_value.is_none() {
        return Err(anyhow!("No data usage available"));
    }

    let data_map: HashMap<NaiveDateTime, i64> = db_results
        .into_iter()
        .map(|row| (row.timestamp, row.active_data_bytes))
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

    while current_time <= end_time {
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
