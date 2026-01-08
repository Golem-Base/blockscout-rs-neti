mod helpers;

use alloy::primitives::{U256, address, hex};
use helpers::{eth_mock_server::EthMockServer, utils::build_test_chain_config};
use optimism_children_indexer_l3::{
    Layer3IndexerTask,
    types::{Layer3IndexerTaskOutputItem, Layer3Withdrawal, Timestamp},
};
use std::str::FromStr;

const MOCKED_CHAIN_ID: i64 = 1234567890;
const MOCKED_LATEST_BLOCK: u64 = 20;
const INDEX_FROM_BLOCK: u64 = 0;
const INJECT_WITHDRAWAL_TX_AT_BLOCK: u64 = 10;
const WITHDRAWAL_TX_HASH: &str =
    "0x91d89ac0e0d32971c8ed3de96d934215a64bcd1c59d2998f495279e708c1eaaa";
const WITHDRAWAL_TX_BLOCK_HASH: &str =
    "0x595b3bdd6b2fb42235e760ba15d3a5a58f1665bea4d5fb526a81bb68ea8be24b";
const WITHDRAWAL_TX_BLOCK: &str = r#"
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

const WITHDRAWAL_TX_RECEIPT: &str = r#"
[
    {
        "blockHash": "0x595b3bdd6b2fb42235e760ba15d3a5a58f1665bea4d5fb526a81bb68ea8be24b",
        "blockNumber": "0x1b990",
        "contractAddress": null,
        "cumulativeGasUsed": "0x112fe",
        "depositNonce": "0x2",
        "depositReceiptVersion": "0x1",
        "effectiveGasPrice": "0x0",
        "from": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
        "gasUsed": "0x5208",
        "logs": [{"address":"0x4200000000000000000000000000000000000016","topics":["0x02a52367d10742d8032712c1bb8e0144ff1ec5ffda1ed7d70bb05a2744955054","0x000100000000000000000000000000000000000000000000000000000000000d","0x00000000000000000000000085193a5ecce8f40fea01b14dd9fdf56e5d3369f6","0x00000000000000000000000085193a5ecce8f40fea01b14dd9fdf56e5d3369f6"],"data":"0x00000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000000186a00000000000000000000000000000000000000000000000000000000000000080c910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e038060000000000000000000000000000000000000000000000000000000000000000","blockHash":"0x7395f9d20e8e32c68e1257904d0dd950ebe0967b257df4f48440ee2fbc6e9023","blockNumber":"0x19e90a","blockTimestamp":"0x693f1984","transactionHash":"0x91d89ac0e0d32971c8ed3de96d934215a64bcd1c59d2998f495279e708c1eaaa","transactionIndex":"0x1","logIndex":"0x0","removed":false}],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000820000000000000000000000000000000000000000000800000200000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000800000000000000000000000000000100000000100000000000000000000000100000000000000000000000008000000000000000000",
        "status": "0x1",
        "to": "0x4200000000000000000000000000000000000016",
        "transactionHash": "0x91d89ac0e0d32971c8ed3de96d934215a64bcd1c59d2998f495279e708c1eaaa",
        "transactionIndex": "0x1",
        "type": "0x7e"
    }
]"#;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn indexing_withdrawals_should_work() {
    // Set up mock RPC
    let eth_mock_server = EthMockServer::start(MOCKED_LATEST_BLOCK).await;
    eth_mock_server
        .mount_block_by_number(
            INJECT_WITHDRAWAL_TX_AT_BLOCK,
            Some(WITHDRAWAL_TX_BLOCK_HASH),
            Some(
                serde_json::from_str(WITHDRAWAL_TX_BLOCK)
                    .expect("Withdrawal TX block serialization error"),
            ),
        )
        .await;
    eth_mock_server
        .mount_block_receipts(
            INJECT_WITHDRAWAL_TX_AT_BLOCK,
            serde_json::from_str(WITHDRAWAL_TX_RECEIPT)
                .expect("Withdrawal TX receipt serialization error"),
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

    // Extract withdrawals from items
    let withdrawals: Vec<Layer3Withdrawal> = items
        .into_iter()
        .filter_map(|item| match item {
            Layer3IndexerTaskOutputItem::Withdrawal(withdrawal) => Some(withdrawal),
            _ => None,
        })
        .collect();

    // Validate correct withdrawal was extracted
    assert_eq!(withdrawals.len(), 1);

    let expected_withdrawal = Layer3Withdrawal {
        chain_id: MOCKED_CHAIN_ID,
        block_number: INJECT_WITHDRAWAL_TX_AT_BLOCK as i64,
        block_hash: const_hex::decode(WITHDRAWAL_TX_BLOCK_HASH).unwrap(),
        block_timestamp: "2021-01-14T08:25:36Z".parse::<Timestamp>().unwrap(),
        tx_hash: const_hex::decode(WITHDRAWAL_TX_HASH).unwrap(),
        nonce: U256::from_str(
            "1766847064778384329583297500742918515827483896875618958121606201292619789",
        )
        .unwrap(),
        sender: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6").to_vec(),
        target: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6").to_vec(),
        value: U256::from_str("1000000000000000").unwrap(),
        gas_limit: U256::from_str("100000").unwrap(),
        data: vec![],
        withdrawal_hash: hex!("0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806")
            .to_vec(),
    };
    assert_eq!(withdrawals[0], expected_withdrawal);
}
