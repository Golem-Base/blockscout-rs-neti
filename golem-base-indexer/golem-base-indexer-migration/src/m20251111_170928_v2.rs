use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_table("transactions").await? {
            return Err(DbErr::Migration(
                "Table transactions does not exist in the database".to_string(),
            ));
        }
        let create_types: Vec<&str> = r#"
CREATE TYPE golem_base_entity_status_type AS ENUM (
    'active',
    'deleted',
    'expired'
);

CREATE TYPE golem_base_operation_type AS ENUM (
    'create',
    'update',
    'delete',
    'extend',
    'changeowner'
);
        "#
        .split(';')
        .collect();

        let create_functions = vec![
            r#"
CREATE FUNCTION golem_base_queue_logs_processing() RETURNS trigger
    language plpgsql
AS $$
declare
    v_address_hash bytea;
begin
    select to_address_hash into v_address_hash from transactions where hash = new.transaction_hash;
    if v_address_hash = '\x4200000000000000000000000000000000000015' then
        insert into golem_base_pending_logs_operations (transaction_hash, block_hash, index, block_number)
            values (new.transaction_hash, new.block_hash, new.index, new.block_number) on conflict do nothing;
    end if;
    return new;
end;
$$;
"#,
            r#"
CREATE FUNCTION golem_base_queue_transaction_cleanup() RETURNS trigger
    language plpgsql
AS $$
begin
    insert into golem_base_pending_transaction_cleanups (hash) values (new.hash);
    return new;
end;
$$;
"#,
            r#"
CREATE FUNCTION golem_base_queue_transaction_processing() RETURNS trigger
    language plpgsql
AS $$
begin
    insert into golem_base_pending_transaction_operations (hash, block_number, index) values (new.hash, new.block_number, new.index) on conflict do nothing;
    return new;
end;
$$;
"#,
        ];

        let create_tables = r#"
CREATE TABLE golem_base_entities (
    key bytea NOT NULL primary key,
    data bytea,
    status golem_base_entity_status_type NOT NULL,
    owner bytea,
    created_at_tx_hash bytea,
    last_updated_at_tx_hash bytea NOT NULL,
    expires_at_block_number bigint,
    inserted_at timestamp without time zone DEFAULT now() NOT NULL,
    updated_at timestamp without time zone DEFAULT now() NOT NULL
);

CREATE TABLE golem_base_entity_locks (
    key bytea NOT NULL primary key
);

CREATE TABLE golem_base_operations (
    entity_key bytea NOT NULL,
    sender bytea NOT NULL,
    recipient bytea NOT NULL,
    operation golem_base_operation_type NOT NULL,
    data bytea,
    btl numeric(21,0),
    new_owner bytea,
    block_hash bytea NOT NULL references blocks(hash),
    transaction_hash bytea NOT NULL references transactions(hash),
    index bigint NOT NULL,
    inserted_at timestamp without time zone DEFAULT now() NOT NULL,
    block_number bigint NOT NULL,
    tx_index integer NOT NULL,

    primary key (transaction_hash, index),

    CONSTRAINT golem_base_operations_check_create CHECK (((operation <> 'create'::golem_base_operation_type) OR (operation <> 'update'::golem_base_operation_type) OR ((data IS NOT NULL) AND (btl IS NOT NULL) AND (new_owner IS NULL)))),
    CONSTRAINT golem_base_operations_check_delete CHECK (((operation <> 'delete'::golem_base_operation_type) OR ((data IS NULL) AND (btl IS NULL) AND (new_owner IS NULL)))),
    CONSTRAINT golem_base_operations_check_extend CHECK (((operation <> 'extend'::golem_base_operation_type) OR ((data IS NULL) AND (btl IS NOT NULL) AND (new_owner IS NULL)))),
    CONSTRAINT golem_base_operations_check_changeowner CHECK (((operation <> 'changeowner'::golem_base_operation_type) OR ((data IS NULL) AND (btl IS NULL) AND (new_owner IS NOT NULL))))
);

CREATE TABLE golem_base_entity_history (
    entity_key bytea NOT NULL,
    block_number bigint NOT NULL,
    block_hash bytea NOT NULL references blocks(hash),
    transaction_hash bytea NOT NULL references transactions(hash),
    tx_index integer NOT NULL,
    op_index bigint NOT NULL,
    block_timestamp timestamp without time zone NOT NULL,
    owner bytea,
    prev_owner bytea,
    sender bytea NOT NULL,
    operation golem_base_operation_type NOT NULL,
    data bytea,
    prev_data bytea,
    btl numeric(21,0),
    status golem_base_entity_status_type NOT NULL,
    prev_status golem_base_entity_status_type,
    expires_at_block_number bigint,
    prev_expires_at_block_number bigint,

    primary key (transaction_hash, op_index),
    foreign key (transaction_hash, op_index) references golem_base_operations(transaction_hash, index)
);

