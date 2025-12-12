use crate::helpers;

use alloy_primitives::Address;
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_address_stats_endpoint_works() {
    let db = helpers::init_db("test", "get_address_stats_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/address/0xd29bb1a1a0f6d2783306a8618b3a5b58cb313152/stats",
    )
    .await;

    let expected: serde_json::Value = serde_json::json!({
        "created_entities": "6",
        "owned_entities": "5",
        "active_entities": "3",
        "size_of_active_entities": "76",
        "failed_transactions": "1",
        "operations_counts": {
            "changeowner_count": "1",
            "create_count": "6",
            "delete_count": "1",
            "extend_count": "1",
            "update_count": "2",
        },
        "total_transactions": "7",
        "first_seen_timestamp": "2025-07-22T11:31:28+00:00",
        "last_seen_timestamp": "2025-07-22T11:31:35+00:00",
        "first_seen_block": "1",
        "last_seen_block": "7",
    });
    assert_eq!(response, expected);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/address/0xee65114387Fe5D2C08F7a8E555aC83C931E3e1f9/stats",
    )
    .await;

    let expected: serde_json::Value = serde_json::json!({
        "created_entities": "0",
        "owned_entities": "0",
        "active_entities": "0",
        "size_of_active_entities": "0",
        "failed_transactions": "0",
        "operations_counts": {
            "changeowner_count": "0",
            "create_count": "0",
            "delete_count": "0",
            "extend_count": "0",
            "update_count": "0",
        },
        "total_transactions": "0",
        "first_seen_timestamp": "2025-07-22T11:31:28+00:00",
        "last_seen_timestamp": "2025-07-22T11:31:28+00:00",
        "first_seen_block": "1",
        "last_seen_block": "1",
    });
    assert_eq!(response, expected);

    let random_address = Address::random();
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/address/{random_address}/stats"))
            .await;
    let expected: serde_json::Value = serde_json::json!({
        "created_entities": "0",
        "owned_entities": "0",
        "active_entities": "0",
        "size_of_active_entities": "0",
        "failed_transactions": "0",
        "operations_counts": {
            "changeowner_count": "0",
            "create_count": "0",
            "delete_count": "0",
            "extend_count": "0",
            "update_count": "0",
        },
        "total_transactions": "0",
        "first_seen_timestamp": null,
        "last_seen_timestamp": null,
        "first_seen_block": null,
        "last_seen_block": null,
    });
    assert_eq!(response, expected);
}
