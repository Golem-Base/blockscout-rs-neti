use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_transactions_per_block_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_transactions_per_block_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/chart/transactions-per-block",
    )
    .await;

    // Verify the structure and metadata
    assert_eq!(response["info"]["id"], "transactionsPerBlock");
    assert_eq!(response["info"]["title"], "Transactions per Block");
    assert_eq!(response["info"]["description"], "Number of transactions for recent blocks");

    // Verify we have chart data
    let chart = response["chart"].as_array().expect("chart should be an array");
    assert!(!chart.is_empty(), "chart should not be empty");

    // The sample data has blocks 0-13, each with 2 transactions except block 0
    // We should get the last 100 blocks, but since we only have 14, we get all of them
    // Block 0 has 0 transactions, blocks 1-13 have 2 transactions each
    
    // Find block 1 to verify it has 2 transactions
    let block_1 = chart.iter().find(|item| item["date"] == "1");
    assert!(block_1.is_some(), "block 1 should be in the results");
    assert_eq!(block_1.unwrap()["value"], "2", "block 1 should have 2 transactions");

    // Find block 7 to verify it has 2 transactions
    let block_7 = chart.iter().find(|item| item["date"] == "7");
    assert!(block_7.is_some(), "block 7 should be in the results");
    assert_eq!(block_7.unwrap()["value"], "2", "block 7 should have 2 transactions");

    // Verify the chart is ordered by block number (ascending)
    let first_block_num = chart[0]["date"].as_str().unwrap().parse::<i32>().unwrap();
    let last_block_num = chart[chart.len() - 1]["date"].as_str().unwrap().parse::<i32>().unwrap();
    assert!(first_block_num < last_block_num, "blocks should be in ascending order");
}


