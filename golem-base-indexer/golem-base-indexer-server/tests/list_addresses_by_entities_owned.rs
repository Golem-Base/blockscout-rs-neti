mod helpers;

use alloy_primitives::TxHash;
use blockscout_service_launcher::test_server;
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
async fn test_list_addresses_by_entities_owned() {
    let db = helpers::init_db("test", "list_addresses_by_entities_owned").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let owner1 = Address::random();
    let owner2 = Address::random();
    let owner3 = Address::random();

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![
                Transaction {
                    sender: owner1,
                    hash: Some(TxHash::random()),
                    operations: EncodableGolemBaseTransaction {
                        creates: vec![Create {
                            btl: 10,
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                },
                Transaction {
                    sender: owner2,
                    hash: Some(TxHash::random()),
                    operations: EncodableGolemBaseTransaction {
                        creates: vec![
                            Create {
                                btl: 10,
                                ..Default::default()
                            },
                            Create {
                                btl: 10,
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                },
                Transaction {
                    sender: owner3,
                    hash: Some(TxHash::random()),
                    operations: EncodableGolemBaseTransaction {
                        creates: vec![
                            Create {
                                btl: 10,
                                ..Default::default()
                            },
                            Create {
                                btl: 10,
                                ..Default::default()
                            },
                            Create {
                                btl: 10,
                                ..Default::default()
                            },
                        ],
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
        test_server::send_get_request(&base, "/api/v1/leaderboard/entities-owned").await;

    let expected = vec![
        serde_json::json!({
            "address": owner3.to_string(),
            "entities_count": "3",
        }),
        serde_json::json!({
            "address": owner2.to_string(),
            "entities_count": "2",
        }),
        serde_json::json!({
            "address": owner1.to_string(),
            "entities_count": "1",
        }),
    ];

    assert_eq!(response["items"].as_array().unwrap().len(), 3);
    assert_fields_array(&response["items"], expected);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/leaderboard/entities-owned?page=2&page_size=2",
    )
    .await;
    let expected = vec![serde_json::json!({
        "address": owner1.to_string(),
        "entities_count": "1",
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
