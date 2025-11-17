use crate::helpers;

use alloy_primitives::{Address, TxHash};
use arkiv_storage_tx::{Extend, StorageTransaction, Update};
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::{
    types::{BlockHash, EntityKey},
    Indexer,
};

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
    let content_type = "application/ogg".to_string();

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                sender,
                operations: StorageTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl: 100,
                        payload: data.clone(),
                        content_type: content_type.clone(),
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
                operations: StorageTransaction {
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
                ..Default::default()
            }],
            ..Default::default()
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
            "content_type": content_type.clone(),
            "prev_content_type": content_type.clone(),
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
            "content_type": content_type.clone(),
            "prev_content_type": content_type.clone(),
        }),
    );

    let update_data: Bytes = b"data".as_slice().into();
    let update_data_hex = bytes_to_hex(&update_data);
    let update_content_type = "application/x-www-form-urlencoded".to_string();

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
                operations: StorageTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl: 100,
                        payload: update_data.clone(),
                        content_type: update_content_type.clone(),
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
                operations: StorageTransaction {
                    deletes: vec![entity_key],
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
            "expires_at_timestamp": "2018-10-13T12:33:26+00:00",
            "expires_at_timestamp_sec": "1539434006",
            "prev_expires_at_timestamp": "2018-10-13T12:41:34+00:00",
            "prev_expires_at_timestamp_sec": "1539434494",
            "content_type": update_content_type.clone(),
            "prev_content_type": content_type.clone(),
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
            "status": "DELETED",
            "operation": "DELETE",
            "expires_at_block_number": "4",
            "prev_expires_at_block_number": "103",
            "expires_at_timestamp": "2018-10-13T12:30:08+00:00",
            "expires_at_timestamp_sec": "1539433808",
            "prev_expires_at_timestamp": "2018-10-13T12:33:26+00:00",
            "prev_expires_at_timestamp_sec": "1539434006",
            "content_type": null,
            "prev_content_type": update_content_type.clone(),
        }),
    );
}
