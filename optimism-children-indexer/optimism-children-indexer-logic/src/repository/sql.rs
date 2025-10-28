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
