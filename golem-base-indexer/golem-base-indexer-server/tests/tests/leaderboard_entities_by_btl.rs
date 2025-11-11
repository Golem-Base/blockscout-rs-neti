use crate::helpers::{self, utils::iso_to_ts_sec};

use alloy_primitives::{Address, BlockHash, TxHash};
use arkiv_storage_tx::{StorageTransaction, Update};
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::{types::EntityKey, Indexer};
use pretty_assertions::assert_eq;

use crate::helpers::{
    assert_json::{assert_fields, assert_fields_array},
    sample::{Block, Transaction},
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_entities_by_btl_endpoint() {
    let db = helpers::init_db("test", "list_entities_by_btl_endpoint").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    let deleted_entity_key = EntityKey::random();
    let data: Bytes = b"data".as_slice().into();

    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(BlockHash::random()),
            number: 1,
            transactions: vec![Transaction {
                hash: Some(TxHash::random()),
                sender: Address::random(),
                operations: StorageTransaction {
                    updates: vec![
                        Update {
                            entity_key: EntityKey::random(),
                            btl: 100,
                            payload: data.clone(),
                            ..Default::default()
                        },
                        Update {
                            entity_key: EntityKey::random(),
                            btl: 200,
                            payload: data.clone(),
                            ..Default::default()
                        },
                        Update {
                            entity_key: EntityKey::random(),
                            btl: 300,
                            payload: data.clone(),
                            ..Default::default()
                        },
                        Update {
                            entity_key: EntityKey::random(),
                            btl: 300,
                            payload: data.clone(),
                            ..Default::default()
                        },
                        Update {
                            entity_key: deleted_entity_key,
                            btl: 10_000,
                            payload: data.clone(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    indexer.tick().await.unwrap();

    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(BlockHash::random()),
            number: 2,
            transactions: vec![Transaction {
                hash: Some(TxHash::random()),
                sender: Address::random(),
                operations: StorageTransaction {
                    deletes: vec![deleted_entity_key],
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();
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
                "expires_at_block_number": "301",
            }),
            serde_json::json!({
                "expires_at_block_number": "301",
            }),
            serde_json::json!({
                "expires_at_block_number": "201",
            }),
            serde_json::json!({
                "expires_at_block_number": "101",
            }),
        ],
    );

    assert_fields(
        &response["items"][0],
        serde_json::json!({
            "expires_at_timestamp": "2018-10-13T12:40:02+00:00",
            "expires_at_timestamp_sec": iso_to_ts_sec("2018-10-13T12:40:02+00:00"),
        }),
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
