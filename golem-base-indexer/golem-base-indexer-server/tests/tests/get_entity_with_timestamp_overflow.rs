use crate::helpers;

use alloy_primitives::{BlockHash, TxHash};
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{types::EntityKey, well_known::SECS_PER_BLOCK, Indexer};
use golem_base_sdk::{
    entity::{EncodableGolemBaseTransaction, Update},
    Address,
};

use crate::helpers::{
    assert_json::assert_fields,
    sample::{Block, Transaction},
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_entity_with_timestamp_overflow() {
    let db = helpers::init_db("test", "get_entity_with_timestamp_overflow").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let indexer = Indexer::new(client.clone(), Default::default());

    let entity_key = EntityKey::random();
    let block_number = 1;
    let block_timestamp = chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00+00:00")
        .unwrap()
        .to_utc();

    let btl = 9000000000000000;

    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(BlockHash::random()),
            number: block_number,
            timestamp: Some(block_timestamp),
            transactions: vec![Transaction {
                hash: Some(TxHash::random()),
                sender: Address::random(),
                operations: EncodableGolemBaseTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl,
                        data: b"data".as_slice().into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ..Default::default()
            }],
        },
    )
    .await
    .unwrap();
    indexer.tick().await.unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entity/{entity_key}")).await;

    let expected_ts_sec = (block_timestamp.timestamp() as u64)
        + block_number * (SECS_PER_BLOCK as u64)
        + btl * (SECS_PER_BLOCK as u64);
    assert_fields(
        &response,
        serde_json::json!({
            "key": entity_key.to_string(),
            "expires_at_timestamp": null,
            "expires_at_timestamp_sec": expected_ts_sec.to_string(),
        }),
    )
}
