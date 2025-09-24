use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{
    well_known::{
        DEPOSIT_CONTRACT_ADDRESS, GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS, L1_BLOCK_CONTRACT_ADDRESS,
        L1_BLOCK_CONTRACT_SENDER_ADDRESS,
    },
    Indexer,
};
use golem_base_sdk::Address;
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_custom_contract_transactions() {
    let db = helpers::init_db("test", "list_custom_contract_transactions").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    let random_addr = Address::random();

    fn gen_tx(from: Address, to: Address) -> Transaction {
        Transaction {
            sender: from,
            to: Some(to),
            ..Default::default()
        }
    }

    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![
                // storage tx
                gen_tx(random_addr, GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS),
                // deposit tx
                gen_tx(random_addr, DEPOSIT_CONTRACT_ADDRESS),
                // housekeeping tx
                gen_tx(L1_BLOCK_CONTRACT_SENDER_ADDRESS, L1_BLOCK_CONTRACT_ADDRESS),
                // custom contract
                gen_tx(Address::random(), Address::random()),
                gen_tx(Address::random(), Address::random()),
                gen_tx(Address::random(), Address::random()),
            ],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    indexer.tick().await.unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/transactions/custom-contract?page=1&page_size=10",
    )
    .await;

    assert_eq!(response["items"].as_array().unwrap().len(), 3);
    assert_eq!(response["pagination"]["total_items"], "3".to_string());

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/transactions/custom-contract?page=2&page_size=2",
    )
    .await;

    assert_eq!(response["items"].as_array().unwrap().len(), 1);
    assert_eq!(response["pagination"]["page"], "2".to_string());
    assert_eq!(response["pagination"]["page_size"], "2".to_string());
    assert_eq!(response["pagination"]["total_items"], "3".to_string());
}
