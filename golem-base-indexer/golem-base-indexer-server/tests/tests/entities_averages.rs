use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use serde_json::{json, Value};
use std::sync::Arc;
use test_server::send_get_request;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn entities_averages_should_work() {
    // Setup
    let db = helpers::init_db("test", "entities_averages_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(Arc::clone(&client), Default::default())
        .tick()
        .await
        .unwrap();

    // Check response
    let expected = json!({
        "average_entity_size": "22",
        "average_entity_btl": "1993",
    });
    let response: Value = send_get_request(&base, "/api/v1/entities/averages").await;
    assert_eq!(response, expected);
}
