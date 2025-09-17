mod helpers;

use alloy_primitives::TxHash;
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::Indexer;
use golem_base_sdk::{
    entity::{Create, EncodableGolemBaseTransaction},
    Address,
};
use pretty_assertions::assert_eq;

use crate::helpers::{
    assert_json::{assert_fields, assert_fields_array},
    sample::{Block, Transaction},
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn list_addresses_by_data_owned() {
    let db = helpers::init_db("test", "list_addresses_by_data_owned").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let owner1 = Address::random();
    let owner2 = Address::random();
    let owner3 = Address::random();

    let data: Bytes = "10 bytes  ".into();
    let create = Create {
        btl: 10,
        data: data.clone(),
        ..Default::default()
    };

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![
                Transaction {
                    sender: owner1,
                    hash: Some(TxHash::random()),
                    operations: EncodableGolemBaseTransaction {
                        creates: vec![create.clone()],
                        ..Default::default()
                    },
                },
                Transaction {
                    sender: owner2,
                    hash: Some(TxHash::random()),
                    operations: EncodableGolemBaseTransaction {
                        creates: vec![create.clone(), create.clone()],
                        ..Default::default()
                    },
                },
                Transaction {
                    sender: owner3,
                    hash: Some(TxHash::random()),
                    operations: EncodableGolemBaseTransaction {
                        creates: vec![create.clone(), create.clone(), create.clone()],
                        ..Default::default()
                    },
                },
            ],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/leaderboard/data-owned").await;

    let expected = vec![
        serde_json::json!({
            "address": owner3.to_string(),
            "data_size": "30",
        }),
        serde_json::json!({
            "address": owner2.to_string(),
            "data_size": "20",
        }),
        serde_json::json!({
            "address": owner1.to_string(),
            "data_size": "10",
        }),
    ];

    assert_eq!(response["items"].as_array().unwrap().len(), 3);
    assert_fields_array(&response["items"], expected);

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/leaderboard/data-owned?page=2&page_size=2")
            .await;
    let expected = vec![serde_json::json!({
        "address": owner1.to_string(),
        "data_size": "10",
    })];
    assert_eq!(response["items"].as_array().unwrap().len(), 1);
    assert_fields_array(&response["items"], expected);
    assert_fields(
        &response["pagination"],
        serde_json::json!({
            "page": "2",
            "page_size": "2",
            "total_pages": "2",
            "total_items": "3",
        }),
    );
}
