use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use serde_json::Value;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_block_operations_should_work() {
    // Setup
    let db = helpers::init_db("test", "chart_block_operations_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    let response: Value =
        test_server::send_get_request(&base, "/api/v1/chart/block-operations").await;

    // Verify the structure and metadata
    assert_eq!(response["info"]["id"], "blockOperations");
    assert_eq!(response["info"]["title"], "Block Operations");
    assert_eq!(
        response["info"]["description"],
        "Number of operations per block by type"
    );

    // Verify we have chart data
    let chart = response["chart"]
        .as_array()
        .expect("chart should be an array");
    assert!(!chart.is_empty(), "chart should not be empty");

    // Verify chart structure - each entry should have the required fields
    for entry in chart.iter() {
        assert!(entry.get("block_number").is_some());
        assert!(entry.get("create_count").is_some());
        assert!(entry.get("update_count").is_some());
        assert!(entry.get("delete_count").is_some());
        assert!(entry.get("extend_count").is_some());
        assert!(entry.get("changeowner_count").is_some());
    }

    // Verify chart is ordered by block number (ascending)
    let first_block = chart[0]["block_number"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let last_block = chart.last().unwrap()["block_number"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(
        first_block < last_block,
        "chart should be in ascending order"
    );

    // Verify that operations exist across the blocks
    let total_creates: u64 = chart
        .iter()
        .map(|item| {
            item["create_count"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .sum();
    let total_updates: u64 = chart
        .iter()
        .map(|item| {
            item["update_count"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .sum();
    let total_deletes: u64 = chart
        .iter()
        .map(|item| {
            item["delete_count"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .sum();
    let total_extends: u64 = chart
        .iter()
        .map(|item| {
            item["extend_count"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .sum();
    let total_changeowners: u64 = chart
        .iter()
        .map(|item| {
            item["changeowner_count"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .sum();

    // Based on sample_data.sql and the indexer logic, we expect operations
    assert!(total_creates > 0, "should have create operations");
    assert!(total_updates > 0, "should have update operations");
    assert!(total_deletes > 0, "should have delete operations");
    assert!(total_extends > 0, "should have extend operations");
    assert!(total_changeowners > 0, "should have changeowner operations");
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn chart_block_operations_respects_limit() {
    // Setup
    let db = helpers::init_db("test", "chart_block_operations_respects_limit").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    // Test with a limit
    let response: Value =
        test_server::send_get_request(&base, "/api/v1/chart/block-operations?limit=5").await;

    let chart = response["chart"]
        .as_array()
        .expect("chart should be an array");

    // Should respect the limit
    assert!(
        chart.len() <= 5,
        "chart should have at most 5 entries, got {}",
        chart.len()
    );

    // Test with default (should be more than 5 blocks with operations)
    let response_default: Value =
        test_server::send_get_request(&base, "/api/v1/chart/block-operations").await;

    let chart_default = response_default["chart"]
        .as_array()
        .expect("chart should be an array");

    // Default limit should return more results (if available)
    // Sample data has operations across multiple blocks
    assert!(
        chart_default.len() >= chart.len(),
        "default limit should return at least as many results as limit=5"
    );
}
