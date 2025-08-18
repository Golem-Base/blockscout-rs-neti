mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_address_stats_endpoint_works() {
    let db = helpers::init_db("test", "get_address_stats_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("fixtures/sample_data.sql")).await;

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
        "active_entities": "4",
        "created_entities": "6",
        "failed_transactions": "1",
        "operations_counts": {
            "create_count": "6",
            "delete_count": "1",
            "extend_count": "1",
            "update_count": "2",
        },
        "size_of_active_entities": "88",
        "total_transactions": "6",
    });
    assert_eq!(response, expected);
}
