use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{types::TxHash, Indexer};
use golem_base_sdk::{
    entity::{Create, EncodableGolemBaseTransaction},
    Address,
};
use helpers::{
    sample::{insert_data, insert_gas_transactions, Block, Transaction},
    utils::refresh_leaderboards,
};
use serde_json::{json, Value};
use std::sync::Arc;
use test_server::send_get_request;

fn endpoint_for_address(address: &Address) -> String {
    format!(
        "/api/v1/address/{}/leaderboard-ranks",
        address.to_checksum(None)
    )
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn address_leaderboard_ranks_should_work() {
    // Setup
    let db = helpers::init_db("test", "address_leaderboard_ranks_should_work").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let indexer = Indexer::new(Arc::clone(&client), Default::default());
    let address1 = Address::random();
    let address2 = Address::random();
    let address3 = Address::random();
    let address4 = Address::random();

    // Address1 is the biggest spender
    insert_gas_transactions(&*client, address1, 1_000_000_000_000_000_000, 1, 1)
        .await
        .unwrap();
    let creates = vec![Create::new(vec![], 1000)]; // A single empty entity with a BTL of 1000
    let block = Block {
        number: 1,
        transactions: vec![Transaction {
            sender: address1,
            hash: Some(TxHash::random()),
            operations: EncodableGolemBaseTransaction {
                creates,
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    insert_data(&*client, block).await.unwrap();

    // Address2 has the most entities created and owned
    insert_gas_transactions(&*client, address2, 1_000_000_000_000_000, 1, 1)
        .await
        .unwrap();
    let creates = vec![Create::new(vec![0; 4], 1); 100]; // A hundred of 4-byte entities with a BTL of 1
    let block = Block {
        number: 2,
        transactions: vec![Transaction {
            sender: address2,
            hash: Some(TxHash::random()),
            operations: EncodableGolemBaseTransaction {
                creates,
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    insert_data(&*client, block).await.unwrap();

    // Address3 is not going to lead any category
    insert_gas_transactions(&*client, address3, 1_000_000_000_000, 1, 1)
        .await
        .unwrap();
    let creates = vec![Create::new(vec![0; 16], 4); 10]; // Ten 16-byte entities with BTL of 4
    let block = Block {
        number: 3,
        transactions: vec![Transaction {
            sender: address3,
            hash: Some(TxHash::random()),
            operations: EncodableGolemBaseTransaction {
                creates,
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    insert_data(&*client, block).await.unwrap();

    // Address4 has the largest amount of data stored
    insert_gas_transactions(&*client, address4, 1_000_000_000, 1, 1)
        .await
        .unwrap();
    let creates = vec![Create::new(vec![0xff; 32768], 256); 2]; // Two 32-kilobyte entities with a
                                                                // BTL of 256
    let block = Block {
        number: 4,
        transactions: vec![Transaction {
            sender: address4,
            hash: Some(TxHash::random()),
            operations: EncodableGolemBaseTransaction {
                creates,
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    insert_data(&*client, block).await.unwrap();

    // Process and refresh leaderboards
    indexer.tick().await.unwrap();
    refresh_leaderboards(Arc::clone(&client)).await.unwrap();

    // Check Address1 ranks
    let expected = json!({
        "biggest_spenders": "1",
        "entities_created": "4",
        "entities_owned": "4",
        "data_owned": "4",
    });
    let response: Value = send_get_request(&base, &endpoint_for_address(&address1)).await;
    assert_eq!(response, expected);

    // Check Address2 ranks
    let expected = json!({
        "biggest_spenders": "2",
        "entities_created": "1",
        "entities_owned": "1",
        "data_owned": "2",
    });
    let response: Value = send_get_request(&base, &endpoint_for_address(&address2)).await;
    assert_eq!(response, expected);

    // Check Address3 ranks
    let expected = json!({
        "biggest_spenders": "3",
        "entities_created": "2",
        "entities_owned": "2",
        "data_owned": "3",
    });
    let response: Value = send_get_request(&base, &endpoint_for_address(&address3)).await;
    assert_eq!(response, expected);

    // Check Address4 ranks
    let expected = json!({
        "biggest_spenders": "4",
        "entities_created": "3",
        "entities_owned": "3",
        "data_owned": "1",
    });
    let response: Value = send_get_request(&base, &endpoint_for_address(&address4)).await;
    assert_eq!(response, expected);
}

#[tokio::test]
#[ignore = "Needs database to run"]
async fn address_leaderboard_ranks_should_return_zeros_for_non_indexed_address() {
    // Setup
    let db = helpers::init_db(
        "test",
        "address_leaderboard_ranks_should_return_zeros_for_non_indexed_address",
    )
    .await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    // Process
    Indexer::new(Arc::clone(&client), Default::default())
        .tick()
        .await
        .unwrap();

    let address = Address::random();
    let expected = json!({
        "biggest_spenders": "0",
        "entities_created": "0",
        "entities_owned": "0",
        "data_owned": "0",
    });

    // Non-indexed address should return zeros
    let response: Value =
        test_server::send_get_request(&base, &endpoint_for_address(&address)).await;
    assert_eq!(response, expected);

    // After updating leaderboards it should still return zeros
    refresh_leaderboards(Arc::clone(&client)).await.unwrap();

    let response: Value =
        test_server::send_get_request(&base, &endpoint_for_address(&address)).await;
    assert_eq!(response, expected);
}
