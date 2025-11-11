use alloy_primitives::{address, Bytes};
use anyhow::Result;
use arkiv_storage_tx::StorageTransaction;
use chrono::{DateTime, Utc};
use golem_base_indexer_logic::{
    arkiv::block_timestamp,
    types::{Address, BlockHash, BlockNumber, TxHash},
};
use sea_orm::{ConnectionTrait, Statement};

#[derive(Default)]
pub struct Block {
    pub number: BlockNumber,
    pub hash: Option<BlockHash>,
    pub transactions: Vec<Transaction>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Default)]
pub struct Transaction {
    pub hash: Option<TxHash>,
    pub sender: Address,
    pub to: Option<Address>,
    pub operations: StorageTransaction,
}

pub async fn insert_data<T: ConnectionTrait>(txn: &T, block: Block) -> Result<()> {
    let block_hash = block.hash.unwrap_or_else(|| BlockHash::random());
    let block_timestamp = block_timestamp(
        block.number,
        &golem_base_indexer_logic::types::Block {
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
    txn.execute(Statement::from_sql_and_values(txn.get_database_backend(), r#"
    insert into blocks (consensus, gas_limit, gas_used, hash, miner_hash, nonce, number, parent_hash, timestamp, inserted_at, updated_at)
    values ('t', 99999, 99999, $1, '', '', $2, $3, $4, current_timestamp, current_timestamp)
"#, [
            block_hash.as_slice().into(),
            block.number.into(),
            parent_hash.as_slice().into(),
            block_timestamp.into(),
        ])).await?;
    for (i, tx) in block.transactions.into_iter().enumerate() {
        let tx_hash = tx.hash.unwrap_or_else(|| TxHash::random());
        let calldata: Bytes = tx.operations.try_into()?;
        let calldata: Vec<u8> = calldata.into();
        let index = i as i64;
        let to = tx
            .to
            .unwrap_or_else(|| address!("0x00000000000000000000000000000061726B6976"));
        txn.execute(Statement::from_sql_and_values(txn.get_database_backend(), r#"
        insert into transactions (gas_used, gas_price, cumulative_gas_used, gas, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, block_timestamp)
        values (100, 100, 100, 100, $1, $6, $2, 0, 0, 0, 1, 0, 0, current_timestamp, current_timestamp, $3, $4, $5, $8, $7)
    "#, [
                tx_hash.as_slice().into(),
                calldata.as_slice().into(),
                block_hash.as_slice().into(),
                block.number.into(),
                tx.sender.as_slice().into(),
                index.into(),
                block_timestamp.into(),
                to.as_slice().into(),
            ])).await?;
    }
    Ok(())
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
