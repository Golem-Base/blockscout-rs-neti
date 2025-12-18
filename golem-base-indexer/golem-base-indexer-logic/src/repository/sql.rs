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

pub const GET_UNPROCESSED_LOGS_EVENTS: &str = r#"
WITH pendings AS MATERIALIZED (
  SELECT
    DISTINCT transaction_hash,
    block_number
  FROM
    golem_base_pending_logs_events
  ORDER BY
    block_number ASC
  LIMIT
    100
), filtered_pending_logs AS MATERIALIZED (
  SELECT
    output.*
  FROM
    golem_base_pending_logs_events output
    INNER JOIN transactions ON transactions.hash = output.transaction_hash
    AND transactions.status = 1
  WHERE
    output.transaction_hash IN (
      SELECT
        transaction_hash
      FROM
        pendings
    )
    AND NOT EXISTS (
      SELECT
        1
      FROM
        golem_base_pending_transaction_operations
      WHERE
        hash = output.transaction_hash
    )
    AND NOT EXISTS (
      SELECT
        1
      FROM
        golem_base_pending_transaction_cleanups
      WHERE
        hash = output.transaction_hash
    )
),
logs_indexed AS MATERIALIZED (
  SELECT
    l.transaction_hash,
    l.block_hash,
    l.index,
    l.first_topic as signature_hash,
    l.data,
    ROW_NUMBER() OVER (
      PARTITION BY l.transaction_hash
      ORDER BY
        l.index
    ):: INTEGER - 1 as op_index
  FROM
    logs l
  WHERE
    l.transaction_hash IN (
      SELECT
        transaction_hash
      FROM
        pendings
    )
    AND l.address_hash = '\x00000000000000000000000000000061726b6976'
)
SELECT
  f.block_number,
  f.block_hash,
  f.transaction_hash,
  f.index,
  l.op_index,
  l.signature_hash,
  l.data
FROM
  filtered_pending_logs f
  INNER JOIN logs_indexed l ON f.transaction_hash = l.transaction_hash
  AND f.index = l.index
  AND f.block_hash = l.block_hash
ORDER BY
  f.block_number,
  f.index;
"#;

pub const GET_UNPROCESSED_LOGS: &str = r#"
select
    pendings.transaction_hash,
    pendings.block_hash,
    pendings.index
from golem_base_pending_logs_operations as pendings
    inner join transactions on transactions.hash = pendings.transaction_hash
    left join golem_base_pending_transaction_cleanups on transactions.hash = pendings.transaction_hash
where
    golem_base_pending_transaction_cleanups is null
    and transactions.status = 1
order by
    pendings.block_number asc,
    pendings.index asc
limit 100
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
order by
    pendings.block_number asc,
    pendings.index asc
limit 100
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
    (select count(*) from golem_base_operations where operation = 'create' and sender = $1) as created_entities,
    count(*) as owned_entities,
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

pub const COUNT_ENTITIES_BY_BLOCK: &str = r#"
SELECT
    COUNT(*) FILTER (WHERE operation = 'create') AS create_count,
    COUNT(*) FILTER (WHERE operation = 'update') AS update_count,
    COUNT(*) FILTER (WHERE operation = 'delete' AND recipient = '\x4200000000000000000000000000000000000015') AS expire_count,
    COUNT(*) FILTER (WHERE operation = 'delete' AND recipient != '\x4200000000000000000000000000000000000015') AS delete_count,
    COUNT(*) FILTER (WHERE operation = 'extend') AS extend_count,
    COUNT(*) FILTER (WHERE operation = 'changeowner') AS changeowner_count
FROM golem_base_operations
INNER JOIN blocks on blocks.hash = golem_base_operations.block_hash
WHERE blocks.number = $1 and blocks.consensus
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

pub const TOTAL_STORAGE_USAGE_BY_BLOCK: &str = r#"
select block_number, storage_usage from golem_base_block_stats where block_number = $1;
"#;

pub const STORAGE_DIFF_BY_BLOCK: &str = r#"
select
    blocks.number as block_number,
    sum(
        coalesce(length(data), 0)
        - coalesce(length(prev_data), 0)
    ) as storage_diff
from blocks
left join golem_base_entity_history
    on blocks.number = golem_base_entity_history.block_number
where blocks.number = any($1)
group by blocks.number
order by blocks.number;
"#;

pub const NEW_DATA_BY_BLOCK: &str = r#"
SELECT
    coalesce(sum(length(data)), 0) new_data
FROM golem_base_entity_history
WHERE
    block_number = $1
    AND operation != 'extend'
    AND operation != 'changeowner'
    AND status = 'active'
"#;

pub const BLOCK_OPERATIONS_TIMESERIES: &str = r#"
SELECT
    block_number,
    COUNT(*) FILTER (WHERE operation = 'create') AS create_count,
    COUNT(*) FILTER (WHERE operation = 'update') AS update_count,
    COUNT(*) FILTER (WHERE operation = 'delete') AS delete_count,
    COUNT(*) FILTER (WHERE operation = 'extend') AS extend_count,
    COUNT(*) FILTER (WHERE operation = 'changeowner') AS changeowner_count
