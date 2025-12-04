use crate::helpers::{self, utils::iso_to_ts_sec};

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_entity_endpoint_works() {
    let db = helpers::init_db("test", "get_entity_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0xc9e98b00f26835a3a6de7d268e5f64dba739e3730e52b84019f1bb4e73ed2296",
    )
    .await;
    let expected: serde_json::Value = serde_json::json!({
      "key": "0xc9e98b00f26835a3a6de7d268e5f64dba739e3730e52b84019f1bb4e73ed2296",
      "content_type": "text/plain",
      "data": "0x757064617465642064617461207769746820616e6e6f746174696f6e73",
      "data_size": "29",
      "status": "ACTIVE",
      "string_annotations": [
        {
            "key": "key",
            "value": "updated",
            "related_entities": "1",
        }
      ],
      "numeric_annotations": [
        {
            "key": "updated",
            "value": "1",
            "related_entities": "1",
        }
      ],
      "created_at_tx_hash": "0x385ae37be55f8e28678afeaccb594ad0a25e013746c5250df31df5d1a1df5806",
      "creator": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
      "created_at_operation_index": "0",
      "created_at_block_number": "4",
      "created_at_timestamp": "2025-07-22T11:31:32+00:00",
      "updated_at_tx_hash": "0x61080cf78f68f5813d841300d7ed257ab1a735271606d4d435e42283c4be8137",
      "updated_at_operation_index": "4",
      "updated_at_block_number": "6",
      "updated_at_timestamp": "2025-07-22T11:31:34+00:00",
      "expires_at_block_number": "2006",
      "expires_at_timestamp": "2025-07-22T15:24:17+00:00",
      "expires_at_timestamp_sec": iso_to_ts_sec("2025-07-22T15:24:17+00:00"),
      "owner": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
      "cost": "0",
      "fees_paid": "0"
    });
    assert_eq!(response, expected);
}
