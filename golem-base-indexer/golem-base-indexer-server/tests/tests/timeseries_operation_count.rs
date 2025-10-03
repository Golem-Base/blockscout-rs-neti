use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use helpers::utils::refresh_timeseries;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_operation_count_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_operation_count_should_work").await;
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
        "/api/v1/chart/operation-count?resolution=HOUR&from=2025-07-22%2011:00&to=2025-07-22%2012:00",
    )
    .await;

    let expected: Value = json!({
        "info": {
            "id": "golemBaseOperationCount",
            "title": "Operations over time",
            "description": "Operations over time",
        },
        "chart": [
            {
                "date": "2025-07-22 11:00",
                "date_to": "2025-07-22 12:00",
                "value": "11",
            }
        ]
    });

    assert_eq!(response, expected);

    // Daily
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/chart/operation-count?resolution=DAY&from=2025-07-22&to=2025-07-23",
    )
    .await;

    let expected: Value = json!({
        "info": {
            "id": "golemBaseOperationCount",
            "title": "Operations over time",
            "description": "Operations over time",
        },
        "chart": [
            {
                "date": "2025-07-22",
                "date_to": "2025-07-23",
                "value": "11",
            }
        ]
    });

    assert_eq!(response, expected);
}
