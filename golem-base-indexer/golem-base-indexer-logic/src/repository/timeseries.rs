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
pub enum GolemBaseTimeseriesDataUsage {
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

#[derive(Iden)]
pub enum GolemBaseTimeseriesStorageForecast {
    Table,
    Timestamp,
    TotalStorage,
}

#[derive(Debug, FromQueryResult)]
struct DbChartStorageForecastDaily {
    pub timestamp: NaiveDate,
    pub total_storage: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbChartStorageForecastHourly {
    pub timestamp: NaiveDateTime,
    pub total_storage: i64,
}

#[instrument(skip(db))]
pub async fn timeseries_data_usage<T: ConnectionTrait>(
    db: &T,
    from: Option<String>,
    to: Option<String>,
    resolution: ChartResolution,
) -> Result<(Vec<ChartPoint>, ChartInfo)> {
    let chart = match resolution {
        ChartResolution::Day => {
            let (from_date, to_date) = parse_date_range(from, to)?;
            let query = build_query_data_usage_daily(from_date, to_date);
            let results = DbChartDataUsageDaily::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get data usage timeseries")?;

            let initial_value = if let Some(from_date) = from_date {
                let lookback_query = build_query_data_usage_daily_last_value(from_date);
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

            generate_points_data_usage_daily(results, from_date, to_date, initial_value)?
        }
        ChartResolution::Hour => {
            let (from_datetime, to_datetime) = parse_datetime_range(from, to)?;
            let query = build_query_data_usage_hourly(from_datetime, to_datetime);
            let results = DbChartDataUsageHourly::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get data usage timeseries")?;

            let initial_value = if let Some(from_dt) = from_datetime {
                let lookback_query = build_query_data_usage_hourly_last_value(from_dt);
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

            generate_points_data_usage_hourly(results, from_datetime, to_datetime, initial_value)?
        }
    };

    let info = ChartInfo {
        id: "golemBaseDataUsage".to_string(),
        title: "Data over time".to_string(),
        description: "Data storage over time".to_string(),
    };

    Ok((chart, info))
}

#[instrument(skip(db))]
pub async fn timeseries_storage_forecast<T: ConnectionTrait>(
    db: &T,
    to: &str,
    resolution: ChartResolution,
) -> Result<(Vec<ChartPoint>, ChartInfo)> {
    let chart = match resolution {
        ChartResolution::Day => {
            let (_, to_date) = parse_date_range(None, Some(to.to_string()))?;
            let query = build_query_storage_forecast_daily(to_date.unwrap());
            let results = DbChartStorageForecastDaily::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get storage forecast timeseries")?;

            tracing::warn!("results = {:#?}", results);

            generate_points_storage_forecast_daily(results, to_date.unwrap())?
        }
        ChartResolution::Hour => {
            let (_, to_datetime) = parse_datetime_range(None, Some(to.to_string()))?;
            let query = build_query_storage_forecast_hourly(to_datetime.unwrap());
            let results = DbChartStorageForecastHourly::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get storage forecast timeseries")?;

            tracing::warn!("results = {:#?}", results);

            generate_points_storage_forecast_hourly(results, to_datetime.unwrap())?
        }
    };

    let info = ChartInfo {
        id: "golemBaseStorageForecast".to_string(),
        title: "Storage forecast".to_string(),
        description: "Chain storage forecast".to_string(),
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

            Some(if from_datetime.is_some() && parsed > current_datetime {
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

fn build_query_data_usage_daily_last_value(before_date: NaiveDate) -> SelectStatement {
    Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseriesDataUsage::Timestamp).cast_as("date"),
            "timestamp",
        )
        .expr_as(
            Expr::max(Expr::col(GolemBaseTimeseriesDataUsage::ActiveDataBytes)),
            GolemBaseTimeseriesDataUsage::ActiveDataBytes,
        )
        .from(GolemBaseTimeseriesDataUsage::Table)
        .and_where(
            Expr::col(GolemBaseTimeseriesDataUsage::Timestamp)
                .cast_as("date")
                .lt(before_date),
        )
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Desc)
        .limit(1)
        .to_owned()
}

fn build_query_data_usage_daily(from: Option<NaiveDate>, to: Option<NaiveDate>) -> SelectStatement {
    let mut query = Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseriesDataUsage::Timestamp).cast_as("date"),
            "timestamp",
        )
        .expr_as(
            Expr::max(Expr::col(GolemBaseTimeseriesDataUsage::ActiveDataBytes)),
            GolemBaseTimeseriesDataUsage::ActiveDataBytes,
        )
        .from(GolemBaseTimeseriesDataUsage::Table)
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Asc)
        .to_owned();

    match (from, to) {
        (Some(from_date), Some(to_date)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesDataUsage::Timestamp)
                    .cast_as("date")
                    .between(from_date, to_date),
            );
        }
        (Some(from_date), None) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesDataUsage::Timestamp)
                    .cast_as("date")
                    .gte(from_date),
            );
        }
        (None, Some(to_date)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesDataUsage::Timestamp)
                    .cast_as("date")
                    .lte(to_date),
            );
        }
        (None, None) => {}
    }

    query
}