CREATE TABLE golem_base_numeric_annotations (
    entity_key bytea NOT NULL,
    operation_tx_hash bytea NOT NULL,
    operation_index bigint NOT NULL,
    active boolean DEFAULT true NOT NULL,
    key text NOT NULL,
    value numeric(21,0) NOT NULL,
    inserted_at timestamp without time zone DEFAULT now() NOT NULL,
    id serial not null primary key,

    foreign key (operation_tx_hash, operation_index) references golem_base_operations(transaction_hash, index),
    foreign key (entity_key) references golem_base_entities(key)
);

CREATE TABLE golem_base_pending_logs_operations (
    transaction_hash bytea NOT NULL,
    block_hash bytea NOT NULL,
    index integer NOT NULL,
    block_number integer NOT NULL,
    primary key (transaction_hash, block_hash, index)
);

CREATE TABLE golem_base_pending_transaction_cleanups (
    hash bytea NOT NULL primary key,
    inserted_at timestamp without time zone DEFAULT now() NOT NULL,

    foreign key (hash) references transactions(hash)
);

CREATE TABLE golem_base_pending_transaction_operations (
    hash bytea NOT NULL,
    inserted_at timestamp without time zone DEFAULT now() NOT NULL,
    block_number bigint NOT NULL,
    index bigint NOT NULL,

    primary key (block_number, index, hash),
    foreign key (hash) references transactions(hash)
);

CREATE TABLE golem_base_string_annotations (
    entity_key bytea NOT NULL,
    operation_tx_hash bytea NOT NULL,
    operation_index bigint NOT NULL,
    active boolean DEFAULT true NOT NULL,
    key text NOT NULL,
    value text NOT NULL,
    inserted_at timestamp without time zone DEFAULT now() NOT NULL,
    id serial NOT NULL primary key,

    foreign key (operation_tx_hash, operation_index) references golem_base_operations(transaction_hash, index),
    foreign key (entity_key) references golem_base_entities(key)
);
"#.split(";").collect();

        let create_mat_views = r#"
CREATE MATERIALIZED VIEW golem_base_entity_data_size_histogram AS
WITH entities AS (
    SELECT 
        OCTET_LENGTH(data) as size
    FROM golem_base_entities
    WHERE 
        status = 'active' 
        AND data IS NOT NULL
),
params AS (
    SELECT
        COALESCE(MIN(size), 0) AS minv,
        COALESCE(MAX(size), 0) AS maxv,
        
        COUNT(*) as total
    FROM
        entities e
),
steps AS (
    SELECT 
        p.minv,
        p.maxv,
        p.total,
        CEIL((p.maxv - p.minv + 1)::numeric / 10)::bigint AS step
    FROM params p
),
buckets AS (
    SELECT 
        gs.bucket,
        (s.minv + (gs.bucket - 1) * s.step) AS bin_start,

        CASE WHEN s.total = 0 
        THEN 
            (s.minv + gs.bucket * s.step)
        ELSE 
            LEAST(s.minv + gs.bucket * s.step - 1, s.maxv)
        END AS bin_end,

        s.total
    FROM steps s
    CROSS JOIN generate_series(1, 10) AS gs(bucket)
),
counts AS (
    SELECT 
        LEAST( 
            10,
            GREATEST(
                1, 
                ((size - s.minv) / s.step) + 1
            )
        ) as bucket,
        COUNT(*) as count
    FROM entities e, steps s
    GROUP BY 1
)

SELECT
    b.bucket,
    b.bin_start,
    b.bin_end,
    COALESCE(c.count, 0) AS count
FROM buckets b
LEFT JOIN counts c USING (bucket)
ORDER BY b.bucket;

CREATE MATERIALIZED VIEW golem_base_leaderboard_biggest_spenders AS
SELECT 
    ROW_NUMBER() OVER(ORDER BY SUM(cumulative_gas_used * gas_price) DESC) AS rank,
    from_address_hash AS address, 
    CAST(SUM(cumulative_gas_used * gas_price) AS TEXT) AS total_fees
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
    SUM(cumulative_gas_used * gas_price) DESC;

