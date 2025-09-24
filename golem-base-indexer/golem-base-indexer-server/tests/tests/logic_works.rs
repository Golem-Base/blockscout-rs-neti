use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_single_logic_tick_works() {
    let db = helpers::init_db("test", "single_logic_tick_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=ACTIVE").await;
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/active_entities.json")).unwrap();
    assert_eq!(response, expected);

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=EXPIRED").await;
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/expired_entities.json")).unwrap();
    assert_eq!(response, expected);

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=DELETED").await;
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/deleted_entities.json")).unwrap();
    assert_eq!(response, expected);
}
