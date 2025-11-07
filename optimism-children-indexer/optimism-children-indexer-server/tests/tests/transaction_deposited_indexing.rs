use crate::helpers;

use alloy_primitives::{address, b256, bytes};
use optimism_children_indexer_logic::{
    repository,
    types::{DepositV0, EventMetadata, FullEvent, PaginationParams, TransactionDepositedEvent},
    Indexer,
};
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_transaction_deposited_indexing() {
    let _ = tracing_subscriber::fmt::try_init();

    let db = helpers::init_db("test", "transaction_deposited_indexing").await;
    let client = db.client();

    let indexer = Indexer::new(client.clone(), Default::default());

    // load txs first, then logs, to simulate how it really happens in blockscout and to test we
    // handle such race condition correctly
    helpers::load_data(
        &*client,
        include_str!("../fixtures/sample_l2_deposit_data.sql"),
    )
    .await;
    indexer.tick().await.unwrap();

    let (events, _) = repository::deposits::list_deposits(
        &*client,
        PaginationParams {
            page: 1,
            page_size: 10,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        events,
        vec![
            FullEvent::<TransactionDepositedEvent<DepositV0>> {
                metadata: EventMetadata {
                    from: address!("0x03a858395F1a6cd22e2B4D31139794AaB58C5D4d"),
                    to: address!("0x370b965e6579099f005387b4c1D7Fb03bC301e3A"),
                    transaction_hash: b256!(
                        "0x387f9c25f22259f8a044d289434f0a5f49f9259205fd1bd2711a16fe29235bfc"
                    ),
                    block_hash: b256!(
                        "0x7ec13ee95beee46e78b51559fbf985b2d1124c5b98e7ecc6174ddf90afa30f16"
                    ),
                    index: 528,
                    block_number: 9398574,
                },
                event: TransactionDepositedEvent::<DepositV0> {
                    from: address!("0x481c965E6579099F005387B4C1D7fb03bC302F4b"),
                    to: address!("0x03a858395F1a6cd22e2B4D31139794AaB58C5D4d"),
                    source_hash: b256!("0x405ed121ccc1cd47773fbe0ef8e14b8d00acf028ac83145da72e5b6d4002efcf"),
                    deposit: DepositV0 {
                        mint: 1000000000000000000u128.try_into().unwrap(),
                        value: 1000000000000000000u128.try_into().unwrap(),
                        gas_limit: 850000u64,
                        is_creation: false,
                        calldata: bytes!(""),
                    }
                },
            },
            FullEvent::<TransactionDepositedEvent<DepositV0>> {
                metadata: EventMetadata {
                    from: address!("0x17acfafcfa4A6912F97d85950F37ceEf97305393"),
                    to: address!("0x8cF3068a4a1C4f329Cc19b7c57BD4b2e7EaA3662"),
                    transaction_hash: b256!(
                        "0x653d3f9ec83c23f5e870e6d2710961a681e3bfbb280d8c19da7739146df3b6bb"
                    ),
                    block_hash: b256!(
                        "0x3466e222249e9f13be5130d4623e1fc2a5bc1c6c258c510773b50977be70f5df"
                    ),
                    index: 2,
                    block_number: 29466
                },
                event: TransactionDepositedEvent::<DepositV0> {
                    from: address!("0x3c41d8343a1cba9fd6f0356039b6c6d844610321"),
                    to: address!("0x4200000000000000000000000000000000000007"),
                    source_hash: b256!("0xecd623c316d24897147aa9de6ce21be1b6d59ad9c8bb32fcdb7386524f8d7578"),
                    deposit: DepositV0 {
                        mint: 1000000000000000u128.try_into().unwrap(),
                        value: 1000000000000000u128.try_into().unwrap(),
                        gas_limit: 414371u64,
                        is_creation: false,
                        calldata: bytes!("0xd764ad0b00010000000000000000000000000000000000000000000000000000000000070000000000000000000000008cf3068a4a1c4f329cc19b7c57bd4b2e7eaa3662000000000000000000000000420000000000000000000000000000000000001000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000a41635f5fd00000000000000000000000017acfafcfa4a6912f97d85950f37ceef973053930000000000000000000000006bbbbb6dd7b1a35aaaaaaaaff99ed8bb3666b2b500000000000000000000000000000000000000000000000000038d7ea4c680000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
                    }
                },
            },
            FullEvent::<TransactionDepositedEvent<DepositV0>> {
                metadata: EventMetadata {
                    from: address!("0x17acfafcfa4A6912F97d85950F37ceEf97305393"),
                    to: address!("0x8cF3068a4a1C4f329Cc19b7c57BD4b2e7EaA3662"),
                    transaction_hash: b256!(
                        "0xc913706ddb07d506aebab4bb006be97c02147fc5ac9a58497f17b22486dc72f3"
                    ),
                    block_hash: b256!(
                        "0xff58bc22f06613e01ef3d63ceeb24e9209ec304c7b71599f84bf804187bc2867"
                    ),
                    index: 2,
                    block_number: 29280
                },
                event: TransactionDepositedEvent::<DepositV0> {
                    from: address!("0x3c41d8343a1cba9fd6f0356039b6c6d844610321"),
                    to: address!("0x4200000000000000000000000000000000000007"),
                    source_hash: b256!("0xf44f481102697a6d757eac393b1cb0c5ce95dab86bb8ea0ee0444e8ef92efd3f"),
                    deposit: DepositV0 {
                        mint: 1000000000000000u128.try_into().unwrap(),
                        value: 1000000000000000u128.try_into().unwrap(),
                        gas_limit: 414371u64,
                        is_creation: false,
                        calldata: bytes!("0xd764ad0b00010000000000000000000000000000000000000000000000000000000000020000000000000000000000008cf3068a4a1c4f329cc19b7c57bd4b2e7eaa3662000000000000000000000000420000000000000000000000000000000000001000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000a41635f5fd00000000000000000000000017acfafcfa4a6912f97d85950f37ceef97305393000000000000000000000000000000000000322d0bbfb94a55a9bb9ead4429d800000000000000000000000000000000000000000000000000038d7ea4c680000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
                    }
                },
            },
        ]
    );
}
