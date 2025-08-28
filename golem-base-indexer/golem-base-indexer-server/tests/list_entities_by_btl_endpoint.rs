mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

use crate::helpers::assert_json::assert_fields_array;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_entities_by_btl_endpoint() {
    let db = helpers::init_db("test", "list_entities_by_btl_endpoint").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("fixtures/sample_data.sql")).await;

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/leaderboard/entities-by-btl?page=1&page_size=10",
    )
    .await;

    assert_eq!(response["items"].as_array().unwrap().len(), 4);
    assert_eq!(response["pagination"]["total_items"], "4".to_string());

    assert_fields_array(
        &response["items"],
        vec![
            serde_json::json!({
                "expires_at_block_number": "3006",
            }),
            serde_json::json!({
                "expires_at_block_number": "2006",
            }),
            serde_json::json!({
                "expires_at_block_number": "2006",
            }),
            serde_json::json!({
                "expires_at_block_number": "1006",
            }),
        ],
    );

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/leaderboard/entities-by-btl?page=2&page_size=2",
    )
    .await;

    assert_eq!(response["items"].as_array().unwrap().len(), 2);
    assert_eq!(response["pagination"]["page"], "2".to_string());
    assert_eq!(response["pagination"]["page_size"], "2".to_string());
}
