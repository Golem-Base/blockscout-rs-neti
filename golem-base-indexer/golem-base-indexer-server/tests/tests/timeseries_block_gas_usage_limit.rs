use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{types::ChartInfo, Indexer};
use helpers::utils::refresh_timeseries;
use serde_json::{json, Value};
use std::sync::Arc;

fn chart_info() -> Value {
    json!(ChartInfo {
        id: "golemBaseBlockGasUsageLimit".to_string(),
        title: "Gas usage over time".to_string(),
        description: "Per block gas used and gas limit".to_string(),
    })
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_block_gas_usage_limit_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_block_gas_usage_limit_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();
    refresh_timeseries(Arc::clone(&client)).await.unwrap();

    // Hourly
    let response: Value =
        test_server::send_get_request(&base, "/api/v1/chart/block-gas-usage-limit").await;

    let expected: Value = json!({
        "info": chart_info(),
        "chart": [
            {
                "block_number": "0",
                "gas_used": "0",
                "gas_limit": "11500000",
                "gas_usage_percentage": "0"
            },
            {
                "block_number": "1",
                "gas_used": "1021000",
                "gas_limit": "11511229",
                "gas_usage_percentage": "8.87"
            },
            {
                "block_number": "2",
                "gas_used": "1022480",
                "gas_limit": "11522469",
                "gas_usage_percentage": "8.87"
            },
            {
                "block_number": "3",
                "gas_used": "1022480",
                "gas_limit": "11533720",
                "gas_usage_percentage": "8.87"
            },
            {
                "block_number": "4",
                "gas_used": "1023160",
                "gas_limit": "11544982",
                "gas_usage_percentage": "8.86"
            },
            {
                "block_number": "5",
                "gas_used": "1022520",
                "gas_limit": "11556255",
                "gas_usage_percentage": "8.85"
            },
            {
                "block_number": "6",
                "gas_used": "1033210",
                "gas_limit": "11567539",
                "gas_usage_percentage": "8.93"
            },
            {
                "block_number": "7",
                "gas_used": "1021680",
                "gas_limit": "11578834",
                "gas_usage_percentage": "8.82"
            },
            {
                "block_number": "8",
                "gas_used": "1022480",
                "gas_limit": "11590140",
                "gas_usage_percentage": "8.82"
            },
            {
                "block_number": "9",
                "gas_used": "1022480",
                "gas_limit": "11601457",
                "gas_usage_percentage": "8.81"
            },
            {
                "block_number": "10",
                "gas_used": "1023160",
                "gas_limit": "11612785",
                "gas_usage_percentage": "8.81"
            },
            {
                "block_number": "11",
                "gas_used": "1022520",
                "gas_limit": "11624124",
                "gas_usage_percentage": "8.8"
            },
            {
                "block_number": "12",
                "gas_used": "1033180",
                "gas_limit": "11635474",
                "gas_usage_percentage": "8.88"
            },
            {
                "block_number": "13",
                "gas_used": "1021680",
                "gas_limit": "11646835",
                "gas_usage_percentage": "8.77"
            }
        ],
    });

    assert_eq!(response, expected);
}
