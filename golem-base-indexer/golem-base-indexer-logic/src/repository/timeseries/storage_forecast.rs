use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use sea_query::{Iden, PostgresQueryBuilder, Query, SelectStatement};
use std::collections::HashMap;
use tracing::instrument;

use crate::types::{ChartInfo, ChartPoint};

use super::common::*;

#[derive(Debug, FromQueryResult)]
struct DbChartStorageForecastHourly {
    pub timestamp: NaiveDateTime,
    pub total_storage: i64,
}

#[derive(Iden)]
pub enum GolemBaseTimeseriesStorageForecast {
    Table,
    Timestamp,
    TotalStorage,
}

#[instrument(skip(db))]
pub async fn timeseries_storage_forecast<T: ConnectionTrait>(
    db: &T,
    to: &str,
    resolution: ChartResolution,
) -> Result<(Vec<ChartPoint>, ChartInfo)> {
    let chart = match resolution {
        ChartResolution::Day | ChartResolution::Week | ChartResolution::Month => {
            let (_, to_date) = parse_date_range(None, Some(to.to_string()))?;
            let query = build_query_storage_forecast_hourly(
                to_date.unwrap().and_hms_opt(23, 59, 59).unwrap(),
            );
            let results = DbChartStorageForecastHourly::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                query.to_string(PostgresQueryBuilder),
            ))
            .all(db)
            .await
            .context("Failed to get storage forecast timeseries")?;

            let interval = match resolution {
                ChartResolution::Week => Duration::days(7),
                ChartResolution::Month => Duration::days(30),
                _ => Duration::days(1),
            };

            generate_points_storage_forecast(results, to_date.unwrap(), interval)?
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

fn generate_points_storage_forecast(
    db_results: Vec<DbChartStorageForecastHourly>,
    to_date: NaiveDate,
    interval: Duration,
) -> Result<Vec<ChartPoint>> {
    if db_results.is_empty() {
        return Err(anyhow!("No data usage available"));
    }

    // Find the initial value and start date
    let first_entry = &db_results[0];
    let initial_value = first_entry.total_storage;
    let start_date = first_entry.timestamp.date();

    let mut data_map: HashMap<NaiveDate, i64> = HashMap::new();

    for row in &db_results {
        let row_date = row.timestamp.date();
        data_map.insert(row_date, row.total_storage);
    }

    let mut points = Vec::new();
    let mut current_date = start_date;
    let mut last_known_value = initial_value;

    let date_format = "%Y-%m-%d";

    while current_date < to_date {
        let next_date = current_date + interval;

        // Find the latest value within the current interval
        let mut value_for_period = last_known_value;

        // Look for the latest data point within this period
        let mut latest_date_in_period = None;
        for date in data_map.keys() {
            if *date >= current_date && *date < next_date {
                match latest_date_in_period {
                    None => latest_date_in_period = Some(*date),
                    Some(existing_date) => {
                        if *date > existing_date {
                            latest_date_in_period = Some(*date);
                        }
                    }
                }
            }
        }

        // Use the value from the latest date in this period, if any
        if let Some(latest_date) = latest_date_in_period {
            if let Some(&latest_value) = data_map.get(&latest_date) {
                value_for_period = latest_value;
                last_known_value = latest_value;
            }
        }

        points.push(ChartPoint {
            date: current_date.format(date_format).to_string(),
            date_to: next_date.format(date_format).to_string(),
            value: value_for_period.to_string(),
        });

        current_date = next_date;
    }

    Ok(points)
}
