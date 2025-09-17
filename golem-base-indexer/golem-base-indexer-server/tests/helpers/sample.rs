use alloy_rlp::encode;
use anyhow::Result;
use golem_base_indexer_logic::{
    golem_base::block_timestamp,
    types::{Address, BlockHash, BlockNumber, TxHash},
};
use golem_base_sdk::entity::EncodableGolemBaseTransaction;
use sea_orm::{ConnectionTrait, Statement};

#[derive(Default)]
pub struct Block {
    pub number: BlockNumber,
    pub hash: Option<BlockHash>,
    pub transactions: Vec<Transaction>,
}

#[derive(Default)]
pub struct Transaction {
    pub hash: Option<TxHash>,
    pub sender: Address,
    pub operations: EncodableGolemBaseTransaction,
}

pub async fn insert_data<T: ConnectionTrait>(txn: &T, block: Block) -> Result<()> {
    let block_hash = block.hash.unwrap_or_else(|| BlockHash::random());
    let block_timestamp = block_timestamp(
        block.number,
        &golem_base_indexer_logic::types::Block {
            number: 0,
            hash: Default::default(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2018-10-13T12:30:00Z")
                .unwrap()
                .to_utc(),
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
        let calldata: Vec<u8> = encode(tx.operations);
        let index = i as i64;
        txn.execute(Statement::from_sql_and_values(txn.get_database_backend(), r#"
        insert into transactions (gas_used, gas_price, cumulative_gas_used, gas, hash, index, input, nonce, r, s, status, v, value, inserted_at, updated_at, block_hash, block_number, from_address_hash, to_address_hash, block_timestamp)
        values (100, 100, 100, 100, $1, $6, $2, 0, 0, 0, 1, 0, 0, current_timestamp, current_timestamp, $3, $4, $5, '\x0000000000000000000000000000000060138453', $7)
    "#, [
                tx_hash.as_slice().into(),
                calldata.as_slice().into(),
                block_hash.as_slice().into(),
                block.number.into(),
                tx.sender.as_slice().into(),
                index.into(),
                block_timestamp.into(),
            ])).await?;
    }
    Ok(())
}
