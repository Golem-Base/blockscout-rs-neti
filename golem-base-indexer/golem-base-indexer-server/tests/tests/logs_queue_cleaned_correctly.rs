use crate::helpers;

use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;
use sea_orm::{ConnectionTrait, Statement};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_logs_queue_cleaned_correctly() {
    let db = helpers::init_db("test", "logs_queue_cleaned_correctly").await;
    let client = db.client();

    let indexer = Indexer::new(client.clone(), Default::default());

    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;
    indexer.tick().await.unwrap();

    let queue: i64 = client
        .query_one(Statement::from_string(
            client.get_database_backend(),
            "select count(*) from golem_base_pending_logs_operations;",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get_by_index(0)
        .unwrap();
    assert_eq!(queue, 0);
}
