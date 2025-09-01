mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn block_stats_should_work() {
    // Setup
    let db = helpers::init_db("test", "block_stats_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    // Test block 1
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/1/stats").await;
    let counts: Value = json!({
        "create_count": "0",
        "update_count": "0",
        "expire_count": "0",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "0",
        "total_bytes": "0",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 2
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/2/stats").await;
    let counts: Value = json!({
        "create_count": "1",
        "update_count": "0",
        "expire_count": "0",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "25",
        "total_bytes": "25",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 3
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/3/stats").await;
    let counts: Value = json!({
        "create_count": "1",
        "update_count": "0",
        "expire_count": "0",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "25",
        "total_bytes": "50",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 4
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/4/stats").await;
    let counts: Value = json!({
        "create_count": "1",
        "update_count": "0",
        "expire_count": "0",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "42",
        "total_bytes": "92",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 5
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/5/stats").await;
    let counts: Value = json!({
        "create_count": "1",
        "update_count": "0",
        "expire_count": "0",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "26",
        "total_bytes": "118",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 6
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/6/stats").await;
    let counts: Value = json!({
        "create_count": "2",
        "update_count": "2",
        "expire_count": "0",
        "delete_count": "1",
        "extend_count": "1",
    });
    let storage: Value = json!({
        "block_bytes": "121",
        "total_bytes": "121",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 7
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/7/stats").await;
    let counts: Value = json!({
        "create_count": "0",
        "update_count": "0",
        "expire_count": "1",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "0",
        "total_bytes": "88",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);

    // Test block 8 - should return zero counters for a block with no operations
    let response: Value = test_server::send_get_request(&base, "/api/v1/block/8/stats").await;
    let counts: Value = json!({
        "create_count": "0",
        "update_count": "0",
        "expire_count": "0",
        "delete_count": "0",
        "extend_count": "0",
    });
    let storage: Value = json!({
        "block_bytes": "0",
        "total_bytes": "88",
    });
    let expected = json!({
        "counts": counts,
        "storage": storage,
    });
    assert_eq!(response, expected);
}