FROM golem_base_operations
GROUP BY block_number
ORDER BY block_number DESC
LIMIT $1
"#;

pub const GET_ADDRESS_ACTIVITY: &str = r#"
WITH
    raw_activity_blocks as (
        SELECT
            MIN(t.block_number) AS first_seen_block,
            MAX(t.block_number) AS last_seen_block
        FROM
            internal_transactions t
        WHERE
            t.from_address_hash = $1
            OR t.to_address_hash = $1
        UNION
        SELECT
            MIN(t.block_number) AS first_seen_block,
            MAX(t.block_number) AS last_seen_block
        FROM
            transactions t
        WHERE
            t.from_address_hash = $1
            OR t.to_address_hash = $1
    ),
    activity_blocks as (
        select
            min(first_seen_block) as first_seen_block,
            max(last_seen_block) as last_seen_block
        from raw_activity_blocks
    )
select
    (select timestamp from blocks where activity_blocks.first_seen_block = number) as first_seen_timestamp,
    (select timestamp from blocks where activity_blocks.last_seen_block = number) as last_seen_timestamp,
    first_seen_block,
    last_seen_block
from activity_blocks;
"#;

pub const ENTITIES_AVERAGES: &str = r#"
SELECT
    COALESCE(AVG(LENGTH(data)), 0)::BIGINT as average_entity_size,
    COALESCE(AVG(expires_at_block_number) - (SELECT MAX(number) FROM blocks), 0)::BIGINT as average_entity_btl
FROM golem_base_entities
WHERE status = 'active';
"#;

pub const ADDRESS_LEADERBOARD_RANKS: &str = r#"
SELECT
    (SELECT rank FROM golem_base_leaderboard_biggest_spenders WHERE address = $1) AS biggest_spenders,
    (SELECT rank FROM golem_base_leaderboard_entities_created WHERE address = $1) AS entities_created,
    (SELECT rank FROM golem_base_leaderboard_entities_owned WHERE address = $1) AS entities_owned,
    (SELECT rank FROM golem_base_leaderboard_data_owned WHERE address = $1) AS data_owned,
    (SELECT rank FROM golem_base_leaderboard_top_accounts WHERE address = $1) AS top_accounts;
"#;

pub const LEADERBOARD_TOP_ACCOUNTS: &str = r#"
SELECT
    rank,
    address,
    balance,
    tx_count
FROM
    golem_base_leaderboard_top_accounts
ORDER BY
    rank ASC
"#;

pub const LEADERBOARD_BIGGEST_SPENDERS: &str = r#"
SELECT
    rank,
    address,
    total_fees
FROM
    golem_base_leaderboard_biggest_spenders
ORDER BY
    rank ASC
"#;

pub const LEADERBOARD_ENTITIES_CREATED: &str = r#"
SELECT
    rank,
    address,
    entities_created_count
FROM
    golem_base_leaderboard_entities_created
ORDER BY
    rank ASC
"#;

pub const LEADERBOARD_ENTITIES_OWNED: &str = r#"
SELECT
    rank,
    address,
    entities_count
FROM
    golem_base_leaderboard_entities_owned
ORDER BY
    rank ASC
"#;

pub const LEADERBOARD_DATA_OWNED: &str = r#"
SELECT
    rank,
    address,
    data_size
FROM
    golem_base_leaderboard_data_owned
ORDER BY
    rank ASC
"#;

pub const LEADERBOARD_LARGEST_ENTITIES: &str = r#"
SELECT
    rank,
    entity_key,
    data_size
FROM
    golem_base_leaderboard_largest_entities
ORDER BY
    rank ASC
"#;

pub const LEADERBOARD_EFFECTIVELY_LARGEST_ENTITIES: &str = r#"
SELECT
    rank,
    entity_key,
    data_size,
    lifespan
FROM
    golem_base_leaderboard_effectively_largest_entities
ORDER BY
    rank ASC
"#;

pub const QUEUE_REINDEX_PREFIX: &str = r#"
insert into golem_base_entities_to_reindex (key) values
"#;

pub const GET_ENTITIES_TO_REINDEX: &str = r#"
select distinct key from golem_base_entities_to_reindex
"#;

pub const FINISH_REINDEX: &str = r#"
delete from golem_base_entities_to_reindex where key = $1
"#;

pub const OLDEST_UNPROCESSED_BLOCK_STATS: &str = r#"
select min(blocks.number) as block_number
from blocks
left join golem_base_block_stats stats
    on blocks.number = stats.block_number
where stats.block_number is null or stats.is_dirty = true
"#;

pub const UPDATE_BLOCK_STATS_PREFIX: &str = r#"
insert into golem_base_block_stats (block_number, storage_usage) values
"#;

pub const UPDATE_BLOCK_STATS_SUFFIX: &str = r#"
on conflict (block_number) do update set is_dirty = false, storage_usage = EXCLUDED.storage_usage;
"#;

pub const MARK_STATS_DIRTY: &str = r#"
update golem_base_block_stats set is_dirty = true where block_number >= $1;
"#;
