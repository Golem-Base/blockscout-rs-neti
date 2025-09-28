use crate::helpers;

use blockscout_service_launcher::test_server;
use chrono::{DateTime, Duration, Utc};
use golem_base_indexer_logic::{
    types::{ChartInfo, ChartPoint, TxHash},
    Indexer,
};
use golem_base_sdk::entity::{Create, EncodableGolemBaseTransaction};
use golem_base_sdk::Address;
use helpers::sample::{insert_data, Block, Transaction};
use helpers::utils::refresh_timeseries;
use serde_json::{json, Value};
use std::sync::Arc;

fn endpoint_for_resolution_and_to(resolution: &str, to: &str) -> String {
    format!(
        "/api/v1/chart/storage-forecast?resolution={}&to={}",
        resolution, to,
    )
}

fn format_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:00").to_string()
}

fn format_date(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn storage_forecast_should_work() {
    // Setup
    let db = helpers::init_db("test", "storage_forecast_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let indexer = Indexer::new(client.clone(), Default::default());

    // Insert test entities
    let utc_current = Utc::now();
    let creates = vec![
        Create::new(vec![0xff; 1024], 1800), // 1k, will expire in one hour
        Create::new(vec![0xff; 2048], 3600), // 2k, will expire in two hours
        Create::new(vec![0xff; 4096], 5400), // 4k, will expire in three hours
        Create::new(vec![0xee; 16384], 43200), // 16k, will expire in one day
        Create::new(vec![0xee; 32768], 86400), // 32k, will expire in two days
        Create::new(vec![0xdd; 65535], 1944000), // 64k, will expire in 45 days
    ];
    let block = Block {
        number: 1,
        timestamp: Some(utc_current),
        transactions: vec![Transaction {
            sender: Address::random(),
            hash: Some(TxHash::random()),
            operations: EncodableGolemBaseTransaction {
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
            value: "121855".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(1)),
            date_to: format_datetime(utc_current + Duration::hours(2)),
            value: "121855".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(2)),
            date_to: format_datetime(utc_current + Duration::hours(3)),
            value: "120831".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(3)),
            date_to: format_datetime(utc_current + Duration::hours(4)),
            value: "118783".to_string(),
        },
        ChartPoint {
            date: format_datetime(utc_current + Duration::hours(4)),
            date_to: format_datetime(utc_current + Duration::hours(5)),
            value: "114687".to_string(),
        },
    ];
    let expected: Value = json!({
        "info": info,
        "chart": points,
    });

    assert_eq!(response, expected);

    // Check daily resolution
    let three_days_from_now = utc_current + Duration::days(3);
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_to("DAY", &format_date(three_days_from_now)),
    )
    .await;

    let points = vec![
        ChartPoint {
            date: format_date(utc_current),
            date_to: format_date(utc_current + Duration::days(1)),
            value: "114687".to_string(),
        },
        ChartPoint {
            date: format_date(utc_current + Duration::days(1)),
            date_to: format_date(utc_current + Duration::days(2)),
            value: "114687".to_string(),
        },
        ChartPoint {
            date: format_date(utc_current + Duration::days(2)),
            date_to: format_date(utc_current + Duration::days(3)),
            value: "71487".to_string(),
        },
    ];
    let expected: Value = json!({
        "info": info,
        "chart": points,
    });

    assert_eq!(response, expected);
}
