use crate::helpers;

use alloy_primitives::hex::ToHexExt;
use arkiv_storage_tx::{Create, NumericAttribute, StorageTransaction, StringAttribute};
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{
    repository,
    types::{EntitiesFilter, EntityStatus, ListEntitiesFilter, PaginationParams},
    Indexer,
};
use pretty_assertions::assert_eq;

use crate::helpers::{
    assert_json,
    sample::{Block, Transaction},
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_related_attributes_work() {
    let db = helpers::init_db("test", "get_related_attributes_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    helpers::sample::insert_data(
        &*client,
        Block {
            transactions: vec![Transaction {
                operations: StorageTransaction {
                    creates: vec![
                        Create {
                            string_attributes: vec![
                                StringAttribute {
                                    key: "key1".into(),
                                    value: "val1".into(),
                                },
                                StringAttribute {
                                    key: "key1".into(),
                                    value: "val2".into(),
                                },
                                StringAttribute {
                                    key: "key2".into(),
                                    value: "val1".into(),
                                },
                            ],
                            numeric_attributes: vec![
                                NumericAttribute {
                                    key: "key1".into(),
                                    value: 1,
                                },
                                NumericAttribute {
                                    key: "key1".into(),
                                    value: 2,
                                },
                                NumericAttribute {
                                    key: "key2".into(),
                                    value: 1,
                                },
                            ],
                            ..Default::default()
                        },
                        Create {
                            string_attributes: vec![
                                StringAttribute {
                                    key: "key1".into(),
                                    value: "val1".into(),
                                },
                                StringAttribute {
                                    key: "key1".into(),
                                    value: "val2".into(),
                                },
                            ],
                            numeric_attributes: vec![
                                NumericAttribute {
                                    key: "key1".into(),
                                    value: 1,
                                },
                                NumericAttribute {
                                    key: "key1".into(),
                                    value: 2,
                                },
                            ],
                            ..Default::default()
                        },
                        Create {
                            string_attributes: vec![StringAttribute {
                                key: "key1".into(),
                                value: "val1".into(),
                            }],
                            numeric_attributes: vec![NumericAttribute {
                                key: "key1".into(),
                                value: 1,
                            }],
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

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    // find the first entity with all attributes
    let (entities, _) = repository::entities::list_entities(
        &*client,
        ListEntitiesFilter {
            entities_filter: EntitiesFilter {
                status: Some(EntityStatus::Active),
                string_attribute: Some(golem_base_indexer_logic::types::StringAttribute {
                    key: "key2".into(),
                    value: "val1".into(),
                }),
                numeric_attribute: None,
                owner: None,
            },
            pagination: PaginationParams {
                page: 0,
                page_size: 100,
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(entities.len(), 1);
    let key = entities[0].key.encode_hex_with_prefix();

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entity/{key}")).await;
    let string_attributes = response
        .as_object()
        .unwrap()
        .get("string_attributes")
        .unwrap();
    let numeric_attributes = response
        .as_object()
        .unwrap()
        .get("numeric_attributes")
        .unwrap();
    assert_json::assert_fields_array(
        string_attributes,
        vec![
            serde_json::json!({
                "key": "key1",
                "value": "val1",
                "related_entities": "3",
            }),
            serde_json::json!({
                "key": "key1",
                "value": "val2",
                "related_entities": "2",
            }),
            serde_json::json!({
                "key": "key2",
                "value": "val1",
                "related_entities": "1",
            }),
        ],
    );
    assert_json::assert_fields_array(
        numeric_attributes,
        vec![
            serde_json::json!({
                "key": "key1",
                "value": "1",
                "related_entities": "3",
            }),
            serde_json::json!({
                "key": "key1",
                "value": "2",
                "related_entities": "2",
            }),
            serde_json::json!({
                "key": "key2",
                "value": "1",
                "related_entities": "1",
            }),
        ],
    );
}
