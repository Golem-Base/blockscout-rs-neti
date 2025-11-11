use crate::helpers;

use alloy_primitives::Address;
use arkiv_storage_tx::{StorageTransaction, Update};
use blockscout_service_launcher::test_server;
use bytes::Bytes;
use golem_base_indexer_logic::{mat_view_scheduler::MatViewScheduler, types::EntityKey, Indexer};

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_get_entity_data_size_histogram() {
    let db = helpers::init_db("test", "get_entity_data_size_histogram").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    let indexer = Indexer::new(client.clone(), Default::default());
    let scheduler = MatViewScheduler::new(client.clone());

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/chart/entity-data-histogram").await;

    assert_eq!(response["items"].as_array().unwrap().len(), 10);
    assert_eq!(
        response["items"],
        serde_json::json!(
            [
                {
                    "bucket": "1",
                    "bin_start": "0",
                    "bin_end": "1",
                    "count": "0"
                },
                {
                    "bucket": "2",
                    "bin_start": "1",
                    "bin_end": "2",
                    "count": "0"
                },
                {
                    "bucket": "3",
                    "bin_start": "2",
                    "bin_end": "3",
                    "count": "0"
                },
                {
                    "bucket": "4",
                    "bin_start": "3",
                    "bin_end": "4",
                    "count": "0"
                },
                {
                    "bucket": "5",
                    "bin_start": "4",
                    "bin_end": "5",
                    "count": "0"
                },
                {
                    "bucket": "6",
                    "bin_start": "5",
                    "bin_end": "6",
                    "count": "0"
                },
                {
                    "bucket": "7",
                    "bin_start": "6",
                    "bin_end": "7",
                    "count": "0"
                },
                {
                    "bucket": "8",
                    "bin_start": "7",
                    "bin_end": "8",
                    "count": "0"
                },
                {
                    "bucket": "9",
                    "bin_start": "8",
                    "bin_end": "9",
                    "count": "0"
                },
                {
                    "bucket": "10",
                    "bin_start": "9",
                    "bin_end": "10",
                    "count": "0"
                }
            ]
        )
    );

    fn gen_tx(bytes_len: u64) -> Transaction {
        gen_tx_with_key(bytes_len, EntityKey::random())
    }
    fn gen_tx_with_key(bytes_len: u64, entity_key: EntityKey) -> Transaction {
        Transaction {
            sender: Address::random(),
            operations: StorageTransaction {
                updates: vec![Update {
                    entity_key,
                    btl: 100,
                    payload: Bytes::from(vec![0u8; bytes_len as usize]),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    let ek1 = EntityKey::random();
    let ek2 = EntityKey::random();

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![
                gen_tx(0),
                gen_tx(0),
                gen_tx(10),
                gen_tx(10),
                gen_tx_with_key(10, ek1),
                gen_tx(1000),
                gen_tx(2000),
                gen_tx(2000),
                gen_tx(2001),
                gen_tx(3000),
                gen_tx(4000),
                gen_tx(5000),
                gen_tx(6000),
                gen_tx(7000),
                gen_tx(8000),
                gen_tx(9000),
                gen_tx(10000),
                gen_tx(10000),
                gen_tx_with_key(10000, ek2),
            ],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 2,
            transactions: vec![Transaction {
                sender: Address::random(),
                operations: StorageTransaction {
                    deletes: vec![ek1, ek2],
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    indexer.tick().await.unwrap();
    scheduler
        .refresh_named_view("golem_base_entity_data_size_histogram")
        .await;

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/chart/entity-data-histogram").await;

    assert_eq!(response["items"].as_array().unwrap().len(), 10);
    assert_eq!(
        response["items"],
        serde_json::json!(
            [
                {
                    "bucket": "1",
                    "bin_start": "0",
                    "bin_end": "1000",
                    "count": "5"
                },
                {
                    "bucket": "2",
                    "bin_start": "1001",
                    "bin_end": "2001",
                    "count": "3"
                },
                {
                    "bucket": "3",
                    "bin_start": "2002",
                    "bin_end": "3002",
                    "count": "1"
                },
                {
                    "bucket": "4",
                    "bin_start": "3003",
                    "bin_end": "4003",
                    "count": "1"
                },
                {
                    "bucket": "5",
                    "bin_start": "4004",
                    "bin_end": "5004",
                    "count": "1"
                },
                {
                    "bucket": "6",
                    "bin_start": "5005",
                    "bin_end": "6005",
                    "count": "1"
                },
                {
                    "bucket": "7",
                    "bin_start": "6006",
                    "bin_end": "7006",
                    "count": "1"
                },
                {
                    "bucket": "8",
                    "bin_start": "7007",
                    "bin_end": "8007",
                    "count": "1"
                },
                {
                    "bucket": "9",
                    "bin_start": "8008",
                    "bin_end": "9008",
                    "count": "1"
                },
                {
                    "bucket": "10",
                    "bin_start": "9009",
                    "bin_end": "10000",
                    "count": "2"
                }
            ]
        )
    );

    let sum: u64 = response["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["count"].as_str().unwrap().parse::<u64>().unwrap())
        .sum();
    assert_eq!(sum, 17);
}
