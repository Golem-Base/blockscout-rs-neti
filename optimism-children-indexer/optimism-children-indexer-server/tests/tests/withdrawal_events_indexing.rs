use crate::helpers;

use alloy_primitives::{address, b256, bytes, U256};
use optimism_children_indexer_logic::{
    repository,
    types::{
        EventMetadata, FullEvent, FullWithdrawal, PaginationParams, Timestamp,
        WithdrawalFinalizedEvent, WithdrawalProvenEvent,
    },
    Indexer,
};
use pretty_assertions::assert_eq;
use std::str::FromStr;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn withdrawal_events_indexing_should_work() {
    // Setup
    let _ = tracing_subscriber::fmt::try_init();
    let db = helpers::init_db("test", "withdrawal_events_indexing_should_work").await;
    let client = db.client();
    let indexer = Indexer::new(client.clone(), Default::default());

    // Load sample L2 data
    helpers::load_data(
        &*client,
        include_str!("../fixtures/sample_l2_withdrawal_data.sql"),
    )
    .await;

    // Run indexer
    indexer.tick().await.unwrap();

    // Load sample L3 data
    helpers::load_data(
        &*client,
        include_str!("../fixtures/sample_l3_withdrawal_data.sql"),
    )
    .await;

    // Fetch events
    let (events, _) = repository::withdrawals::list_withdrawals(
        &*client,
        PaginationParams {
            page: 1,
            page_size: 10,
        },
    )
    .await
    .unwrap();

    let expected = vec![
        FullWithdrawal {
            chain_id: 21377321,
            l3_block_number: 123128,
            l3_block_hash: b256!(
                "0x48079624a0b115a086742a6ab17ec5849c73d7b67264409e8b11e2c624622cc1"
            ),
            l3_block_timestamp: "2025-12-16 05:13:09.095795Z".parse::<Timestamp>().unwrap(),
            l3_tx_hash: b256!("0xf40aa108279a01cd10c4ee1aa07099733f746b6cb6b18b9342d5d603ae7eec4c"),
            nonce: U256::from_str(
                "1766847064778384329583297500742918515827483896875618958121606201292619792",
            )
            .unwrap(),
            sender: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6"),
            target: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6"),
            value: U256::from_str("100000000000000").unwrap(),
            gas_limit: U256::from_str("100000").unwrap(),
            data: bytes!(),
            withdrawal_hash: b256!(
                "0x5710ecaa2f28c84097517d9e91ddd9e40717b7cb9ea9a4781fc93b9dc9a8306f"
            ),
            proving_tx: None,
            finalizing_tx: None,
        },
        FullWithdrawal {
            chain_id: 60138453025,
            l3_block_number: 1734398,
            l3_block_hash: b256!(
                "0x3d275c76cfcc2b73600f455f42fc27bbebbdd51752f800d451cbbad028677082"
            ),
            l3_block_timestamp: "2025-12-16 05:13:09.095795Z".parse::<Timestamp>().unwrap(),
            l3_tx_hash: b256!("0xff9b299fabddbaa12a49c04e6a6088c69c10b93e64d5718d0b0e8b92659a3652"),
            nonce: U256::from_str(
                "1766847064778384329583297500742918515827483896875618958121606201292619790",
            )
            .unwrap(),
            sender: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6"),
            target: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6"),
            value: U256::from_str("100000000000000").unwrap(),
            gas_limit: U256::from_str("100000").unwrap(),
            data: bytes!(),
            withdrawal_hash: b256!(
                "0x6205f1d611864e90818bfc25cb2de88d17d82973d0a1987bf0b2b934144f26b5"
            ),
            proving_tx: Some(FullEvent {
                metadata: EventMetadata {
                    from: address!("0xf6c95fc7e9a0c1d2e3f4a5b6c7d8e9f0a1b2c3d4"),
                    to: address!("0x4c3f68ab087ef1c74b422c0beb6be37eee944a1f"),
                    transaction_hash: b256!(
                        "0xe079e741dc364041092233e5029e130fe57f72fed0f0c6230972bc0a25c30b09"
                    ),
                    block_hash: b256!(
                        "0xb8f6e04c68cf33dd5348e28f05e05e830c072ef55f59c0f22cfd084d3d3e2e7d"
                    ),
                    index: 333,
                    block_number: 1716600,
                    block_timestamp: "2025-06-23 16:33:27Z".parse::<Timestamp>().unwrap(),
                },
                event: WithdrawalProvenEvent {
                    withdrawal_hash: b256!(
                        "0x6205f1d611864e90818bfc25cb2de88d17d82973d0a1987bf0b2b934144f26b5"
                    ),
                    from: address!("0xcc1d9565e0f8b9c8f6a7f8e9f1a2b3c4d5e6f7a8"),
                    to: address!("0xf6c95fc7e9a0c1d2e3f4a5b6c7d8e9f0a1b2c3d4"),
                },
            }),
            finalizing_tx: None,
        },
        FullWithdrawal {
            chain_id: 60138453025,
            l3_block_number: 1698058,
            l3_block_hash: b256!(
                "0x7395f9d20e8e32c68e1257904d0dd950ebe0967b257df4f48440ee2fbc6e9023"
            ),
            l3_block_timestamp: "2025-12-15T22:40:26.392458Z".parse::<Timestamp>().unwrap(),
            l3_tx_hash: b256!("0x91d89ac0e0d32971c8ed3de96d934215a64bcd1c59d2998f495279e708c1eaaa"),
            nonce: U256::from_str(
                "1766847064778384329583297500742918515827483896875618958121606201292619789",
            )
            .unwrap(),
            sender: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6"),
            target: address!("0x85193a5ecce8f40fea01b14dd9fdf56e5d3369f6"),
            value: U256::from_str("1000000000000000").unwrap(),
            gas_limit: U256::from_str("100000").unwrap(),
            data: bytes!(),
            withdrawal_hash: b256!(
                "0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806"
            ),
            proving_tx: Some(FullEvent {
                metadata: EventMetadata {
                    from: address!("0xe5b84fc6e8c9b3db25a58e0a1d9287e0de4bc96a"),
                    to: address!("0x4c3f68ab087ef1c74b422c0beb6be37eee944a1f"),
                    transaction_hash: b256!(
                        "0xd968d630cb253930981122d4918d12ffc46e61edcf9fb512f8617ba9914b2fa8"
                    ),
                    block_hash: b256!(
                        "0xa7e5d93b57bf22cc4237f17f94d94d719b961de44e48bfe11bedef73c2c1d6c5"
                    ),
                    index: 222,
                    block_number: 1716500,
                    block_timestamp: "2025-06-06 08:32:40Z".parse::<Timestamp>().unwrap(),
                },
                event: WithdrawalProvenEvent {
                    withdrawal_hash: b256!(
                        "0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806"
                    ),
                    from: address!("0xbb0c8454e9d220d835c594e630731cbbd396ae14"),
                    to: address!("0xe5b84fc6e8c9b3db25a58e0a1d9287e0de4bc96a"),
                },
            }),
            finalizing_tx: Some(FullEvent {
                metadata: EventMetadata {
                    from: address!("0xd4a77d618b9f512960fa21714361341e129cae6b"),
                    to: address!("0x4c3f68ab087ef1c74b422c0beb6be37eee944a1f"),
                    transaction_hash: b256!(
                        "0xc857c519ba142819870011c3817c11feb35d50dcb8eb401f7506aa9803a1e9e7"
                    ),
                    block_hash: b256!(
                        "0x96f4e92a46af11bb3126f36f06f83c608a850cf33d37afd00adcfe62b1b0c5b4"
                    ),
                    index: 111,
                    block_number: 1716393,
                    block_timestamp: "2025-06-06 08:32:40Z".parse::<Timestamp>().unwrap(),
                },
                event: WithdrawalFinalizedEvent {
                    withdrawal_hash: b256!(
                        "0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806"
                    ),
                    success: true,
                },
            }),
        },
    ];

    assert_eq!(events, expected);
}
