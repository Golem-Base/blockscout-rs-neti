mod helpers;

use alloy_primitives::{BlockHash, TxHash};
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::{types::EntityKey, Indexer};
use golem_base_sdk::{
    entity::{EncodableGolemBaseTransaction, Update},
    Address,
};
use pretty_assertions::assert_eq;

use crate::helpers::{
    assert_json::assert_fields_array,
    sample::{Block, Transaction},
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_largest_entities_endpoint() {
    let db = helpers::init_db("test", "list_largest_entities_endpoint").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    fn gen_bytes(size: usize) -> Bytes {
        let vec = vec![0u8; size];
        Bytes::from(vec)
    }

    fn gen_update(data: Bytes) -> Update {
        Update {
            entity_key: EntityKey::random(),
            btl: 100,
            data,
            ..Default::default()
        }
    }

    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(BlockHash::random()),
            number: 1,
            transactions: vec![Transaction {
                hash: Some(TxHash::random()),
                sender: Address::random(),
                operations: EncodableGolemBaseTransaction {
                    updates: vec![
                        gen_update(gen_bytes(5)),
                        gen_update(gen_bytes(10)),
                        gen_update(gen_bytes(20)),
                        gen_update(gen_bytes(30)),
                        gen_update(gen_bytes(100)),
                    ],
                    ..Default::default()
                },
            }],
        },
    )
    .await
    .unwrap();
    indexer.tick().await.unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/leaderboard/largest-entities?page=1&page_size=10",
    )
    .await;

    assert_eq!(response["items"].as_array().unwrap().len(), 5);
    assert_eq!(response["pagination"]["total_items"], "5".to_string());

    assert_fields_array(
        &response["items"],
        vec![
            serde_json::json!({
                "data_size": "100",
            }),
            serde_json::json!({
                "data_size": "30",
            }),
            serde_json::json!({
                "data_size": "20",
            }),
            serde_json::json!({
                "data_size": "10",
            }),
            serde_json::json!({
                "data_size": "5",
            }),
        ],
    );

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/leaderboard/largest-entities?page=2&page_size=2",
    )
    .await;

    assert_eq!(response["items"].as_array().unwrap().len(), 2);
    assert_eq!(response["pagination"]["page"], "2".to_string());
    assert_eq!(response["pagination"]["page_size"], "2".to_string());
    assert_eq!(response["pagination"]["total_items"], "5".to_string());
}
