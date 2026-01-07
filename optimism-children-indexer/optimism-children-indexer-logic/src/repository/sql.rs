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
    d.block_timestamp,
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

pub const LIST_WITHDRAWALS_WITH_TX: &str = r#"
SELECT
    -- L3 withdrawal information (from MessagePassed event)
    w.chain_id,
    w.block_number AS l3_block_number,
    w.block_hash AS l3_block_hash,
    w.block_timestamp AS l3_block_timestamp,
    w.tx_hash AS l3_tx_hash,
    w.nonce,
    w.sender,
    w.target,
    w.value,
    w.gas_limit,
    w.data,
    w.withdrawal_hash,

    -- L2 WithdrawalProven event information
    wp.transaction_hash AS proven_tx_hash,
    wp.block_hash AS proven_block_hash,
    wp.block_number AS proven_block_number,
    wp.block_timestamp AS proven_block_timestamp,
    wp.index AS proven_log_index,
    wp.from AS proven_from,
    wp.to AS proven_to,
    t_proven.from_address_hash AS proven_tx_from,
    t_proven.to_address_hash AS proven_tx_to,

    -- L2 WithdrawalFinalized event information
    wf.transaction_hash AS finalized_tx_hash,
    wf.block_hash AS finalized_block_hash,
    wf.block_number AS finalized_block_number,
    wf.block_timestamp AS finalized_block_timestamp,
    wf.index AS finalized_log_index,
    wf.success AS finalized_success,
    t_finalized.from_address_hash AS finalized_tx_from,
    t_finalized.to_address_hash AS finalized_tx_to
FROM optimism_children_l3_withdrawals w
    LEFT JOIN optimism_children_withdrawal_proven_events wp
        ON wp.withdrawal_hash = w.withdrawal_hash
    LEFT JOIN transactions t_proven
        ON t_proven.hash = wp.transaction_hash
    LEFT JOIN optimism_children_withdrawal_finalized_events wf
        ON wf.withdrawal_hash = w.withdrawal_hash
    LEFT JOIN transactions t_finalized
        ON t_finalized.hash = wf.transaction_hash
ORDER BY
    w.chain_id ASC,
    w.block_number DESC,
    w.id DESC
"#;
