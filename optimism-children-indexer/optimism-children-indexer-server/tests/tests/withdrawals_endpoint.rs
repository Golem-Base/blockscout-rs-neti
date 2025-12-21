use crate::helpers;

use blockscout_service_launcher::test_server;
use optimism_children_indexer_logic::Indexer;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_withdrawals_endpoint() {
    // Setup
    let db = helpers::init_db("test", "test_withdrawals_endpoint").await;
    let client = db.client();
    let base = helpers::init_optimism_children_indexer_server(db, |x| x).await;
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

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/withdrawals").await;

    let expected = json!({
      "items": [
        {
          "chain_id": "21377321",
          "l3_block_number": "123128",
          "l3_block_hash": "0x48079624a0b115a086742a6ab17ec5849c73d7b67264409e8b11e2c624622cc1",
          "l3_tx_hash": "0xf40aa108279a01cd10c4ee1aa07099733f746b6cb6b18b9342d5d603ae7eec4c",
          "nonce": "1766847064778384329583297500742918515827483896875618958121606201292619792",
          "sender": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
          "target": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
          "value": "100000000000000",
          "gas_limit": "100000",
          "data": "0x",
          "withdrawal_hash": "0x5710ecaa2f28c84097517d9e91ddd9e40717b7cb9ea9a4781fc93b9dc9a8306f",
          "proving_tx": null,
          "finalizing_tx": null
        },
        {
          "chain_id": "60138453025",
          "l3_block_number": "1734398",
          "l3_block_hash": "0x3d275c76cfcc2b73600f455f42fc27bbebbdd51752f800d451cbbad028677082",
          "l3_tx_hash": "0xff9b299fabddbaa12a49c04e6a6088c69c10b93e64d5718d0b0e8b92659a3652",
          "nonce": "1766847064778384329583297500742918515827483896875618958121606201292619790",
          "sender": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
          "target": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
          "value": "100000000000000",
          "gas_limit": "100000",
          "data": "0x",
          "withdrawal_hash": "0x6205f1d611864e90818bfc25cb2de88d17d82973d0a1987bf0b2b934144f26b5",
          "proving_tx": {
            "metadata": {
              "transaction_hash": "0xe079e741dc364041092233e5029e130fe57f72fed0f0c6230972bc0a25c30b09",
              "block_hash": "0xb8f6e04c68cf33dd5348e28f05e05e830c072ef55f59c0f22cfd084d3d3e2e7d",
              "block_number": "1716600",
              "from": "0xF6c95fc7E9a0C1D2E3F4A5b6c7D8E9F0A1B2c3d4",
              "to": "0x4C3F68ab087ef1C74B422c0BEB6be37Eee944A1F",
              "success": true
            },
            "event": {
              "withdrawal_hash": "0x6205f1d611864e90818bfc25cb2de88d17d82973d0a1987bf0b2b934144f26b5",
              "from": "0xCc1D9565e0F8B9C8f6A7f8e9f1a2b3c4d5E6f7A8",
              "to": "0xF6c95fc7E9a0C1D2E3F4A5b6c7D8E9F0A1B2c3d4"
            }
          },
          "finalizing_tx": null
        },
        {
          "chain_id": "60138453025",
          "l3_block_number": "1698058",
          "l3_block_hash": "0x7395f9d20e8e32c68e1257904d0dd950ebe0967b257df4f48440ee2fbc6e9023",
          "l3_tx_hash": "0x91d89ac0e0d32971c8ed3de96d934215a64bcd1c59d2998f495279e708c1eaaa",
          "nonce": "1766847064778384329583297500742918515827483896875618958121606201292619789",
          "sender": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
          "target": "0x85193a5EcCe8f40Fea01B14dd9FDF56E5D3369f6",
          "value": "1000000000000000",
          "gas_limit": "100000",
          "data": "0x",
          "withdrawal_hash": "0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806",
          "proving_tx": {
            "metadata": {
              "transaction_hash": "0xd968d630cb253930981122d4918d12ffc46e61edcf9fb512f8617ba9914b2fa8",
              "block_hash": "0xa7e5d93b57bf22cc4237f17f94d94d719b961de44e48bfe11bedef73c2c1d6c5",
              "block_number": "1716500",
              "from": "0xE5b84fC6E8c9b3db25a58E0a1D9287e0de4bC96a",
              "to": "0x4C3F68ab087ef1C74B422c0BEB6be37Eee944A1F",
              "success": true
            },
            "event": {
              "withdrawal_hash": "0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806",
              "from": "0xbb0c8454e9d220d835C594E630731CbBd396Ae14",
              "to": "0xE5b84fC6E8c9b3db25a58E0a1D9287e0de4bC96a"
            }
          },
          "finalizing_tx": {
            "metadata": {
              "transaction_hash": "0xc857c519ba142819870011c3817c11feb35d50dcb8eb401f7506aa9803a1e9e7",
              "block_hash": "0x96f4e92a46af11bb3126f36f06f83c608a850cf33d37afd00adcfe62b1b0c5b4",
              "block_number": "1716393",
              "from": "0xd4A77D618b9F512960Fa21714361341E129CAe6B",
              "to": "0x4C3F68ab087ef1C74B422c0BEB6be37Eee944A1F",
              "success": true
            },
            "event": {
              "withdrawal_hash": "0xc910628eb139dff2e031ea22334629dc574750f9f2d68ddf42c789b211e03806",
              "success": true
            }
          }
        }
      ],
      "pagination": {
        "page": "1",
        "page_size": "100",
        "total_pages": "1",
        "total_items": "3"
      },
      "next_page_params": null
    });

    assert_eq!(response, expected);
}
