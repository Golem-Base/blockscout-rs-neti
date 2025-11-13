use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{
    types::{ChartInfo, ChartPoint},
    Indexer,
};
use helpers::utils::refresh_timeseries;
use serde_json::{json, Value};
use std::sync::Arc;

fn chart_info() -> Value {
    json!(ChartInfo {
        id: "golemBaseEntityCount".to_string(),
        title: "Entities over time".to_string(),
        description: "Total number of entities on the chain over time".to_string(),
    })
}

fn endpoint_for_resolution_and_dates(resolution: &str, from: &str, to: &str) -> String {
    format!(
        "/api/v1/chart/entity-count?resolution={resolution}&from={from}&to={to}"
    )
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_entity_count_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_entity_count_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();
    refresh_timeseries(Arc::clone(&client)).await.unwrap();

    // Hourly
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_dates("HOUR", "2025-07-22%2011:00", "2025-07-22%2012:00"),
    )
    .await;

    let points = vec![ChartPoint {
        date: "2025-07-22 11:00".to_string(),
        date_to: "2025-07-22 12:00".to_string(),
        value: "4".to_string(),
    }];

    let expected: Value = json!({
        "info": chart_info(),
        "chart": points,
    });

    assert_eq!(response, expected);

    // Daily
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_dates("DAY", "2025-07-22", "2025-07-23"),
    )
    .await;

    let points = vec![ChartPoint {
        date: "2025-07-22".to_string(),
        date_to: "2025-07-23".to_string(),
        value: "4".to_string(),
    }];

    let expected: Value = json!({
        "info": chart_info(),
        "chart": points,
    });

    assert_eq!(response, expected);
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_entity_count_should_work_without_data_indexed() {
    // Setup
    let db = helpers::init_db(
        "test",
        "chart_entity_count_should_work_without_data_indexed",
    )
    .await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();
    refresh_timeseries(Arc::clone(&client)).await.unwrap();

    // Daily
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_dates("DAY", "2025-10-01", "2025-10-03"),
    )
    .await;

    let points = vec![
        ChartPoint {
            date: "2025-10-01".to_string(),
            date_to: "2025-10-02".to_string(),
            value: "0".to_string(),
        },
        ChartPoint {
            date: "2025-10-02".to_string(),
            date_to: "2025-10-03".to_string(),
            value: "0".to_string(),
        },
    ];

    let expected: Value = json!({
        "info": chart_info(),
        "chart": points,
    });

    assert_eq!(response, expected);

    // Hourly
    let response: Value = test_server::send_get_request(
        &base,
        &endpoint_for_resolution_and_dates("HOUR", "2025-10-01%2010:00", "2025-10-01%2012:00"),
    )
    .await;

    let points = vec![
        ChartPoint {
            date: "2025-10-01 10:00".to_string(),
            date_to: "2025-10-01 11:00".to_string(),
            value: "0".to_string(),
        },
        ChartPoint {
            date: "2025-10-01 11:00".to_string(),
            date_to: "2025-10-01 12:00".to_string(),
            value: "0".to_string(),
        },
    ];

    let expected: Value = json!({
        "info": chart_info(),
        "chart": points,
    });

    assert_eq!(response, expected);
}
