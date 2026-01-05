use std::thread::sleep;

use crate::helpers::{self, utils::gen_block_resp};

use blockscout_service_launcher::test_server;
use wiremock::{
    matchers::{method, path_regex, query_param},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_consensus_info() {
    let db = helpers::init_db("test", "get_consensus_info").await;

    let rpc_mock = MockServer::start().await;
    let blockscout_mock = MockServer::start().await;

    let base = helpers::init_golem_base_indexer_server(db, |mut x| {
        x.external_services.l3_rpc_url = rpc_mock.uri();
        x.external_services.l2_blockscout_url = blockscout_mock.uri();
        x.external_services.l2_batcher_address =
            "0x268d5F26c5db34A929fb4aE9096EbA2c1C05Ec0F".to_string();
        x.external_services.l2_batch_inbox_address =
            "0x00917b20026005FD08c4163de344e14Fd83Fb740".to_string();
        x.external_services.cache_ttl_seconds = 1;
        x
    })
    .await;

    // Test fallback to zeros
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/chain/consensus-info").await;

    assert_eq!(
        response,
        serde_json::json!({
            "finalized_block_number": "0",
            "finalized_block_timestamp": "1970-01-01 00:00:00 UTC",
            "safe_block_number": "0",
            "safe_block_timestamp": "1970-01-01 00:00:00 UTC",
            "unsafe_block_number": "0",
            "unsafe_block_timestamp": "1970-01-01 00:00:00 UTC",
            "rollup_gas_price": "0",
            "rollup_gas_used": "0",
            "rollup_transaction_fee":"0",
            "rollup_average_transaction_cost": "0",
        })
    );

    // Allow cache to expire
    sleep(std::time::Duration::from_secs(1));

    // IDs will increment with each request
    let rpc_response = serde_json::json!([
        gen_block_resp(123, 666, 3), // latest
        gen_block_resp(120, 665, 4), // safe
        gen_block_resp(110, 655, 5), // finalized
    ]);

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&rpc_response))
        .mount(&rpc_mock)
        .await;

    let addresses_tx_1: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/blockscout_addresses_tx_1.json")).unwrap();
    let addresses_tx_2: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/blockscout_addresses_tx_2.json")).unwrap();
    let addresses_counters: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/blockscout_addresses_counters.json"
    ))
    .unwrap();
    let txinfo: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/blockscout_txinfo_v2.json")).unwrap();

    Mock::given(method("GET"))
        .and(path_regex(r"/addresses/[a-zA-Z0-9]{1,}/transactions$"))
        .and(query_param(
            "hash",
            "0x70b14fa93e371d1361793596850bd51cb39740af1a15d441fcd363dd0cc859f2",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(&addresses_tx_2))
        .mount(&blockscout_mock)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"/addresses/[a-zA-Z0-9]{1,}/transactions$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&addresses_tx_1))
        .mount(&blockscout_mock)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"/addresses/[a-zA-Z0-9]{1,}/counters$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&addresses_counters))
        .mount(&blockscout_mock)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"/transactions/[a-zA-Z0-9]{1,}$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&txinfo))
        .mount(&blockscout_mock)
        .await;

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/chain/consensus-info").await;

    let expected: serde_json::Value = serde_json::json!({
        "unsafe_block_number": "123",
        "unsafe_block_timestamp": "1970-01-01 00:11:06 UTC",
        "safe_block_number": "120",
        "safe_block_timestamp": "1970-01-01 00:11:05 UTC",
        "finalized_block_number": "110",
        "finalized_block_timestamp": "1970-01-01 00:10:55 UTC",
        "rollup_gas_price": "1000282107",
        "rollup_gas_used": "23272",
        "rollup_transaction_fee":"23278565194104",
        "rollup_average_transaction_cost": "1302377"
      }
    );
    assert_eq!(response, expected);
}
