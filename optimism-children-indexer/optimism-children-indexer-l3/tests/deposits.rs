mod helpers;

use helpers::eth_mock_server::EthMockServer;
use helpers::utils::build_test_chain_config;
use optimism_children_indexer_l3::{
    Layer3IndexerTask,
    types::{Layer3Deposit, Layer3IndexerTaskOutputItem},
};

const MOCKED_CHAIN_ID: i64 = 1234567890;
const MOCKED_LATEST_BLOCK: u64 = 20;
const INDEX_FROM_BLOCK: u64 = 0;
const INJECT_TX_AT_BLOCK: u64 = 10;
const DEPOSIT_TX_HASH: &str = "0xb41fd72d60425a9d836d9307b6afcd8b8b217c6fe4f09d9cf7bbe155944069a2";
const DEPOSIT_TX_BLOCK_HASH: &str =
    "0x595b3bdd6b2fb42235e760ba15d3a5a58f1665bea4d5fb526a81bb68ea8be24b";
const DEPOSIT_TX_SOURCE_HASH: &str =
    "0x405ed121ccc1cd47773fbe0ef8e14b8d00acf028ac83145da72e5b6d4002efcf";
const DEPOSIT_TX_BLOCK: &str = r#"
[
    {
        "blockHash": "0x595b3bdd6b2fb42235e760ba15d3a5a58f1665bea4d5fb526a81bb68ea8be24b",
        "blockNumber": "0x1b990",
        "from": "0x481c965e6579099f005387b4c1d7fb03bc302f4b",
        "gas": "0xcf850",
        "gasPrice": "0x0",
        "hash": "0xb41fd72d60425a9d836d9307b6afcd8b8b217c6fe4f09d9cf7bbe155944069a2",
        "input": "0x",
        "nonce": "0x2",
        "to": "0x03a858395f1a6cd22e2b4d31139794aab58c5d4d",
        "transactionIndex": "0x1",
        "value": "0xde0b6b3a7640000",
        "type": "0x7e",
        "v": "0x0",
        "r": "0x0",
        "s": "0x0",
        "sourceHash": "0x405ed121ccc1cd47773fbe0ef8e14b8d00acf028ac83145da72e5b6d4002efcf",
        "mint": "0xde0b6b3a7640000",
        "depositReceiptVersion": "0x1"
    }
]
"#;

const DEPOSIT_TX_RECEIPT: &str = r#"
[
    {
        "blockHash": "0x595b3bdd6b2fb42235e760ba15d3a5a58f1665bea4d5fb526a81bb68ea8be24b",
        "blockNumber": "0x1b990",
        "contractAddress": null,
        "cumulativeGasUsed": "0x112fe",
        "depositNonce": "0x2",
        "depositReceiptVersion": "0x1",
        "effectiveGasPrice": "0x0",
        "from": "0x481c965e6579099f005387b4c1d7fb03bc302f4b",
        "gasUsed": "0x5208",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "status": "0x1",
        "to": "0x03a858395f1a6cd22e2b4d31139794aab58c5d4d",
        "transactionHash": "0xb41fd72d60425a9d836d9307b6afcd8b8b217c6fe4f09d9cf7bbe155944069a2",
        "transactionIndex": "0x1",
        "type": "0x7e"
    }
]"#;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn indexing_deposits_should_work() {
    // Set up mock RPC
    let eth_mock_server = EthMockServer::start(MOCKED_LATEST_BLOCK).await;
    eth_mock_server
        .mount_block_by_number(
            INJECT_TX_AT_BLOCK,
            Some(DEPOSIT_TX_BLOCK_HASH),
            Some(
                serde_json::from_str(DEPOSIT_TX_BLOCK)
                    .expect("Deposit TX block serialization error"),
            ),
        )
        .await;
    eth_mock_server
        .mount_block_receipts(
            INJECT_TX_AT_BLOCK,
            serde_json::from_str(DEPOSIT_TX_RECEIPT)
                .expect("Deposit TX receipt serialization error"),
        )
        .await;
    eth_mock_server.mount_defaults().await;

    // Build chain config
    let chain_config = build_test_chain_config(
        "Test chain",
        MOCKED_CHAIN_ID,
        &eth_mock_server.uri(),
        INDEX_FROM_BLOCK,
    );

    // Set up indexer
    let indexer_task = Layer3IndexerTask::new(chain_config.clone());

    // Run indexer, collect items
    let (config, items) = indexer_task.run().await.expect("IndexerTask failed");

    // Assert range of blocks was indexed
    assert_eq!(config.l3_last_indexed_block, MOCKED_LATEST_BLOCK as i64);
    assert_eq!(config.l3_latest_block, Some(MOCKED_LATEST_BLOCK as i64));

    // Extract deposits from items
    let deposits: Vec<Layer3Deposit> = items
        .into_iter()
        .filter_map(|item| match item {
            Layer3IndexerTaskOutputItem::Deposit(deposit) => Some(deposit),
            _ => None,
        })
        .collect();

    // Validate correct deposit was extracted
    assert_eq!(deposits.len(), 1);

    let extracted_deposit = Layer3Deposit {
        chain_id: MOCKED_CHAIN_ID,
        block_hash: const_hex::decode(DEPOSIT_TX_BLOCK_HASH).unwrap(),
        tx_hash: const_hex::decode(DEPOSIT_TX_HASH).unwrap(),
        source_hash: const_hex::decode(DEPOSIT_TX_SOURCE_HASH).unwrap(),
        success: true,
    };
    assert_eq!(deposits[0], extracted_deposit);
}
