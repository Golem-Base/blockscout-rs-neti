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

pub const COUNT_ENTITIES_BY_BLOCK: &str = r#"
SELECT
    COUNT(*) FILTER (WHERE operation = 'create') AS create_count,
    COUNT(*) FILTER (WHERE operation = 'update') AS update_count,
    COUNT(*) FILTER (WHERE operation = 'delete' AND status = 'expired') AS expire_count,
    COUNT(*) FILTER (WHERE operation = 'delete' AND status = 'deleted') AS delete_count,
    COUNT(*) FILTER (WHERE operation = 'extend') AS extend_count
FROM golem_base_entity_history
WHERE block_number = $1
"#;

pub const GET_STRING_ANNOTATIONS_WITH_RELATIONS: &str = r#"
select
    a.key,
    a.value,
    count(*) as related_entities
from golem_base_string_annotations as a
join golem_base_string_annotations as related using (key, value)
where
    a.active = 't'
    and related.active = 't'
    and a.entity_key = $1
group by key, value
"#;

pub const GET_NUMERIC_ANNOTATIONS_WITH_RELATIONS: &str = r#"
select
    a.key,
    a.value,
    count(*) as related_entities
from golem_base_numeric_annotations as a
join golem_base_numeric_annotations as related using (key, value)
where
    a.active = 't'
    and related.active = 't'
    and a.entity_key = $1
group by key, value
"#;

pub const LIST_ADDRESS_BY_ENTITIES_OWNED: &str = r#"
SELECT
    owner as address,
    COUNT(*) AS entities_count
FROM 
    golem_base_entities
WHERE 
    owner IS NOT NULL
    AND status = 'active'
GROUP BY 
    owner
ORDER BY 
    entities_count DESC
"#;

pub const STORAGE_USAGE_BY_BLOCK: &str = r#"
WITH latest_entities_per_block AS (
  SELECT
    block_number,
    data,
    status,
    entity_key,
    ROW_NUMBER() OVER (PARTITION BY entity_key ORDER BY block_number DESC) as rn
  FROM golem_base_entity_history
  WHERE block_number <= $1
),
current_state AS (
  SELECT
    block_number,
    data,
    status,
    entity_key
  FROM latest_entities_per_block
  WHERE rn = 1 AND status = 'active'
),
block_metrics AS (
  SELECT
    $1 as block_number,
    -- Storage added in this specific block
    COALESCE(SUM(CASE WHEN block_number = $1 THEN LENGTH(data) END), 0) as block_bytes,
    -- Total storage up to and including this block
    COALESCE(SUM(LENGTH(data)), 0) as total_bytes
  FROM current_state
)
SELECT
  block_number,
  block_bytes,
  total_bytes
FROM block_metrics;
"#;
