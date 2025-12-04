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
        "cost": "0"
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
        "cost": "0"
    });
    assert_fields(&response, expected);
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn entity_cost_calculation_should_work() {
    // Setup
    let db = helpers::init_db("test", "entity_cost_calculation_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/sample_events.sql")).await;
    let indexer = Indexer::new(client, Default::default());
    indexer.tick().await.unwrap();

    // Entity '0xd45c0192c8d31259e3e9814ef92dfbaab2f93b5634ebb12b1f1d6a281295c937'
    // (CREATE)
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0xd45c0192c8d31259e3e9814ef92dfbaab2f93b5634ebb12b1f1d6a281295c937",
    )
    .await;
    let expected = json!({
        "key": "0xd45c0192c8d31259e3e9814ef92dfbaab2f93b5634ebb12b1f1d6a281295c937",
        "cost": "255",
    });
    assert_fields(&response, expected);

    // Entity '0x273c8d51639b1bda61b9980245d02f1ce09755275238a61213c6f0151e4daa3a'
    // (CREATE + CHANGEOWNER)
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0x273c8d51639b1bda61b9980245d02f1ce09755275238a61213c6f0151e4daa3a",
    )
    .await;
    let expected = json!({
        "key": "0x273c8d51639b1bda61b9980245d02f1ce09755275238a61213c6f0151e4daa3a",
        "cost": "1337",
    });
    assert_fields(&response, expected);

    // Entity '0x51fb191a5bc9f9bdd81078f85acd81dc9f963a4289ff4a0946074fa68d94e5bd'
    // (CREATE + EXTEND)
    // Should not overflow 256-bit
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0x51fb191a5bc9f9bdd81078f85acd81dc9f963a4289ff4a0946074fa68d94e5bd",
    )
    .await;
    let expected = json!({
        "key": "0x51fb191a5bc9f9bdd81078f85acd81dc9f963a4289ff4a0946074fa68d94e5bd",
        "cost": "115792089237316195423570985008687907853269984665640564039457584007913129639935",
    });
    assert_fields(&response, expected);

    // Entity '0xa2d390d9229f9145cfd5d0872600c5070c2cc75018100d685c2f44e34d019c09'
    // (CREATE + UPDATE)
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0xa2d390d9229f9145cfd5d0872600c5070c2cc75018100d685c2f44e34d019c09",
    )
    .await;
    let expected = json!({
        "key": "0xa2d390d9229f9145cfd5d0872600c5070c2cc75018100d685c2f44e34d019c09",
        "cost": "340282366920938463463374607431768212479",
    });
    assert_fields(&response, expected);

    // Entity '0x2ec62779f7d18be24f828e6ee58695cd79b3f602ad10dc7161a22f5477b46f47'
    // (DELETE)
    let response: Value = test_server::send_get_request(
        &base,
        "/api/v1/entity/0x2ec62779f7d18be24f828e6ee58695cd79b3f602ad10dc7161a22f5477b46f47",
    )
    .await;
    let expected = json!({
        "key": "0x2ec62779f7d18be24f828e6ee58695cd79b3f602ad10dc7161a22f5477b46f47",
        "cost": "0",
    });
    assert_fields(&response, expected);
}
