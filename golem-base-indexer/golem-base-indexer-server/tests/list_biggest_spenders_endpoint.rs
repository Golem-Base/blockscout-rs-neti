mod helpers;

use anyhow::Result;
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::{
    types::{Address, BlockHash, CurrencyAmount, TxHash},
    Indexer,
};
use pretty_assertions::assert_eq;
use sea_orm::{ConnectionTrait, Statement};

use crate::helpers::assert_json::{assert_fields, assert_fields_array, assert_has_keys};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_list_biggest_spenders_endpoint() {
    let db = helpers::init_db("test", "list_biggest_spenders_endpoint").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    let sender = Address::random();

    let gas_price = 5;
    let cumulative_gas_used = 5;
    let tx_count = 5;
    let expected_small_total_fees = format!("{}", gas_price * cumulative_gas_used * tx_count);

    insert_gas_transactions(&*client, sender, gas_price, cumulative_gas_used, tx_count)
        .await
        .unwrap();
    indexer.tick().await.unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/transactions/stats/biggest-spenders?page=1&page_size=10",
    )
    .await;

    assert_has_keys(&response, &["items", "pagination"]);
    assert_eq!(response["pagination"]["total_items"], "1".to_string());
    assert_fields_array(
        &response["items"],
        vec![serde_json::json!({
            "rank": "1".to_string(),
            "address": sender.to_string(),
            "total_fees": expected_small_total_fees,
        })],
    );

    let gas_price = 1_000_000_000_100_000;
    let cumulative_gas_used = 1_000_000_000_100_000;
    let expected_total_fees =
        (CurrencyAmount::from(gas_price) * CurrencyAmount::from(cumulative_gas_used)).to_string();

    for _ in 1..=10 {
        insert_gas_transactions(
            &*client,
            Address::random(),
            gas_price,
            cumulative_gas_used,
            1,
        )
        .await
        .unwrap();
    }
    indexer.tick().await.unwrap();

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/transactions/stats/biggest-spenders?page=1&page_size=10",
    )
    .await;

    assert_eq!(response["pagination"]["total_items"], "11".to_string());
    for items in response["items"].as_array().unwrap() {
        assert_eq!(items["total_fees"].as_str().unwrap(), expected_total_fees);
    }

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/transactions/stats/biggest-spenders?page=2&page_size=10",
    )
    .await;
    assert_eq!(&response["items"][0]["total_fees"], "125");

    let response: serde_json::Value = test_server::send_get_request(
        &base,
        "/api/v1/transactions/stats/biggest-spenders?page=2&page_size=1",
    )
    .await;

    assert_fields(
        &response["pagination"],
        serde_json::json!({
            "page": "2",
            "page_size": "1",
            "total_items": "11",
            "total_pages": "11",
        }),
    );
}

async fn insert_gas_transactions<T: ConnectionTrait>(
    client: &T,
    sender: Address,
    gas_price: u64,
    cumulative_gas_used: u64,
    count: u64,
) -> Result<()> {
    for _ in 0..count {
        let tx_hash = TxHash::random();
        let block_hash = BlockHash::random();
        client.execute(Statement::from_sql_and_values(
            client.get_database_backend(),
            r#"
            INSERT INTO transactions (gas_used, gas_price, cumulative_gas_used, gas, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash)
            VALUES (100, $4, $5, 100, $1, 0, '', 0, 0, 0, 1, 0, 0, current_timestamp, current_timestamp, $2, 1, $3)
            "#,
            [
                tx_hash.as_slice().into(),
                block_hash.as_slice().into(),
                sender.as_slice().into(),
                gas_price.into(),
                cumulative_gas_used.into(),
            ],
        )).await?;
    }
    Ok(())
}
