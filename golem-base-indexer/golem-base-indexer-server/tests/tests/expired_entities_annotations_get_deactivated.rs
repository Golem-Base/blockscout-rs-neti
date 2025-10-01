use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_expired_entities_annotations_get_deactivated() {
    let db = helpers::init_db("test", "expired_entities_annotations_get_deactivated").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0x4455e54a419991f302a495eea0be0fe37c121aae1f9c0048e7a1ec45900cd0cf",
    )
    .await;
    let expected: serde_json::Value = serde_json::json!({
      "key": "0x4455e54a419991f302a495eea0be0fe37c121aae1f9c0048e7a1ec45900cd0cf",
      "data": null,
      "data_size": null,
      "status": "EXPIRED",
      "string_annotations": [],
      "numeric_annotations": [],
      "created_at_tx_hash": "0x61080cf78f68f5813d841300d7ed257ab1a735271606d4d435e42283c4be8137",
      "created_at_operation_index": "0",
      "created_at_block_number": "6",
      "created_at_timestamp": "2025-07-22T11:31:34+00:00",
      "updated_at_tx_hash": "0x1b7b3d0ac4b9636a34c72e6ab55a115a2abaa74dfcbf492d5b0b58fe13a04a96",
      "updated_at_operation_index": "0",
      "updated_at_block_number": "7",
      "updated_at_timestamp": "2025-07-22T11:31:35+00:00",
      "expires_at_block_number": "7",
      "expires_at_timestamp": "2025-07-22T14:17:39+00:00",
      "expires_at_timestamp_sec": "1753193871",
      "owner": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
      "gas_used": "0",
      "fees_paid": "0"
    });
    assert_eq!(response, expected);
}
