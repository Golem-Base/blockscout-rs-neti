use crate::helpers;

use blockscout_service_launcher::test_server;
use optimism_children_indexer_logic::Indexer;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_deposits_endpoint() {
    let db = helpers::init_db("test", "deposits_endpoint").await;
    let client = db.client();

    let base = helpers::init_optimism_children_indexer_server(db, |x| x).await;

    let indexer = Indexer::new(client.clone(), Default::default());

    // load txs first, then logs, to simulate how it really happens in blockscout and to test we
    // handle such race condition correctly
    helpers::load_data(
        &*client,
        include_str!("../fixtures/sample_l2_deposit_data.sql"),
    )
    .await;
    indexer.tick().await.unwrap();

    // load L3 sample data
    helpers::load_data(
        &*client,
        include_str!("../fixtures/sample_l3_deposit_data.sql"),
    )
    .await;

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/deposits").await;

    assert_eq!(
        response,
        json!({
            "items": [
                {
                    "destination_chain_id": "21377321",
                    "execution_tx": {
                        "block_hash": "0x595b3bdd6b2fb42235e760ba15d3a5a58f1665bea4d5fb526a81bb68ea8be24b",
                        "block_number": "113040",
                        "from": "0x481c965E6579099F005387B4C1D7fb03bC302F4b",
                        "to": "0x03a858395F1a6cd22e2B4D31139794AaB58C5D4d",
                        "transaction_hash": "0xb41fd72d60425a9d836d9307b6afcd8b8b217c6fe4f09d9cf7bbe155944069a2",
                        "success": true,
                    },
                    "from": "0x481c965E6579099F005387B4C1D7fb03bC302F4b",
                    "gas_limit": "850000",
                    "init_tx": {
                        "block_hash": "0x7ec13ee95beee46e78b51559fbf985b2d1124c5b98e7ecc6174ddf90afa30f16",
                        "block_number": "9398574",
                        "from": "0x03a858395F1a6cd22e2B4D31139794AaB58C5D4d",
                        "to": "0x370b965e6579099f005387b4c1D7Fb03bC301e3A",
                        "transaction_hash": "0x387f9c25f22259f8a044d289434f0a5f49f9259205fd1bd2711a16fe29235bfc",
                        "success": true,
                    },
                    "is_creation": false,
                    "mint": "1000000000000000000",
                    "to": "0x03a858395F1a6cd22e2B4D31139794AaB58C5D4d",
                    "value": "1000000000000000000",
                },
                {
                    "destination_chain_id": "60138453025",
                    "execution_tx": {
                        "block_hash": "0xa1075db55d416d3ca199f55b6084e2115b9345e16c5cf302fc80e9d5fbf5d48d",
                        "block_number": "82060",
                        "from": "0x2f67dd5b165D37b179D3cEcEBFa67765a38c8ad3",
                        "to": "0x7bB44cCbAb1445d064049ed6a1ccb6f116aDd1B9",
                        "transaction_hash": "0x6b52cc514f3a4d953d5648a69d3f309e52aaeb55e62ee683d4ca526163cc153d",
                        "success": true,
                    },
                    "from": "0x3C41d8343A1Cba9FD6f0356039b6c6d844610321",
                    "gas_limit": "414371",
                    "init_tx": {
                        "block_hash": "0x3466e222249e9f13be5130d4623e1fc2a5bc1c6c258c510773b50977be70f5df",
                        "block_number": "29466",
                        "from": "0x17acfafcfa4A6912F97d85950F37ceEf97305393",
                        "to": "0x8cF3068a4a1C4f329Cc19b7c57BD4b2e7EaA3662",
                        "transaction_hash": "0x653d3f9ec83c23f5e870e6d2710961a681e3bfbb280d8c19da7739146df3b6bb",
                        "success": true,
                    },
                    "is_creation": false,
                    "mint": "1000000000000000",
                    "to": "0x4200000000000000000000000000000000000007",
                    "value": "1000000000000000",
                },
                {
                    "destination_chain_id": null,
                    "execution_tx": null,
                    "from": "0x3C41d8343A1Cba9FD6f0356039b6c6d844610321",
                    "gas_limit": "414371",
                    "init_tx": {
                        "block_hash": "0xff58bc22f06613e01ef3d63ceeb24e9209ec304c7b71599f84bf804187bc2867",
                        "block_number": "29280",
                        "from": "0x17acfafcfa4A6912F97d85950F37ceEf97305393",
                        "to": "0x8cF3068a4a1C4f329Cc19b7c57BD4b2e7EaA3662",
                        "transaction_hash": "0xc913706ddb07d506aebab4bb006be97c02147fc5ac9a58497f17b22486dc72f3",
                        "success": true,
                    },
                    "is_creation": false,
                    "mint": "1000000000000000",
                    "to": "0x4200000000000000000000000000000000000007",
                    "value": "1000000000000000",
                },
            ],
            "next_page_params": {
                "page": "1",
                "items_count": "100",
                "total_items": "3",
                "total_pages": "1",
            },
        })
    );
}
