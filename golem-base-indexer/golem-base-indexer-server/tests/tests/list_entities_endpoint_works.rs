use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use golem_base_sdk::{
    entity::{Create, EncodableGolemBaseTransaction},
    NumericAnnotation, StringAnnotation,
};
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_entities_endpoint_works() {
    let db = helpers::init_db("test", "list_entities_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                operations: EncodableGolemBaseTransaction {
                    creates: vec![
                        Create {
                            string_annotations: vec![StringAnnotation {
                                key: "foo".into(),
                                value: "bar".into(),
                            }],
                            numeric_annotations: vec![NumericAnnotation {
                                key: "foo".into(),
                                value: 123,
                            }],
                            ..Default::default()
                        },
                        Create {
                            string_annotations: vec![StringAnnotation {
                                key: "foo".into(),
                                value: "bar".into(),
                            }],
                            numeric_annotations: vec![NumericAnnotation {
                                key: "foo".into(),
                                value: 123,
                            }],
                            ..Default::default()
                        },
                        Create {
                            string_annotations: vec![StringAnnotation {
                                key: "foo".into(),
                                value: "rab".into(),
                            }],
                            numeric_annotations: vec![NumericAnnotation {
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
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities?status=ACTIVE&string_annotation_key=foo&string_annotation_value=bar",
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 2);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities/count?status=ACTIVE&string_annotation_key=foo&string_annotation_value=bar",
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "2");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities?status=ACTIVE&numeric_annotation_key=foo&numeric_annotation_value=123",
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 2);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities/count?status=ACTIVE&numeric_annotation_key=foo&numeric_annotation_value=123",
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "2");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities?status=ACTIVE&string_annotation_key=foo&string_annotation_value=rab",
    )
    .await;
    assert_eq!(response["items"].as_array().unwrap().len(), 1);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/entities/count?status=ACTIVE&string_annotation_key=foo&string_annotation_value=rab",
    )
    .await;
    assert_eq!(response["count"].as_str().unwrap(), "1");
}
