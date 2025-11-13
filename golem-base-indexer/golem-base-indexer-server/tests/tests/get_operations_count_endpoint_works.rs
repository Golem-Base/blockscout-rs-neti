use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_operations_count_endpoint_works() {
    let db = helpers::init_db("test", "get_operations_count_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let test_cases = vec![
        (
            "/api/v1/operations/count",
            serde_json::json!({
                "changeowner_count": "1",
                "create_count": "6",
                "delete_count": "2",
                "extend_count": "1",
                "update_count": "2"
            }),
        ),
        (
            "/api/v1/operations/count?sender=0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
            serde_json::json!({
                "changeowner_count": "1",
                "create_count": "6",
                "delete_count": "1",
                "extend_count": "1",
                "update_count": "2"
            }),
        ),
        (
            "/api/v1/operations/count?transaction_hash=0x1b7b3d0ac4b9636a34c72e6ab55a115a2abaa74dfcbf492d5b0b58fe13a04a96",
            serde_json::json!({
                "changeowner_count": "0",
                "create_count": "0",
                "delete_count": "1",
                "extend_count": "0",
                "update_count": "0"
            }),
        ),
        (
            "/api/v1/operations/count?block_number_or_hash=0xe6f06416be4859119817b2f1d3d0f8c8fa2729804c4795452c5810e3c54b67d2",
            serde_json::json!({
                "changeowner_count": "0",
                "create_count": "2",
                "delete_count": "1",
                "extend_count": "1",
                "update_count": "2"
            }),
        ),
        (
            "/api/v1/operations/count?block_number_or_hash=6",
            serde_json::json!({
                "changeowner_count": "0",
                "create_count": "2",
                "delete_count": "1",
                "extend_count": "1",
                "update_count": "2"
            }),
        ),
        (
            "/api/v1/operations/count?entity_key=0x901799b2f558af736716b4dc4427424e1d07d420cbb8bc53ba15489c5727e84b",
            serde_json::json!({
                "changeowner_count": "0",
                "create_count": "1",
                "delete_count": "0",
                "extend_count": "1",
                "update_count": "0"
            }),
        ),
        (
            "/api/v1/operations/count?transaction_hash=0x0000000000000000000000000000000000000000000000000000000000000000",
            serde_json::json!({
                "changeowner_count": "0",
                "create_count": "0",
                "delete_count": "0",
                "extend_count": "0",
                "update_count": "0"
            }),
        ),
    ];

    for (url, expected) in test_cases {
        let response: serde_json::Value = test_server::send_get_request(&base, url).await;
        assert_eq!(response, expected, "Test failed for URL: {}", url);
    }
}
