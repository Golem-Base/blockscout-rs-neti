mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_entity_history_endpoint_works() {
    let db = helpers::init_db("test", "get_entity_history_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let entity_key = "0x9eac1ce575a48fc3dff0b2c68b9025c5645b12b148106546e723ff4372dfa1ba";

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entities/{entity_key}/history"))
            .await;

    assert_eq!(response["pagination"], one_page_default_pagination(2));
    assert_eq!(
        response["items"],
        serde_json::json!([
            {
                "entity_key": entity_key,
                "block_timestamp": "2025-07-22T11:31:31+00:00",
                "block_number": "3",
                "transaction_hash": "0xe6f8d9804b3c90d037ada0f2becec32375fab8a76a813df9e589d7729302d8e9",
                "tx_index": "1",
                "status": "ACTIVE",
                "prev_status": null,
                "sender": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
                "operation": "CREATE",
                "data": "0x6461746120746861742077696C6C2062652075706461746564",
                "prev_data": null,
                "btl": "1000",
                "expires_at_block_number": "1003",
                "prev_expires_at_block_number": null,
                "op_index": "0",
            },
            {
                "entity_key": entity_key,
                "block_timestamp": "2025-07-22T11:31:34+00:00",
                "block_number": "6",
                "transaction_hash": "0x61080cf78f68f5813d841300d7ed257ab1a735271606d4d435e42283c4be8137",
                "tx_index": "1",
                "status": "ACTIVE",
                "prev_status": "ACTIVE",
                "sender": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
                "operation": "UPDATE",
                "data": "0x757064617465642064617461",
                "prev_data": "0x6461746120746861742077696C6C2062652075706461746564",
                "btl": "2000",
                "expires_at_block_number": "2006",
                "prev_expires_at_block_number": "1003",
                "op_index": "3",
        }])
    );

    let entity_key = "0x901799b2f558af736716b4dc4427424e1d07d420cbb8bc53ba15489c5727e84b";

    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entities/{entity_key}/history"))
            .await;

    assert_eq!(response["pagination"], one_page_default_pagination(2));
    assert_eq!(
        response["items"],
        serde_json::json!([
            {
                "entity_key": entity_key,
                "block_timestamp": "2025-07-22T11:31:33+00:00",
                "block_number": "5",
                "transaction_hash": "0xcda0828e3bddc077c05487533aef77fc52f417e1beccea1962996a23de3e32f5",
                "tx_index": "1",
                "status": "ACTIVE",
                "prev_status": null,
                "sender": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
                "operation": "CREATE",
                "data": "0x6461746120746861742077696C6C20626520657874656E646564",
                "prev_data": null,
                "btl": "1000",
                "expires_at_block_number": "1005",
                "prev_expires_at_block_number": null,
                "op_index": "0",
            },
            {
                "entity_key": entity_key,
                "block_timestamp": "2025-07-22T11:31:34+00:00",
                "block_number": "6",
                "transaction_hash": "0x61080cf78f68f5813d841300d7ed257ab1a735271606d4d435e42283c4be8137",
                "tx_index": "1",
                "status": "ACTIVE",
                "prev_status": "ACTIVE",
                "sender": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
                "operation": "EXTEND",
                "data": null,
                "prev_data": "0x6461746120746861742077696C6C20626520657874656E646564",
                "btl": "2001",
                "expires_at_block_number": "2007",
                "prev_expires_at_block_number": "1005",
                "op_index": "5",
        }])
    );
}

fn one_page_default_pagination(expected_total_items: u64) -> serde_json::Value {
    serde_json::json!({
        "page": "1",
        "page_size": "100",
        "total_items": expected_total_items.to_string(),
        "total_pages": "1"
    })
}
