mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_single_logic_tick_works() {
    let db = helpers::init_db("test", "startup_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/operations").await;
    assert_eq!(
        response,
        serde_json::json!(include_str!("fixtures/operations.json"))
    );

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities").await;
    assert_eq!(
        response,
        serde_json::json!(include_str!("fixtures/entities.json"))
    );
}