fn build_query_data_usage_hourly_last_value(before_datetime: NaiveDateTime) -> SelectStatement {
    Query::select()
        .columns([
            GolemBaseTimeseriesDataUsage::Timestamp,
            GolemBaseTimeseriesDataUsage::ActiveDataBytes,
        ])
        .from(GolemBaseTimeseriesDataUsage::Table)
        .and_where(Expr::col(GolemBaseTimeseriesDataUsage::Timestamp).lt(before_datetime))
        .order_by(
            GolemBaseTimeseriesDataUsage::Timestamp,
            sea_query::Order::Desc,
        )
        .limit(1)
        .to_owned()
}

fn build_query_data_usage_hourly(
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
) -> SelectStatement {
    let mut query = Query::select()
        .columns([
            GolemBaseTimeseriesDataUsage::Timestamp,
            GolemBaseTimeseriesDataUsage::ActiveDataBytes,
        ])
        .from(GolemBaseTimeseriesDataUsage::Table)
        .order_by(
            GolemBaseTimeseriesDataUsage::Timestamp,
            sea_query::Order::Asc,
        )
        .to_owned();

    match (from, to) {
        (Some(from_datetime), Some(to_datetime)) => {
            query.and_where(
                Expr::col(GolemBaseTimeseriesDataUsage::Timestamp)
                    .between(from_datetime, to_datetime),
            );
        }
        (Some(from_datetime), None) => {
            query.and_where(Expr::col(GolemBaseTimeseriesDataUsage::Timestamp).gte(from_datetime));
        }
        (None, Some(to_datetime)) => {
            query.and_where(Expr::col(GolemBaseTimeseriesDataUsage::Timestamp).lte(to_datetime));
        }
        (None, None) => {}
    }

    query
}

fn build_query_storage_forecast_daily(to_date: NaiveDate) -> SelectStatement {
    Query::select()
        .expr_as(
            Expr::col(GolemBaseTimeseriesStorageForecast::Timestamp).cast_as("date"),
            "timestamp",
        )
        .expr_as(
            Expr::max(Expr::col(GolemBaseTimeseriesStorageForecast::TotalStorage)),
            GolemBaseTimeseriesStorageForecast::TotalStorage,
        )
        .from(GolemBaseTimeseriesStorageForecast::Table)
        .and_where(
            Expr::col(GolemBaseTimeseriesStorageForecast::Timestamp)
                .cast_as("date")
                .lte(to_date),
        )
        .group_by_col("timestamp")
        .order_by("timestamp", sea_query::Order::Desc)
        .to_owned()
}

fn build_query_storage_forecast_hourly(to_datetime: NaiveDateTime) -> SelectStatement {
    Query::select()
        .columns([
            GolemBaseTimeseriesStorageForecast::Timestamp,
            GolemBaseTimeseriesStorageForecast::TotalStorage,
        ])
        .from(GolemBaseTimeseriesStorageForecast::Table)
        .and_where(Expr::col(GolemBaseTimeseriesStorageForecast::Timestamp).lte(to_datetime))
        .order_by(
            GolemBaseTimeseriesStorageForecast::Timestamp,
            sea_query::Order::Asc,
        )
        .to_owned()
}

fn generate_points_data_usage_daily(
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

fn generate_points_data_usage_hourly(
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

fn generate_points_storage_forecast_hourly(
    db_results: Vec<DbChartStorageForecastHourly>,
    to_datetime: NaiveDateTime,
) -> Result<Vec<ChartPoint>> {
    if db_results.is_empty() {
        return Err(anyhow!("No data usage available"));
    }

    let first_entry = &db_results[0];
    let initial_value = first_entry.total_storage;
    let start_time = first_entry.timestamp;

    let data_map: HashMap<NaiveDateTime, i64> = db_results
        .into_iter()
        .skip(1)
        .map(|row| (row.timestamp, row.total_storage))
        .collect();

    let mut points = Vec::new();
    let mut current_time = start_time;
    let mut last_known_value = initial_value;

    while current_time < to_datetime {
        let next_hour = current_time + Duration::hours(1);

        let value = match data_map.get(&current_time) {
            Some(&actual_value) => {
                last_known_value = actual_value;
                actual_value
            }
            None => last_known_value,
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

fn generate_points_storage_forecast_daily(
    db_results: Vec<DbChartStorageForecastDaily>,
    to_datetime: NaiveDate,
) -> Result<Vec<ChartPoint>> {
    if db_results.is_empty() {
        return Err(anyhow!("No data usage available"));
    }

    let first_entry = &db_results[0];
    let initial_value = first_entry.total_storage;
    let start_time = first_entry.timestamp;

    let data_map: HashMap<NaiveDate, i64> = db_results
        .into_iter()
        .skip(1)
        .map(|row| (row.timestamp, row.total_storage))
        .collect();

    let mut points = Vec::new();
    let mut current_time = start_time;
    let mut last_known_value = initial_value;

    while current_time < to_datetime {
        let next_day = current_time + Duration::days(1);

        let value = match data_map.get(&current_time) {
            Some(&actual_value) => {
                last_known_value = actual_value;
                actual_value
            }
            None => last_known_value,
        };

        points.push(ChartPoint {
            date: current_time.format("%Y-%m-%d").to_string(),
            date_to: next_day.format("%Y-%m-%d").to_string(),
            value: value.to_string(),
        });

        current_time = next_day;
    }

    Ok(points)
}
