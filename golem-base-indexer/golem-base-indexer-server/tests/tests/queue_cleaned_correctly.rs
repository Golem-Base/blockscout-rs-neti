use crate::helpers;

use golem_base_indexer_logic::Indexer;
use golem_base_sdk::entity::{Create, EncodableGolemBaseTransaction};
use pretty_assertions::assert_eq;
use sea_orm::{ConnectionTrait, Statement};

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_queue_gets_cleaned() {
    let db = helpers::init_db("test", "queue_gets_cleaned").await;
    let client = db.client();
    helpers::sample::insert_data(
        &*client,
        Block {
            transactions: vec![Transaction {
                operations: EncodableGolemBaseTransaction {
                    creates: vec![Create {
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

    let queue: i64 = client
        .query_one(Statement::from_string(
            client.get_database_backend(),
            "select count(*) from golem_base_pending_transaction_operations;",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get_by_index(0)
        .unwrap();
    assert_eq!(queue, 1);

    Indexer::new(client.clone(), Default::default())
        .tick()
        .await
        .unwrap();

    let queue: i64 = client
        .query_one(Statement::from_string(
            client.get_database_backend(),
            "select count(*) from golem_base_pending_transaction_operations;",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get_by_index(0)
        .unwrap();
    assert_eq!(queue, 0);
}