CREATE MATERIALIZED VIEW golem_base_leaderboard_data_owned AS
SELECT
    ROW_NUMBER() OVER(ORDER BY SUM(LENGTH(data)) DESC) AS rank,
    owner AS address,
    SUM(LENGTH(data)) AS data_size
FROM 
    golem_base_entities
WHERE 
    owner IS NOT NULL
    AND status = 'active'
GROUP BY 
    owner
ORDER BY 
    data_size DESC;

CREATE MATERIALIZED VIEW golem_base_leaderboard_effectively_largest_entities AS
SELECT
    ROW_NUMBER() OVER(ORDER BY (data_size * lifespan) DESC) AS rank,
    entity_key,
    data_size,
    lifespan
FROM (
    SELECT
        key AS entity_key,
        OCTET_LENGTH(data) AS data_size,
        COALESCE(expires_at_block_number - createtx.block_number, 0)  AS lifespan
    FROM
        golem_base_entities
    INNER JOIN
        transactions AS createtx ON golem_base_entities.created_at_tx_hash = createtx.hash
    WHERE 
        golem_base_entities.status = 'active'
) raw
ORDER BY
    (data_size * lifespan) DESC;

CREATE MATERIALIZED VIEW golem_base_leaderboard_entities_created AS
SELECT
    ROW_NUMBER() OVER(ORDER BY COUNT(*) DESC, MIN(inserted_at) ASC) AS rank,
    sender AS address,
    COUNT(*) AS entities_created_count,
    MIN(inserted_at) AS first_created_at
FROM
    golem_base_operations
WHERE
    operation = 'create'
    AND sender IS NOT NULL
GROUP BY
    address
ORDER BY
    entities_created_count DESC,
    first_created_at ASC;

CREATE MATERIALIZED VIEW golem_base_leaderboard_entities_owned AS
SELECT
    ROW_NUMBER() OVER(ORDER BY COUNT(*) DESC) as rank,
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
    entities_count DESC;

CREATE MATERIALIZED VIEW golem_base_leaderboard_largest_entities AS
SELECT
    ROW_NUMBER() OVER(ORDER BY length(data) DESC) AS rank,
    key AS entity_key,
    LENGTH(data) AS data_size
FROM
    golem_base_entities
WHERE 
    data IS NOT NULL
    AND status = 'active'
ORDER BY
    data_size DESC;

CREATE MATERIALIZED VIEW golem_base_leaderboard_top_accounts AS
SELECT
    ROW_NUMBER() OVER(ORDER BY fetched_coin_balance DESC, hash ASC) AS rank,
    hash AS address,
    coalesce(fetched_coin_balance, 0) as balance,
    coalesce(transactions_count, 0) as tx_count
FROM
    addresses
ORDER BY
    fetched_coin_balance DESC,
    hash ASC;

CREATE MATERIALIZED VIEW golem_base_timeseries_data_usage AS
WITH hourly_changes AS (
    SELECT 
        DATE_TRUNC('hour', block_timestamp) AS timestamp,
        SUM(
            CASE 
                WHEN operation = 'create' THEN 
                    COALESCE(length(data), 0)
                WHEN operation = 'update' THEN 
                    COALESCE(length(data), 0) - COALESCE(length(prev_data), 0)
                WHEN operation = 'delete' THEN 
                    -COALESCE(length(data), 0)
                ELSE 0  -- Ignores 'extend' and any other operations
            END
        ) AS hourly_data_change
    FROM golem_base_entity_history
    WHERE operation IN ('create', 'update', 'delete')
    GROUP BY date_trunc('hour', block_timestamp)
)
SELECT 
    timestamp,
    GREATEST(
        SUM(hourly_data_change) OVER (ORDER BY timestamp ROWS UNBOUNDED PRECEDING), 
        0
    )::BIGINT AS active_data_bytes
FROM hourly_changes
ORDER BY timestamp;

