mod helpers;

use golem_base_indexer_logic::{repository, types::EntityKey, Indexer};
use golem_base_sdk::entity::{EncodableGolemBaseTransaction, Update};
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_expired_entities_dont_get_updated() {
    let db = helpers::init_db("test", "expired_entities_dont_get_updated").await;
    let client = db.client();
    let entity_key = EntityKey::random();

    let indexer = Indexer::new(client.clone(), Default::default());
    // inserting out of order
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 2,
            transactions: vec![Transaction {
                operations: EncodableGolemBaseTransaction {
                    deletes: vec![entity_key],
                    ..Default::default()
                },
                ..Default::default()
            }],
        },
    )
    .await
    .unwrap();
    indexer.tick().await.unwrap();
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                operations: EncodableGolemBaseTransaction {
                    updates: vec![Update {
                        entity_key,
                        btl: 100,
                        data: b"asd".as_slice().into(),
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
        .unwrap()
        .unwrap();

    assert_eq!(entity.data, None);
}
