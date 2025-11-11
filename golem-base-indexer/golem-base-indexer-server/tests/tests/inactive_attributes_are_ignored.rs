use crate::helpers;

use arkiv_storage_tx::{NumericAttribute, StorageTransaction, StringAttribute, Update};
use golem_base_indexer_logic::{repository, types::EntityKey, Indexer};
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_inactive_attributes_are_ignored() {
    let db = helpers::init_db("test", "inactive_attributes_are_ignored").await;
    let client = db.client();
    let entity_key = EntityKey::random();

    let indexer = Indexer::new(client.clone(), Default::default());
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                operations: StorageTransaction {
                    updates: vec![Update {
                        entity_key,
                        string_attributes: vec![StringAttribute {
                            key: "inactive".into(),
                            value: "val1".into(),
                        }],
                        numeric_attributes: vec![NumericAttribute {
                            key: "inactive".into(),
                            value: 123,
                        }],
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
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 2,
            transactions: vec![Transaction {
                operations: StorageTransaction {
                    updates: vec![Update {
                        entity_key,
                        string_attributes: vec![StringAttribute {
                            key: "active".into(),
                            value: "val1".into(),
                        }],
                        numeric_attributes: vec![NumericAttribute {
                            key: "active".into(),
                            value: 123,
                        }],
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

    let string_attributes =
        repository::attributes::find_active_string_attributes(&*client, entity_key)
            .await
            .unwrap();
    assert_eq!(string_attributes.len(), 1);

    let numeric_attributes =
        repository::attributes::find_active_numeric_attributes(&*client, entity_key)
            .await
            .unwrap();
    assert_eq!(numeric_attributes.len(), 1);
}