CREATE MATERIALIZED VIEW golem_base_timeseries_entity_count AS
WITH hourly_operations AS (
    SELECT 
        DATE_TRUNC('hour', block_timestamp) AS timestamp,
        COUNT(*) FILTER (WHERE operation = 'create') AS creates,
        COUNT(*) FILTER (WHERE operation = 'update') AS updates,
        COUNT(*) FILTER (WHERE operation = 'delete' AND status = 'deleted') AS deletes,
        COUNT(*) FILTER (WHERE operation = 'delete' AND status = 'expired') AS expires,
        COUNT(*) FILTER (WHERE operation = 'extend') AS extends
    FROM golem_base_entity_history
    GROUP BY DATE_TRUNC('hour', block_timestamp)
),
hourly_net_change AS (
    SELECT
        timestamp,
        creates,
        updates,
        deletes,
        expires,
        extends,
        (creates - deletes - expires)::BIGINT AS net_change
    FROM hourly_operations
)
SELECT 
    timestamp,
    creates,
    updates,
    deletes,
    expires,
    extends,
    GREATEST(
        SUM(net_change) OVER (ORDER BY timestamp ROWS UNBOUNDED PRECEDING), 
        0
    )::BIGINT AS total_entities
FROM hourly_net_change
ORDER BY timestamp;

create materialized view golem_base_timeseries_operation_count as
select 
    date_trunc('hour', block_timestamp) as timestamp,
    operation,
    count(*) as operation_count
from golem_base_entity_history
group by 1, 2
order by 1;

CREATE MATERIALIZED VIEW golem_base_timeseries_storage_forecast AS
WITH active_entities AS (
    SELECT 
        DATE_TRUNC('hour', block_timestamp + btl * '2 seconds'::INTERVAL + INTERVAL '1 hour') AS expires_at,
        CASE 
            WHEN data IS NOT NULL THEN LENGTH(data)
            ELSE 0
        END AS storage_bytes
    FROM golem_base_entity_history
    WHERE 
        btl IS NOT NULL
        AND btl > 0
        AND block_timestamp AT TIME ZONE 'UTC' + btl * '2 seconds'::INTERVAL > (NOW() AT TIME ZONE 'UTC')
),
hourly_expirations AS (
    SELECT 
        expires_at,
        SUM(storage_bytes) AS bytes_expiring
    FROM active_entities
    GROUP BY expires_at
),
current_total AS (
    SELECT 
        DATE_TRUNC('hour', NOW() AT TIME ZONE 'UTC')::timestamp AS timestamp,
        COALESCE(SUM(bytes_expiring), 0)::BIGINT AS total_storage
    FROM hourly_expirations
),
future_projections AS (
    SELECT 
        expires_at::timestamp AS timestamp,
        COALESCE((SUM(bytes_expiring) OVER (
            ORDER BY expires_at DESC 
            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
        ) - bytes_expiring), 0)::BIGINT AS total_storage
    FROM hourly_expirations
)
SELECT timestamp, total_storage FROM current_total
UNION
SELECT timestamp, total_storage 
FROM future_projections 
WHERE timestamp NOT IN (SELECT timestamp FROM current_total)
ORDER BY timestamp;
        "#.split(";").collect();

        let create_indices_and_triggers = r#"
