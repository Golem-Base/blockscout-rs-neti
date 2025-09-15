mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_data_usage_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_data_usage_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    client
        .execute(Statement::from_string(
            DbBackend::Postgres,
            "REFRESH MATERIALIZED VIEW golem_base_timeseries",
        ))
        .await
        .expect("Refresh of MATERIALIZED VIEW failed!");

    // Hourly
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/chart/data-usage?resolution=HOUR&from=2025-07-22%2011:00&to=2025-07-22%2012:00",
    )
    .await;

    let expected: Value = json!({
        "info": {
            "id": "golemBaseDataUsage",
            "title": "Data over time",
            "description": "Data storage over time",
        },
        "chart": [
            {
                "date": "2025-07-22 11:00",
                "date_to": "2025-07-22 12:00",
                "value": "146",
            }
        ]
    });

    assert_eq!(response, expected);

    // Daily
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/chart/data-usage?resolution=DAY&from=2025-07-22&to=2025-07-23",
    )
    .await;

    let expected: Value = json!({
        "info": {
            "id": "golemBaseDataUsage",
            "title": "Data over time",
            "description": "Data storage over time",
        },
        "chart": [
            {
                "date": "2025-07-22",
                "date_to": "2025-07-23",
                "value": "146",
            }
        ]
    });

    assert_eq!(response, expected);
}
