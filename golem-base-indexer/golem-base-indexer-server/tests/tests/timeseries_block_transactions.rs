use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use serde_json::Value;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_block_transactions_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_block_transactions_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/chart/block-transactions",
    )
    .await;

    // Verify the structure and metadata
    assert_eq!(response["info"]["id"], "blockTransactions");
    assert_eq!(response["info"]["title"], "Block Transactions");
    assert_eq!(response["info"]["description"], "Number of transactions for recent blocks");

    // Verify we have chart data
    let chart = response["chart"].as_array().expect("chart should be an array");
    assert!(!chart.is_empty(), "chart should not be empty");

    // The sample data has blocks 0-13, each with 2 transactions except block 0
    // Verify specific blocks have correct transaction counts
    let block_1 = chart.iter().find(|item| item["block_number"] == "1");
    assert_eq!(block_1.unwrap()["tx_count"], "2");

    let block_7 = chart.iter().find(|item| item["block_number"] == "7");
    assert_eq!(block_7.unwrap()["tx_count"], "2");

    // Verify chart is ordered by block number (ascending)
    let first_block = chart[0]["block_number"].as_str().unwrap().parse::<u64>().unwrap();
    let last_block = chart.last().unwrap()["block_number"].as_str().unwrap().parse::<u64>().unwrap();
    assert!(first_block < last_block);
}
