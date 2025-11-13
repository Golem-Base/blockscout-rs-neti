use crate::helpers;

use alloy_primitives::Address;
use arkiv_storage_tx::{Create, StorageTransaction};
use blockscout_service_launcher::test_server;
use chrono::{DateTime, Duration, Utc};
use golem_base_indexer_logic::{
    types::{ChartInfo, ChartPoint, TxHash},
    Indexer,
};
use helpers::{
    sample::{insert_data, Block, Transaction},
    utils::refresh_timeseries,
};
use serde_json::{json, Value};
use std::sync::Arc;

fn endpoint_for_resolution_and_to(resolution: &str, to: &str) -> String {
    format!("/api/v1/chart/storage-forecast?resolution={resolution}&to={to}",)
}

fn format_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:00").to_string()
}

fn format_date(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn storage_forecast_hourly_should_work() {
    // Setup
    let db = helpers::init_db("test", "storage_forecast_hourly_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let indexer = Indexer::new(client.clone(), Default::default());

    // Insert test entities
    let utc_current = Utc::now();
    let creates = vec![
        Create {
            payload: vec![0xff; 1024].into(),
            btl: 1800,
            ..Default::default()
        },
        Create {
            payload: vec![0xff; 1024].into(),
            btl: 3600,
            ..Default::default()
        },
        Create {
            payload: vec![0xff; 1024].into(),
            btl: 5400,
            ..Default::default()
        },
    ];
    let block = Block {
        number: 1,
        timestamp: Some(utc_current),
        transactions: vec![Transaction {
            sender: Address::random(),
            hash: Some(TxHash::random()),
            operations: StorageTransaction {
                creates,
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    insert_data(&*client, block).await.unwrap();

    // Process and refresh timeseries
    indexer.tick().await.unwrap();
    refresh_timeseries(Arc::clone(&client)).await.unwrap();

    // Check hourly resolution
    let five_hours_from_now = utc_current + Duration::hours(5);
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_to("HOUR", &format_datetime(five_hours_from_now)),
    )
    .await;

    let info = json!(ChartInfo {
        id: "golemBaseStorageForecast".to_string(),
        title: "Storage forecast".to_string(),
        description: "Chain storage forecast".to_string(),
    });

    let points = vec![
        ChartPoint {
            date: format_datetime(utc_current),
            date_to: format_datetime(utc_current + Duration::hours(1)),
            value: "3072".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(1)),
            date_to: format_datetime(utc_current + Duration::hours(2)),
            value: "3072".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(2)),
            date_to: format_datetime(utc_current + Duration::hours(3)),
            value: "2048".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(3)),
            date_to: format_datetime(utc_current + Duration::hours(4)),
            value: "1024".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(4)),
            date_to: format_datetime(utc_current + Duration::hours(5)),
            value: "0".to_string(),
        },
    ];
    let expected: Value = json!({
        "info": info,
        "chart": points,
    });

    assert_eq!(response, expected);
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn storage_forecast_daily_should_work() {
    // Setup
    let db = helpers::init_db("test", "storage_forecast_daily_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let indexer = Indexer::new(client.clone(), Default::default());

    // Insert test entities
    let utc_current = Utc::now();
    let creates = vec![
        Create {
            payload: vec![0xee; 4096].into(),
            btl: 43200,
            ..Default::default()
        },
        Create {
            payload: vec![0xee; 4096].into(),
            btl: 86400,
            ..Default::default()
        },
        Create {
            payload: vec![0xee; 4096].into(),
            btl: 1944000,
            ..Default::default()
        },
    ];
    let block = Block {
        number: 1,
        timestamp: Some(utc_current),
        transactions: vec![Transaction {
            sender: Address::random(),
            hash: Some(TxHash::random()),
            operations: StorageTransaction {
                creates,
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    insert_data(&*client, block).await.unwrap();

    // Process and refresh timeseries
    indexer.tick().await.unwrap();
    refresh_timeseries(Arc::clone(&client)).await.unwrap();

    // Check daily resolution
    let three_days_from_now = utc_current + Duration::days(3);
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_to("DAY", &format_date(three_days_from_now)),
    )
    .await;

    let info = json!(ChartInfo {
        id: "golemBaseStorageForecast".to_string(),
        title: "Storage forecast".to_string(),
        description: "Chain storage forecast".to_string(),
    });

    let points = vec![
        ChartPoint {
            date: format_date(utc_current),
            date_to: format_date(utc_current + Duration::days(1)),
            value: "12288".to_string(),
        },
        ChartPoint {
            date: format_date(utc_current + Duration::days(1)),
            date_to: format_date(utc_current + Duration::days(2)),
            value: "8192".to_string(),
        },
        ChartPoint {
            date: format_date(utc_current + Duration::days(2)),
            date_to: format_date(utc_current + Duration::days(3)),
            value: "4096".to_string(),
        },
    ];
    let expected: Value = json!({
        "info": info,
        "chart": points,
    });

    assert_eq!(response, expected);
}
