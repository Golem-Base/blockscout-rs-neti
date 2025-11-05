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
    d.calldata
from optimism_children_transaction_deposited_events_v0 d
    inner join transactions t on t.hash = d.transaction_hash
order by
    d.block_number desc,
    t.index desc,
    d.index desc
"#;