CREATE INDEX golem_base_entity_active_data_size_index ON golem_base_entities (octet_length(data)) WHERE ((status = 'active') AND (data IS NOT NULL));
CREATE UNIQUE INDEX golem_base_entity_active_data_size_output_index ON golem_base_entity_data_size_histogram (bucket);
CREATE INDEX golem_base_entity_history_entity_key_block_number_tx_index__idx ON golem_base_entity_history (entity_key, block_number, tx_index, op_index);
CREATE INDEX golem_base_entity_history_entity_key_operation_idx ON golem_base_entity_history (entity_key, operation);
CREATE INDEX golem_base_entity_history_status_block_number_idx ON golem_base_entity_history (status, block_number);
CREATE UNIQUE INDEX golem_base_leaderboard_biggest_spenders_output_index ON golem_base_leaderboard_biggest_spenders (rank);
CREATE UNIQUE INDEX golem_base_leaderboard_data_owned_output_index ON golem_base_leaderboard_data_owned (rank);
CREATE UNIQUE INDEX golem_base_leaderboard_effectively_largest_entities_output_inde ON golem_base_leaderboard_effectively_largest_entities (rank);
CREATE UNIQUE INDEX golem_base_leaderboard_entities_created_output_index ON golem_base_leaderboard_entities_created (rank);
CREATE UNIQUE INDEX golem_base_leaderboard_entities_owned_output_index ON golem_base_leaderboard_entities_owned (rank);
CREATE UNIQUE INDEX golem_base_leaderboard_largest_entities_output_index ON golem_base_leaderboard_largest_entities (rank);
CREATE UNIQUE INDEX golem_base_leaderboard_top_accounts_output_index ON golem_base_leaderboard_top_accounts (rank);
CREATE INDEX golem_base_numeric_annotations_entity_idx ON golem_base_numeric_annotations (entity_key);
CREATE INDEX golem_base_numeric_annotations_key_idx ON golem_base_numeric_annotations (key);
CREATE INDEX golem_base_numeric_annotations_op_idx ON golem_base_numeric_annotations (operation_tx_hash, operation_index);
CREATE INDEX golem_base_operations_block_hash_idx ON golem_base_operations (block_hash);
CREATE INDEX golem_base_operations_entity_key_idx ON golem_base_operations (entity_key);
CREATE INDEX golem_base_operations_sender_idx ON golem_base_operations (sender);
CREATE INDEX golem_base_operations_transaction_hash_idx ON golem_base_operations (transaction_hash);
CREATE INDEX golem_base_pending_logs_operations_block_number_idx ON golem_base_pending_logs_operations (block_number);
CREATE INDEX golem_base_string_annotations_entity_idx ON golem_base_string_annotations (entity_key);
CREATE INDEX golem_base_string_annotations_key_idx ON golem_base_string_annotations (key);
CREATE INDEX golem_base_string_annotations_op_idx ON golem_base_string_annotations (operation_tx_hash, operation_index);
CREATE UNIQUE INDEX golem_base_timeseries_data_usage_output_index ON golem_base_timeseries_data_usage ("timestamp");
CREATE UNIQUE INDEX golem_base_timeseries_entity_count_timestamp_idx ON golem_base_timeseries_entity_count ("timestamp");
CREATE UNIQUE INDEX golem_base_timeseries_operation_count_output_index ON golem_base_timeseries_operation_count (operation, "timestamp");
CREATE UNIQUE INDEX golem_base_timeseries_storage_forecast_output_index ON golem_base_timeseries_storage_forecast ("timestamp");

CREATE TRIGGER golem_base_handle_logs_insert
    AFTER INSERT ON logs FOR EACH ROW
    WHEN (
        new.address_hash = '\x00000000000000000000000000000061726B6976' AND
        new.first_topic = '\xe3dbbcdb0a31e8bbde82b5756869daff81ae12c21009a8f7fcc8a07e00948a0f'
        AND new.block_number IS NOT NULL
    ) EXECUTE FUNCTION golem_base_queue_logs_processing();
CREATE TRIGGER golem_base_handle_logs_update
    AFTER UPDATE ON logs FOR EACH ROW
    WHEN (
        new.address_hash = '\x00000000000000000000000000000061726B6976' AND
        new.first_topic = '\xe3dbbcdb0a31e8bbde82b5756869daff81ae12c21009a8f7fcc8a07e00948a0f' AND
        new.block_number IS NOT NULL AND
        old.block_number IS NULL
    ) EXECUTE FUNCTION golem_base_queue_logs_processing();
CREATE TRIGGER golem_base_handle_tx_insert
    AFTER INSERT ON transactions FOR EACH ROW
    WHEN (
        new.to_address_hash = '\x00000000000000000000000000000061726B6976' AND
        new.block_hash IS NOT NULL AND
        new.status = 1 AND
        new.input <> '\x'
    ) EXECUTE FUNCTION golem_base_queue_transaction_processing();
CREATE TRIGGER golem_base_handle_tx_update 
    AFTER UPDATE ON transactions FOR EACH ROW
    WHEN (
        new.to_address_hash = '\x00000000000000000000000000000061726B6976' AND
        (old.block_hash IS NULL OR old.status = 0) AND
        new.block_hash IS NOT NULL AND
        new.status = 1 AND
        new.input <> '\x'
    ) EXECUTE FUNCTION golem_base_queue_transaction_processing();
CREATE TRIGGER golem_base_handle_tx_update_for_cleanup
    AFTER UPDATE ON transactions FOR EACH ROW
    WHEN (
        new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x00000000000000000000000000000061726B6976') AND
        new.block_hash IS NULL AND
        old.block_hash IS NOT NULL
    ) EXECUTE FUNCTION golem_base_queue_transaction_cleanup();
        "#.split(";").collect();

        let initial_data = r#"
INSERT INTO 
    smart_contracts (
        name,
        abi,
        address_hash,
        inserted_at,
        updated_at,
        compiler_version,
        optimization,
        contract_source_code,
        contract_code_md5
    )
