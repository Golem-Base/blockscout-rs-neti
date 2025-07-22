pub const GET_LOGS: &str = r#"
select
    data,
    index,
    first_topic,
    second_topic,
    third_topic,
    fourth_topic
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
select transactions.hash
from transactions
left join golem_base_operations
    on transactions.hash = golem_base_operations.transaction_hash
where
    golem_base_operations.transaction_hash is null
    and transactions.to_address_hash in ($1, $2) 
    and transactions.status = 1
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
