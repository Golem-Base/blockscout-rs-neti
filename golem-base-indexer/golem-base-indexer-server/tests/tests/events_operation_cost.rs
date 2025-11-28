use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use helpers::assert_json::assert_fields;
use serde_json::{json, Value};

const TX_HASH: &str = "0xdf1c6dd5c0ca10d6b440dab586eadff97b4c98f184f10886bb52eb489ee3098d";

#[tokio::test]
#[ignore = "Needs database to run"]
async fn extracting_operation_cost_should_work() {
    // Setup
    let db = helpers::init_db("test", "extracting_operation_cost_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_events.sql")).await;
    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    // Operation 0: CREATE
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/0")).await;
    let expected = json!({
        "operation": "CREATE",
        "cost": "255",
    });
    assert_fields(&response, expected);

    // Operation 1: CREATE
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/1")).await;
    let expected = json!({
        "operation": "CREATE",
        "cost": "1234567890",
    });
    assert_fields(&response, expected);

    // Operation 2: DELETE has no cost associated
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/2")).await;
    let expected = json!({
        "operation": "DELETE",
        "cost": null,
    });
    assert_fields(&response, expected);

    // Operation 3: UPDATE (max 128-bit value)
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/3")).await;
    let expected = json!({
        "operation": "UPDATE",
        "cost": "340282366920938463463374607431768211455",
    });
    assert_fields(&response, expected);

    // Operation 4: UPDATE
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/4")).await;
    let expected = json!({
        "operation": "UPDATE",
        "cost": "0",
    });
    assert_fields(&response, expected);

    // Operation 5: EXTEND (max 256-bit value)
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/5")).await;
    let expected = json!({
        "operation": "EXTEND",
        "cost": "115792089237316195423570985008687907853269984665640564039457584007913129639935",
    });
    assert_fields(&response, expected);

    // Operation 6: CHANGEOWNER has no cost associated
    let response: Value =
        test_server::send_get_request(&base, &format!("/api/v1/operation/{TX_HASH}/6")).await;
    let expected = json!({
        "operation": "CHANGEOWNER",
        "cost": null,
    });
    assert_fields(&response, expected);
}
