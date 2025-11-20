use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_extend_delete_reorg_handling_works() {
    let db = helpers::init_db("test", "extend_delete_reorg_handling").await;
    let client = db.client();
    let indexer = Indexer::new(db.client(), Default::default());
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    helpers::load_data(
        &*client,
        include_str!("../fixtures/reorg_extend_delete_step1.sql"),
    )
    .await;
    indexer.tick().await.unwrap();
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=ACTIVE").await;
    let expected: serde_json::Value = serde_json::json!([
            {
      "key": "0xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d",
      "content_type": "text/plain",
      "data": "0x74657374",
      "status": "ACTIVE",
      "created_at_tx_hash": "0xae9430e348f74284c3c91443b134b835961901862dd6ef24f32e646f346449a1",
      "last_updated_at_tx_hash": "0x488a9a57364c22e819a6af41fca5db893a2dee1f678d859ec6bd5079aae71453",
      "expires_at_block_number": "225"
            }
        ]
    );
    assert_eq!(
        response.as_object().unwrap().get("items").unwrap(),
        &expected
    );

    helpers::load_data(
        &*client,
        include_str!("../fixtures/reorg_extend_delete_step2.sql"),
    )
    .await;
    indexer.tick().await.unwrap();
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=DELETED").await;
    let expected: serde_json::Value = serde_json::json!(
       [
            {
      "key": "0xfa9a092a3b2b2ac68357798634030f86e018cfacea23783429b3101caaebe95d",
      "content_type": null,
      "data": null,
      "status": "DELETED",
      "created_at_tx_hash": "0xae9430e348f74284c3c91443b134b835961901862dd6ef24f32e646f346449a1",
      "last_updated_at_tx_hash": "0xdac82fe3f61d518aefddb840e859699f50ab0713ce1ab0c0123ebbcee05fb325",
      "expires_at_block_number": "3"
            }
        ]
    );
    assert_eq!(
        response.as_object().unwrap().get("items").unwrap(),
        &expected
    );
}
