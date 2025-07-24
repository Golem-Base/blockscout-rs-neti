mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_update_reorg_handling_works() {
    let db = helpers::init_db("test", "update_reorg_handling").await;
    let client = db.client();
    let indexer = Indexer::new(db.client(), Default::default());
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    helpers::load_data(&*client, include_str!("fixtures/reorg_update_step1.sql")).await;
    indexer.tick().await.unwrap();
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities").await;
    let expected: serde_json::Value = serde_json::json!({
        "items": [
            {
      "key": "0xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d",
      "data": "0x746869732077696c6c2062652064726f707065642062792072656f7267",
      "status": "ACTIVE",
      "created_at_tx_hash": "0xae9430e348f74284c3c91443b134b835961901862dd6ef24f32e646f346449a1",
      "last_updated_at_tx_hash": "0x1872c3e9c4c76b5802b9a7c3f7798fac5bb8110d2707e145701acf90dd6de559",
      "expires_at_block_number": "103"
            }
        ]
    });
    assert_eq!(response, expected);

    helpers::load_data(&*client, include_str!("fixtures/reorg_update_step2.sql")).await;
    indexer.tick().await.unwrap();
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities").await;
    let expected: serde_json::Value = serde_json::json!({
        "items": [
            {
      "key": "0xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d",
      "data": "0x746869732077696c6c2062652061667465722072656f7267",
      "status": "ACTIVE",
      "created_at_tx_hash": "0xae9430e348f74284c3c91443b134b835961901862dd6ef24f32e646f346449a1",
      "last_updated_at_tx_hash": "0x1932fed6f6464781ee6e928cf6b43a49d0dbb1024c9ac6c91ef480852c794cb9",
      "expires_at_block_number": "103"
            }
        ]
    });
    assert_eq!(response, expected);
}
