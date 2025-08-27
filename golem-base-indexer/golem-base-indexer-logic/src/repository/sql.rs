pub const GET_LOGS: &str = r#"
select
    data,
    index,
    first_topic,
    second_topic,
    third_topic,
    fourth_topic,
    transaction_hash
from logs
where
    transaction_hash = $1
    and first_topic = $2
order by index asc
"#;

pub const GET_LATEST_UPDATE_OPERATION_INDEX: &str = r#"
select
    transactions.block_number as block_number,
    transactions.index as transaction_index,
    golem_base_operations.index as operation_index
from golem_base_operations
inner join transactions
    on golem_base_operations.transaction_hash = transactions.hash
where
    golem_base_operations.entity_key = $1
    and transactions.block_number is not null
order by
    transactions.block_number desc,
    transactions.index desc,
    golem_base_operations.index desc
limit 1
"#;

pub const GET_UNPROCESSED_TX_HASHES: &str = r#"
select hash
from golem_base_pending_transaction_operations as pendings
inner join transactions using (hash)
left join golem_base_pending_transaction_cleanups using (hash)
where
    golem_base_pending_transaction_cleanups is null
    and transactions.to_address_hash in ($1, $2) 
    and transactions.status = 1
    and transactions.block_hash is not null
"#;

pub const GET_TX_BY_HASH: &str = r#"
select 
    from_address_hash,
    to_address_hash,
    hash,
    block_number,
    block_hash,
    index,
    input
from transactions
where
    hash = $1
"#;

pub const FIND_ENTITIES_BY_TX_HASH: &str = r#"
select 
    e.*,
    e.status::text as status
from golem_base_entities as e
where
    exists (
        select 1
        from golem_base_operations as o
        where
            o.entity_key = e.key
            and o.transaction_hash = $1
    )
"#;

pub const FIND_LATEST_UPDATE_OPERATION: &str = r#"
select 
    o.*,
    o.operation::text as operation
from golem_base_operations o
inner join transactions t
    on t.hash = o.transaction_hash
where
    o.operation = 'update'
    and o.entity_key = $1
order by
    t.block_number desc,
    t.index desc,
    o.index desc
limit 1;
"#;

pub const FIND_LATEST_LOG: &str = r#"
select
    data,
    logs.index,
    first_topic,
    second_topic,
    third_topic,
    fourth_topic,
    transaction_hash
from logs
inner join transactions on transactions.hash = logs.transaction_hash
where
    first_topic = $1
    and second_topic = $2
order by
    transactions.block_number desc,
    transactions.index desc,
    logs.index desc
"#;

pub const FIND_LATEST_OPERATION: &str = r#"
select 
    o.*,
    o.operation::text as operation
from golem_base_operations o
inner join transactions t
    on t.hash = o.transaction_hash
where
    o.entity_key = $1
order by
    t.block_number desc,
    t.index desc,
    o.index desc
limit 1;
"#;

pub const COUNT_ENTITIES_BY_OWNER: &str = r#"
select
    count(*) as total_entities,
    count(*) filter (where status = 'active') as active_entities,
    coalesce(sum(length(data)) filter (where status = 'active'), 0) as size_of_active_entities
from golem_base_entities
where owner = $1
"#;

pub const COUNT_TRANSACTIONS_BY_OWNER: &str = r#"
select
    count(*) as total_transactions,
    count(*) filter (where status = 0) as failed_transactions
from transactions
where
    from_address_hash = $1
    and block_consensus = 't'
"#;

pub const FIND_TX_FEE_BIGGEST_SPENDERS: &str = r#"
SELECT 
    ROW_NUMBER() OVER(ORDER BY SUM(cumulative_gas_used * gas_price) DESC) as rank,
    from_address_hash as address, 
    CAST(SUM(cumulative_gas_used * gas_price) AS TEXT) as total_fees
FROM 
    transactions
WHERE
    cumulative_gas_used IS NOT NULL
    AND cumulative_gas_used > 0
    AND gas_price IS NOT NULL
    AND gas_price > 0
GROUP BY 
    from_address_hash
ORDER BY 
    SUM(cumulative_gas_used * gas_price) DESC
"#;
