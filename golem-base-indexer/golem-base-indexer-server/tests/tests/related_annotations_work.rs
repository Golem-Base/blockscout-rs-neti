use crate::helpers;

use alloy_primitives::hex::ToHexExt;
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{
    repository,
    types::{EntitiesFilter, EntityStatus, ListEntitiesFilter, PaginationParams},
    Indexer,
};
use golem_base_sdk::{
    entity::{Create, EncodableGolemBaseTransaction},
    NumericAnnotation, StringAnnotation,
};
use pretty_assertions::assert_eq;

use crate::helpers::{
    assert_json,
    sample::{Block, Transaction},
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_related_annotations_work() {
    let db = helpers::init_db("test", "get_related_annotations_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    helpers::sample::insert_data(
        &*client,
        Block {
            transactions: vec![Transaction {
                operations: EncodableGolemBaseTransaction {
                    creates: vec![
                        Create {
                            string_annotations: vec![
                                StringAnnotation {
                                    key: "key1".into(),
                                    value: "val1".into(),
                                },
                                StringAnnotation {
                                    key: "key1".into(),
                                    value: "val2".into(),
                                },
                                StringAnnotation {
                                    key: "key2".into(),
                                    value: "val1".into(),
                                },
                            ],
                            numeric_annotations: vec![
                                NumericAnnotation {
                                    key: "key1".into(),
                                    value: 1,
                                },
                                NumericAnnotation {
                                    key: "key1".into(),
                                    value: 2,
                                },
                                NumericAnnotation {
                                    key: "key2".into(),
                                    value: 1,
                                },
                            ],
                            ..Default::default()
                        },
                        Create {
                            string_annotations: vec![
                                StringAnnotation {
                                    key: "key1".into(),
                                    value: "val1".into(),
                                },
                                StringAnnotation {
                                    key: "key1".into(),
                                    value: "val2".into(),
                                },
                            ],
                            numeric_annotations: vec![
                                NumericAnnotation {
                                    key: "key1".into(),
                                    value: 1,
                                },
                                NumericAnnotation {
                                    key: "key1".into(),
                                    value: 2,
                                },
                            ],
                            ..Default::default()
                        },
                        Create {
                            string_annotations: vec![StringAnnotation {
                                key: "key1".into(),
                                value: "val1".into(),
                            }],
                            numeric_annotations: vec![NumericAnnotation {
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

    // find the first entity with all annotations
    let (entities, _) = repository::entities::list_entities(
        &*client,
        ListEntitiesFilter {
            entities_filter: EntitiesFilter {
                status: Some(EntityStatus::Active),
                string_annotation: Some(golem_base_indexer_logic::types::StringAnnotation {
                    key: "key2".into(),
                    value: "val1".into(),
                }),
                numeric_annotation: None,
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
    let string_annotations = response
        .as_object()
        .unwrap()
        .get("string_annotations")
        .unwrap();
    let numeric_annotations = response
        .as_object()
        .unwrap()
        .get("numeric_annotations")
        .unwrap();
    assert_json::assert_fields_array(
        string_annotations,
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
        numeric_annotations,
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
