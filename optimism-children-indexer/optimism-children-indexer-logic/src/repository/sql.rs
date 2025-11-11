pub const GET_UNPROCESSED_LOGS: &str = r#"
select
    pendings.transaction_hash,
    pendings.block_hash,
    pendings.index
from optimism_children_pending_logs as pendings
order by
    pendings.block_number asc,
    pendings.index asc
"#;

pub const GET_TX_BY_HASH: &str = r#"
select 
    t.from_address_hash,
    t.to_address_hash,
    t.hash,
    t.block_number,
    t.block_hash,
    b.timestamp as block_timestamp,
    t.index,
    t.input
from transactions t
    inner join blocks b on t.block_hash = b.hash
where
    t.hash = $1
"#;

pub const LIST_DEPOSITS_WITH_TX: &str = r#"
select 
    t.from_address_hash as tx_from,
    t.to_address_hash as tx_to,
    d.transaction_hash as tx_hash,
    d.block_hash,
    d.block_number,
    d.index,
    d.from as deposit_from,
    d.to as deposit_to,
    d.source_hash,
    d.mint,
    d.value,
    d.gas_limit,
    d.is_creation,
    d.calldata,
    l3d.chain_id as chain_id,
    l3d.block_hash as execution_tx_block_hash,
    l3d.block_number as execution_tx_block_number,
    l3d.to as execution_tx_to,
    l3d.from as execution_tx_from,
    l3d.tx_hash as execution_tx_hash,
    l3d.success as execution_tx_success
from optimism_children_transaction_deposited_events_v0 d
    inner join transactions t on t.hash = d.transaction_hash
    left join optimism_children_l3_deposits l3d on l3d.source_hash = d.source_hash
order by
    d.block_number desc,
    t.index desc,
    d.index desc
"#;
