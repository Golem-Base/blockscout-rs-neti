mod helpers;

use alloy_primitives::{Address, TxHash};
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::{
    types::{BlockHash, EntityKey},
    Indexer,
};
use golem_base_sdk::entity::{EncodableGolemBaseTransaction, Extend, Update};

use crate::helpers::{
    assert_json::assert_fields,
    sample::{Block, Transaction},
    utils::bytes_to_hex,
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_operation_endpoint() {
    let db = helpers::init_db("test", "get_operation_endpoint").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let entity_key = EntityKey::random();
    let sender = Address::random();

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    let data: Bytes = b"data".as_slice().into();
    let data_hex = bytes_to_hex(&data);
    let extend_by = 123;

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                sender,
                operations: EncodableGolemBaseTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl: 100,
                        data: data.clone(),
                        ..Default::default()
                    }],
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

    let block_hash = BlockHash::random();
    let tx_hash = TxHash::random();
    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(block_hash),
            number: 2,
            transactions: vec![Transaction {
                hash: Some(tx_hash),
                sender,
                operations: EncodableGolemBaseTransaction {
                    extensions: vec![
                        Extend {
                            entity_key,
                            number_of_blocks: extend_by,
                        },
                        Extend {
                            entity_key,
                            number_of_blocks: extend_by,
                        },
                    ],
                    ..Default::default()
                },
            }],
        },
    )
    .await
    .unwrap();

    indexer.tick().await.unwrap();

    let op_index = 0;
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{tx_hash}/{op_index}"))
            .await;

    assert_fields(
        &response,
        serde_json::json!({
            "entity_key": entity_key.to_string(),
            "block_number": "2",
            "block_hash": block_hash.to_string(),
            "transaction_hash": tx_hash.to_string(),
            "data": data_hex,
            "prev_data": data_hex,
            "status": "ACTIVE",
            "operation": "EXTEND",
            "expires_at_block_number": format!("{}", 101 + extend_by),
            "prev_expires_at_block_number": "101",
        }),
    );

    let op_index = 1;
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{tx_hash}/{op_index}"))
            .await;

    assert_fields(
        &response,
        serde_json::json!({
            "entity_key": entity_key.to_string(),
            "block_number": "2",
            "block_hash": block_hash.to_string(),
            "transaction_hash": tx_hash.to_string(),
            "data": data_hex,
            "prev_data": data_hex,
            "status": "ACTIVE",
            "operation": "EXTEND",
            "expires_at_block_number": format!("{}", 101 + extend_by + extend_by),
            "prev_expires_at_block_number": format!("{}", 101 + extend_by),
        }),
    );

    let update_data: Bytes = b"data".as_slice().into();
    let update_data_hex = bytes_to_hex(&update_data);

    let block_hash = BlockHash::random();
    let tx_hash = TxHash::random();
    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(block_hash),
            number: 3,
            transactions: vec![Transaction {
                hash: Some(tx_hash),
                sender,
                operations: EncodableGolemBaseTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl: 100,
                        data: update_data.clone(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            }],
        },
    )
    .await
    .unwrap();

    indexer.tick().await.unwrap();

    let block_hash_2 = BlockHash::random();
    let tx_hash_2 = TxHash::random();
    helpers::sample::insert_data(
        &*client,
        Block {
            hash: Some(block_hash_2),
            number: 4,
            transactions: vec![Transaction {
                hash: Some(tx_hash_2),
                sender,
                operations: EncodableGolemBaseTransaction {
                    deletes: vec![entity_key],
                    ..Default::default()
                },
            }],
        },
    )
    .await
    .unwrap();

    indexer.tick().await.unwrap();

    let op_index = 0;
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{tx_hash}/{op_index}"))
            .await;

    assert_fields(
        &response,
        serde_json::json!({
            "entity_key": entity_key.to_string(),
            "block_number": "3",
            "block_hash": block_hash.to_string(),
            "transaction_hash": tx_hash.to_string(),
            "data": update_data_hex,
            "prev_data": data_hex,
            "status": "ACTIVE",
            "operation": "UPDATE",
            "expires_at_block_number": "103",
            "prev_expires_at_block_number": format!("{}", 101 + extend_by + extend_by),
        }),
    );

    let op_index = 0;
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{tx_hash_2}/{op_index}"))
            .await;

    assert_fields(
        &response,
        serde_json::json!({
            "entity_key": entity_key.to_string(),
            "block_number": "4",
            "block_hash": block_hash_2.to_string(),
            "transaction_hash": tx_hash_2.to_string(),
            "data": null,
            "prev_data": update_data_hex,
            "status": "EXPIRED",
            "operation": "DELETE",
            "expires_at_block_number": "4",
            "prev_expires_at_block_number": "103",
        }),
    );
}
