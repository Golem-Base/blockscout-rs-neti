use crate::{
    arkiv::block_timestamp,
    types::{Address, BlockHash, BlockNumber, TxHash},
};
use alloy_primitives::{address, Bytes};
use anyhow::Result;
use arkiv_storage_tx::StorageTransaction;
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, Statement, Value};

#[derive(Default, Clone)]
pub struct Block {
    pub number: BlockNumber,
    pub hash: Option<BlockHash>,
    pub transactions: Vec<Transaction>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Default, Clone)]
pub struct Transaction {
    pub hash: Option<TxHash>,
    pub sender: Address,
    pub to: Option<Address>,
    pub operations: StorageTransaction,
}

pub async fn insert_data_multi<T: ConnectionTrait>(txn: &T, blocks: Vec<Block>) -> Result<()> {
    let results = blocks
        .into_iter()
        .map(|block| {
            let mut blocks_params = vec![];
            let mut txs_params = vec![];
            let block_hash = block.hash.unwrap_or_else(|| BlockHash::random());
            let block_timestamp = block_timestamp(
                block.number,
                &crate::types::Block {
                    number: 0,
                    hash: Default::default(),
                    timestamp: block.timestamp.unwrap_or(
                        chrono::DateTime::parse_from_rfc3339("2018-10-13T12:30:00Z")
                            .unwrap()
                            .to_utc(),
                    ),
                },
            );
            let parent_hash = BlockHash::random();
            blocks_params.push((block_hash, block.number, parent_hash, block_timestamp));
            for (i, tx) in block.transactions.into_iter().enumerate() {
                let tx_hash: Vec<u8> = tx
                    .hash
                    .unwrap_or_else(|| TxHash::random())
                    .as_slice()
                    .into();
                let calldata: Bytes = tx.operations.try_into().unwrap();
                let calldata: Vec<u8> = calldata.into();
                let index = i as i64;
                let to = tx
                    .to
                    .unwrap_or_else(|| address!("0x00000000000000000000000000000061726B6976"));
                txs_params.push((
                    tx_hash,
                    index,
                    calldata,
                    block_hash,
                    block.number,
                    tx.sender,
                    to,
                    block_timestamp,
                ));
            }
            (blocks_params, txs_params)
        })
        .collect::<Vec<_>>();
    let blocks_params = results.iter().flat_map(|v| v.0.clone()).collect::<Vec<_>>();
    let txs_params = results.iter().flat_map(|v| v.1.clone()).collect::<Vec<_>>();

    let prefix = "insert into blocks (consensus, gas_limit, gas_used, hash, miner_hash, nonce, number, parent_hash, timestamp, inserted_at, updated_at) values ";
    let values = blocks_params
        .iter()
        .enumerate()
        .map(|(i, _)| {
            format!(
            "('t', 99999, 99999, ${}, '', '', ${}, ${}, ${}, current_timestamp, current_timestamp)",
            4 * i + 1,
            4 * i + 2,
            4 * i + 3,
            4 * i + 4
        )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params = blocks_params
        .iter()
        .fold(Vec::<Value>::new(), |mut acc, v| {
            acc.push(v.0.as_slice().into());
            acc.push(v.1.into());
            acc.push(v.2.as_slice().into());
            acc.push(v.3.into());
            acc
        });
    txn.execute(Statement::from_sql_and_values(
        txn.get_database_backend(),
        format!("{prefix}{values}"),
        params,
    ))
    .await?;

    let prefix = "insert into transactions (gas_used, gas_price, cumulative_gas_used, gas, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, block_timestamp) values ";
    let values = txs_params
        .iter()
        .enumerate()
        .map(|(i, _)| {
            format!(
            "(100, 100, 100, 100, ${}, ${}, ${}, 0, 0, 0, 1, 0, 0, current_timestamp, current_timestamp, ${}, ${}, ${}, ${}, ${})",
            8 * i + 1,
            8 * i + 2,
            8 * i + 3,
            8 * i + 4,
            8 * i + 5,
            8 * i + 6,
            8 * i + 7,
            8 * i + 8,
        )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params = txs_params.iter().fold(Vec::<Value>::new(), |mut acc, v| {
        acc.push(v.0.as_slice().into());
        acc.push(v.1.into());
        acc.push(v.2.as_slice().into());
        acc.push(v.3.as_slice().into());
        acc.push(v.4.into());
        acc.push(v.5.as_slice().into());
        acc.push(v.6.as_slice().into());
        acc.push(v.7.into());
        acc
    });
    txn.execute(Statement::from_sql_and_values(
        txn.get_database_backend(),
        format!("{prefix}{values}"),
        params,
    ))
    .await?;

    Ok(())
}

pub async fn insert_data<T: ConnectionTrait>(txn: &T, block: Block) -> Result<()> {
    insert_data_multi(txn, vec![block]).await
}

pub async fn insert_gas_transactions<T: ConnectionTrait>(
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
