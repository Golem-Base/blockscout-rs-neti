mod helpers;

use alloy_primitives::{BlockHash, TxHash};
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{types::EntityKey, Indexer};
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

    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(BlockHash::random()),
            number: 1,
            transactions: vec![Transaction {
                hash: Some(TxHash::random()),
                sender: Address::random(),
                operations: EncodableGolemBaseTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl: 9000000000000000,
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

    assert_fields(
        &response,
        serde_json::json!({
            "key": entity_key.to_string(),
            "expires_at_timestamp": null,
        }),
    )
}