VALUES (
  'ArkivStorage',
  '[
    {
      "type":"event",
      "name":"ArkivEntityCreated",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":true,"name":"owner","type":"address"},
        {"indexed":false,"name":"expirationBlock","type":"uint256"},
        {"indexed":false,"name":"cost","type":"uint256"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"ArkivEntityUpdated",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":true,"name":"owner","type":"address"},
        {"indexed":false,"name":"oldExpirationBlock","type":"uint256"},
        {"indexed":false,"name":"newExpirationBlock","type":"uint256"},
        {"indexed":false,"name":"cost","type":"uint256"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"ArkivEntityDeleted",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":true,"name":"owner","type":"address"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"ArkivEntityBTLExtended",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":true,"name":"owner","type":"address"},
        {"indexed":false,"name":"oldExpirationBlock","type":"uint256"},
        {"indexed":false,"name":"newExpirationBlock","type":"uint256"},
        {"indexed":false,"name":"cost","type":"uint256"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"ArkivEntityOwnerChanged",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":true,"name":"oldOwner","type":"address"},
        {"indexed":true,"name":"newOwner","type":"address"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"ArkivEntityExpired",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":true,"name":"owner","type":"address"}
      ],
      "anonymous":false
    }
  ]'::jsonb,
  decode('00000000000000000000000000000061726B6976','hex'),
  NOW(), 
  NOW(),
  '0.0.0', 
  false, 
  'ArkivStorage', 
  '0x00'
);


insert into golem_base_pending_logs_operations (transaction_hash, block_hash, index, block_number)
select
    txs.hash,
    txs.block_hash,
    logs.index,
    txs.block_number
from golem_base_pending_transaction_operations tx_queue
inner join transactions txs on tx_queue.hash = txs.hash
inner join logs on txs.hash = logs.transaction_hash
where txs.to_address_hash = '\x4200000000000000000000000000000000000015';


insert into golem_base_pending_transaction_operations (hash, block_number, index)
select hash, block_number, index from transactions
where
    to_address_hash = '\x00000000000000000000000000000061726B6976'
    and block_hash is not null
    and status = 1;
        "#
        .split(";")
        .collect();

        let stmts: Vec<Statement> = [
            create_types,
            create_functions,
            create_tables,
            create_mat_views,
            create_indices_and_triggers,
            initial_data,
        ]
        .concat()
        .into_iter()
        .map(|v| Statement::from_string(DatabaseBackend::Postgres, v))
        .collect();

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(r#"
        -- delete ArkivStorage from smart_contracts
        DELETE FROM smart_contracts WHERE name = 'ArkivStorage' AND contract_source_code = 'ArkivStorage';

        -- drop triggers
        DROP TRIGGER IF EXISTS golem_base_handle_tx_update_for_cleanup ON transactions;
        DROP TRIGGER IF EXISTS golem_base_handle_tx_update ON transactions;
        DROP TRIGGER IF EXISTS golem_base_handle_tx_insert ON transactions;
        DROP TRIGGER IF EXISTS golem_base_handle_logs_update ON logs;
        DROP TRIGGER IF EXISTS golem_base_handle_logs_insert ON logs;

        -- drop materialized views
        DROP MATERIALIZED VIEW IF EXISTS golem_base_entity_data_size_histogram;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_biggest_spenders;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_data_owned;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_effectively_largest_entities;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_entities_created;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_entities_owned;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_largest_entities;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_top_accounts;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_data_usage;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_entity_count;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_operation_count;
        DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_storage_forecast;

        -- drop tables
        DROP TABLE IF EXISTS golem_base_string_annotations;
        DROP TABLE IF EXISTS golem_base_pending_transaction_operations;
        DROP TABLE IF EXISTS golem_base_pending_transaction_cleanups;
        DROP TABLE IF EXISTS golem_base_pending_logs_operations;
        DROP TABLE IF EXISTS golem_base_numeric_annotations;
        DROP TABLE IF EXISTS golem_base_entity_history;
        DROP TABLE IF EXISTS golem_base_operations;
        DROP TABLE IF EXISTS golem_base_entity_locks;
        DROP TABLE IF EXISTS golem_base_entities;

        -- drop functions
        DROP FUNCTION IF EXISTS golem_base_queue_transaction_processing();
        DROP FUNCTION IF EXISTS golem_base_queue_transaction_cleanup();
        DROP FUNCTION IF EXISTS golem_base_queue_logs_processing();

        -- drop types
        DROP TYPE IF EXISTS golem_base_operation_type;
        DROP TYPE IF EXISTS golem_base_entity_status_type;
        "#).await?;

        Ok(())
    }
}
