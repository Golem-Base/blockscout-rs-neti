mod helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_operations_endpoint_works() {
    let db = helpers::init_db("test", "list_operations_endpoint_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("fixtures/sample_data.sql")).await;

    Indexer::new(client, Default::default())
        .tick()
        .await
        .unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/operations?operation=CREATE").await;
    let expected_count = 6;
    assert_eq!(response["items"].as_array().unwrap().len(), expected_count);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=UPDATE&sender=0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
    )
    .await;
    let expected_count: u64 = 2;
    assert_eq!(
        response["pagination"],
        serde_json::json!({
            "page": "1",
            "page_size": "100",
            "total_items": expected_count.to_string(),
            "total_pages": "1"
        })
    );

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=DELETE&transaction_hash=0x1b7b3d0ac4b9636a34c72e6ab55a115a2abaa74dfcbf492d5b0b58fe13a04a96",
    )
    .await;
    let expected: serde_json::Value = serde_json::json!({
        "items": [
            {
                "entity_key": "0x4455e54a419991f302a495eea0be0fe37c121aae1f9c0048e7a1ec45900cd0cf",
                "sender": "0xDeaDDEaDDeAdDeAdDEAdDEaddeAddEAdDEAd0001",
                "operation": "DELETE",
                "data": null,
                "btl": null,
                "block_hash": "0xa53a0b7fd703287e99eeeed02b692cfd16ab8f313847e17c0580ca3aaab50076",
                "transaction_hash": "0x1b7b3d0ac4b9636a34c72e6ab55a115a2abaa74dfcbf492d5b0b58fe13a04a96",
                "index": "0"
            }
        ],
        "pagination": {
            "page": "1",
            "page_size": "100",
            "total_items": "1",
            "total_pages": "1"
        }
    });
    assert_eq!(response, expected);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=EXTEND&block_hash=0xe6f06416be4859119817b2f1d3d0f8c8fa2729804c4795452c5810e3c54b67d2",
    )
    .await;
    let expected: serde_json::Value = serde_json::json!({
        "items": [
             {
                "entity_key": "0x901799b2f558af736716b4dc4427424e1d07d420cbb8bc53ba15489c5727e84b",
                "sender": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
                "operation": "EXTEND",
                "data": null,
                "btl": "2001",
                "block_hash": "0xe6f06416be4859119817b2f1d3d0f8c8fa2729804c4795452c5810e3c54b67d2",
                "transaction_hash": "0x61080cf78f68f5813d841300d7ed257ab1a735271606d4d435e42283c4be8137",
                "index": "5"
            },
        ],
        "pagination": {
            "page": "1",
            "page_size": "100",
            "total_items": "1",
            "total_pages": "1"
        }
    });
    assert_eq!(response, expected);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=EXTEND&entity_key=0x901799b2f558af736716b4dc4427424e1d07d420cbb8bc53ba15489c5727e84b",
    )
    .await;
    let expected: serde_json::Value = serde_json::json!({
        "items": [
             {
                "entity_key": "0x901799b2f558af736716b4dc4427424e1d07d420cbb8bc53ba15489c5727e84b",
                "sender": "0xD29Bb1a1a0F6D2783306a8618b3a5b58CB313152",
                "operation": "EXTEND",
                "data": null,
                "btl": "2001",
                "block_hash": "0xe6f06416be4859119817b2f1d3d0f8c8fa2729804c4795452c5810e3c54b67d2",
                "transaction_hash": "0x61080cf78f68f5813d841300d7ed257ab1a735271606d4d435e42283c4be8137",
                "index": "5"
            },
        ],
        "pagination": {
            "page": "1",
            "page_size": "100",
            "total_items": "1",
            "total_pages": "1"
        }
    });
    assert_eq!(response, expected);

    let empty_response = serde_json::json!({"items": [], "pagination": {
        "page": "1",
        "page_size": "100",
        "total_items": "0",
        "total_pages": "0"
    }});

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=CREATE&sender=0x0000000000000000000000000000000000000000",
    )
    .await;
    let expected: serde_json::Value = empty_response.clone();
    assert_eq!(response, expected);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=CREATE&page=2&page_size=2",
    )
    .await;
    let expected_count = 2;
    let expected_pagination = serde_json::json!({
        "page": "2",
        "page_size": "2",
        "total_items": "6",
        "total_pages": "3"
    });
    assert_eq!(response["items"].as_array().unwrap().len(), expected_count);
    assert_eq!(response["pagination"], expected_pagination);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=CREATE&page=4&page_size=2",
    )
    .await;
    let expected_count = 0;
    let expected_pagination = serde_json::json!({
        "page": "4",
        "page_size": "2",
        "total_items": "6",
        "total_pages": "3"
    });
    assert_eq!(response["items"].as_array().unwrap().len(), expected_count);
    assert_eq!(response["pagination"], expected_pagination);

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/operations?operation=CREATE&page=0&page_size=256",
    )
    .await;
    let expected_count = 6;
    let expected_pagination = serde_json::json!({
        "page": "1",
        "page_size": "100",
        "total_items": "6",
        "total_pages": "1"
    });
    assert_eq!(response["items"].as_array().unwrap().len(), expected_count);
    assert_eq!(response["pagination"], expected_pagination);
}
