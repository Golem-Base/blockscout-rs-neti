mod helpers;

use alloy_primitives::Address;
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::{types::EntityKey, Indexer};
use golem_base_sdk::entity::{EncodableGolemBaseTransaction, Extend, Update};

use crate::helpers::{
    assert_json::{assert_fields, assert_fields_array},
    sample::{Block, Transaction},
    utils::bytes_to_hex,
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_entity_history_endpoint_works() {
    let db = helpers::init_db("test", "get_entity_history_endpoint_works").await;
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
            }],
        },
    )
    .await
    .unwrap();

    indexer.tick().await.unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entities/{entity_key}")).await;

    assert_fields(
        &response,
        serde_json::json!({
            "key": entity_key.to_string(),
            "data": data_hex,
            "data_size": "4",
            "expires_at_block_number": "101",
            "status": "ACTIVE",
        }),
    );

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 2,
            transactions: vec![Transaction {
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

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 3,
            transactions: vec![Transaction {
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

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entities/{entity_key}/history"))
            .await;

    assert_fields(
        &response["pagination"],
        serde_json::json!({
            "page": "1",
            "page_size": "100",
            "total_items": "4",
            "total_pages": "1",
        }),
    );

    assert_fields_array(
        &response["items"],
        vec![
            serde_json::json!({
                "entity_key": entity_key.to_string(),
                "operation": "UPDATE",
                "status": "ACTIVE",
                "prev_status": null,
                "data": data_hex,
                "prev_data": null,
                "block_number": "1",
                "btl": "100",
                "expires_at_block_number": "101",
                "prev_expires_at_block_number": null,
            }),
            serde_json::json!({
                "entity_key": entity_key.to_string(),
                "operation": "EXTEND",
                "block_number": "2",
                "data": data_hex,
                "prev_data": data_hex,
                "expires_at_block_number": format!("{}", 101 + extend_by),
                "prev_expires_at_block_number": "101",
            }),
            serde_json::json!({
                "entity_key": entity_key.to_string(),
                "operation": "EXTEND",
                "block_number": "2",
                "data": data_hex,
                "prev_data": data_hex,
                "expires_at_block_number": format!("{}", 101 + extend_by + extend_by),
                "prev_expires_at_block_number": format!("{}", 101 + extend_by),
            }),
            serde_json::json!({
                "entity_key": entity_key.to_string(),
                "operation": "DELETE",
                "status": "EXPIRED",
                "prev_status": "ACTIVE",
                "data": null,
                "prev_data": data_hex,
                "block_number": "3",
                "btl": null,
                "expires_at_block_number": "3",
                "prev_expires_at_block_number": format!("{}", 101 + extend_by + extend_by),
            }),
        ],
    );

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        &format!("/api/v1/entities/{entity_key}/history?page=2&page_size=1"),
    )
    .await;

    assert_fields(
        &response["pagination"],
        serde_json::json!({
            "page": "2",
            "page_size": "1",
            "total_items": "4",
            "total_pages": "4",
        }),
    );

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entities/{entity_key}")).await;

    assert_fields(
        &response,
        serde_json::json!({
            "key": entity_key.to_string(),
            "data": null,
            "data_size": null,
            "btl": null,
            "expires_at_block_number": "3",
            "status": "DELETED",
        }),
    );
}
