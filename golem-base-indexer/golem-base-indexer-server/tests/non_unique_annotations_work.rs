mod helpers;

use golem_base_indexer_logic::{repository, types::EntityKey, Indexer};
use golem_base_sdk::{
    entity::{EncodableGolemBaseTransaction, Update},
    NumericAnnotation, StringAnnotation,
};
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_non_unique_annotations_work() {
    let db = helpers::init_db("test", "non_unique_annotations_work").await;
    let client = db.client();
    let entity_key = EntityKey::random();

    let indexer = Indexer::new(client.clone(), Default::default());
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                operations: EncodableGolemBaseTransaction {
                    updates: vec![Update {
                        entity_key,
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
                                value: 123,
                            },
                            NumericAnnotation {
                                key: "key1".into(),
                                value: 432,
                            },
                        ],
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ..Default::default()
            }],
        },
    )
    .await
    .unwrap();
    indexer.tick().await.unwrap();

    let entity = repository::entities::get_entity(&*client, entity_key)
        .await
        .unwrap();
    assert!(entity.is_some());

    let string_annotations =
        repository::annotations::find_active_string_annotations(&*client, entity_key)
            .await
            .unwrap();
    assert_eq!(string_annotations.len(), 2);

    let numeric_annotations =
        repository::annotations::find_active_numeric_annotations(&*client, entity_key)
            .await
            .unwrap();
    assert_eq!(numeric_annotations.len(), 2);
}
