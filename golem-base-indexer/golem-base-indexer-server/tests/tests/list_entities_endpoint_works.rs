use crate::helpers;

use alloy_primitives::Address;
use arkiv_storage_tx::{Create, NumericAttribute, StorageTransaction, StringAttribute};
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_entities_endpoint_works() {
    let db = helpers::init_db("test", "list_entities_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let owner = Address::random();

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![
                Transaction {
                    operations: StorageTransaction {
                        creates: vec![
                            Create {
                                string_attributes: vec![StringAttribute {
                                    key: "foo".into(),
                                    value: "bar".into(),
                                }],
                                numeric_attributes: vec![NumericAttribute {
                                    key: "foo".into(),
                                    value: 123,
                                }],
                                ..Default::default()
                            },
                            Create {
                                string_attributes: vec![StringAttribute {
                                    key: "foo".into(),
                                    value: "bar".into(),
                                }],
                                numeric_attributes: vec![NumericAttribute {
                                    key: "foo".into(),
                                    value: 123,
                                }],
                                ..Default::default()
                            },
                            Create {
                                string_attributes: vec![StringAttribute {
                                    key: "foo".into(),
                                    value: "rab".into(),
                                }],
                                numeric_attributes: vec![NumericAttribute {
                                    key: "foo".into(),
                                    value: 321,
                                }],
                                ..Default::default()
                            },
                            Create {
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Transaction {
                    sender: owner,
                    operations: StorageTransaction {
                        creates: vec![
                            Create {
                                ..Default::default()
                            },
                            Create {
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                    ..Default::default()
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
        test_server::send_get_request(&base, "/api/v1/entities?status=ALL").await;
    assert_eq!(response["items"].as_array().unwrap().len(), 6);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities?status=ACTIVE&string_attribute_key=foo&string_attribute_value=bar",
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 2);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities/count?status=ACTIVE&string_attribute_key=foo&string_attribute_value=bar",
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "2");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities?status=ACTIVE&numeric_attribute_key=foo&numeric_attribute_value=123",
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 2);

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities/count?status=ALL").await;
    assert_eq!(response["count"].as_str().unwrap(), "6");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities/count?status=ACTIVE&numeric_attribute_key=foo&numeric_attribute_value=123",
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "2");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities?status=ACTIVE&string_attribute_key=foo&string_attribute_value=rab",
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 1);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities/count?status=ACTIVE&string_attribute_key=foo&string_attribute_value=rab",
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "1");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        &format!("/api/v1/entities?status=ACTIVE&owner={owner}"),
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 2);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        &format!("/api/v1/entities/count?status=ACTIVE&owner={owner}"),
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "2");
}
