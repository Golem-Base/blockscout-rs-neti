use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        drop view if exists golem_base_entity_history cascade;
        alter table golem_base_operations
            add column block_number bigint not null,
            add column tx_index integer not null;
        create table golem_base_entity_history (
            entity_key bytea not null,
            block_number bigint not null,
            block_hash bytea not null references blocks (hash),
            transaction_hash bytea not null references transactions (hash),
            tx_index integer not null,
            op_index bigint not null,
            block_timestamp timestamp not null,
            owner bytea,
            sender bytea not null,
            operation golem_base_operation_type not null,
            data bytea,
            prev_data bytea,
            btl numeric(21, 0),
            status golem_base_entity_status_type not null,
            prev_status golem_base_entity_status_type,
            expires_at_block_number bigint,
            prev_expires_at_block_number bigint,

            primary key (transaction_hash, op_index),
            foreign key (transaction_hash, op_index) references golem_base_operations (transaction_hash, index)
        );
        create index on golem_base_entity_history (entity_key, block_number, tx_index, op_index);
        create index on golem_base_entity_history (entity_key, operation);
        create index on golem_base_entity_history (status, block_number);
        alter table golem_base_operations add constraint fk_operations_transactions foreign key (transaction_hash) references transactions (hash);
        alter table golem_base_entities alter column expires_at_block_number drop not null;
    "#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_operations drop constraint fk_operations_transactions;
        alter table golem_base_entities alter column expires_at_block_number set not null;
        drop table golem_base_entity_history;
CREATE OR REPLACE VIEW golem_base_entity_history AS
WITH 
entity_state_raw AS (
  SELECT
    o.entity_key,
    t.block_number,
    t.block_hash,
    o.transaction_hash AS transaction_hash,
    t.index AS tx_index,
    o.index AS op_index,
    o.inserted_at AS op_inserted_at,
    t.block_timestamp,
    o.sender AS sender,
    o.operation AS operation,
    o.data AS original_data,
    o.btl AS btl,

    CASE
      WHEN o.operation = 'delete' AND t.to_address_hash = '\x4200000000000000000000000000000000000015'
        THEN 'expired'
      WHEN o.operation = 'delete' THEN 'deleted'
      ELSE 'active'
    END AS status

  FROM golem_base_operations o
  JOIN transactions t ON o.transaction_hash = t.hash
  WHERE t.block_consensus
),

entity_state AS (
  SELECT
    esr.*,
    CASE
      WHEN 
        esr.operation = 'delete' 
      THEN NULL
      ELSE 
        COALESCE(esr.original_data, latest_data.data) 
    END AS data
  FROM 
    entity_state_raw esr
  LEFT JOIN LATERAL (
    SELECT prev.original_data AS data
    FROM entity_state_raw prev
  WHERE prev.entity_key = esr.entity_key
    AND prev.original_data IS NOT NULL
    AND (
      (prev.block_number, prev.tx_index, prev.op_index, prev.op_inserted_at) < 
      (esr.block_number, esr.tx_index, esr.op_index, esr.op_inserted_at)
    )
    ORDER BY prev.block_number DESC, prev.tx_index DESC, prev.op_index DESC, prev.op_inserted_at DESC
    LIMIT 1
  ) latest_data ON true
),

entity_state_base_exp AS (
  SELECT
    es.*,
    CASE
      WHEN es.operation IN ('create', 'update')
         THEN es.block_number + es.btl::bigint
      ELSE NULL
    END AS base_expires_at
  FROM entity_state es
),

entity_state_group AS (
  SELECT
    es.*,
    -- increment group id for each create/update operation
    SUM(
      CASE 
        WHEN es.operation IN ('create','update') 
        THEN 1 
        ELSE 0 
      END
    ) OVER (
      PARTITION BY es.entity_key
      ORDER BY es.block_number, es.tx_index, es.op_index, es.op_inserted_at
    ) AS group_id
  FROM entity_state_base_exp es
),

entity_state_sum_group_exp AS (
  SELECT
    es.*,

    MAX(base_expires_at) 
    FILTER (WHERE base_expires_at IS NOT NULL)
    OVER (PARTITION BY entity_key, group_id) AS group_base_expires_at,

    SUM(
      CASE 
        WHEN operation = 'extend' 
        THEN btl::bigint 
        ELSE 0 
      END
    ) OVER (
      PARTITION BY entity_key, group_id
      ORDER BY block_number, tx_index, op_index, op_inserted_at
    ) AS group_exp_sum
  FROM entity_state_group es
),

entity_state_final_exp AS (
  SELECT
    es.*,
    CASE
      WHEN es.operation IN ('create','update') THEN es.base_expires_at
      WHEN es.operation = 'extend' THEN es.group_base_expires_at + es.group_exp_sum
      WHEN es.operation = 'delete' THEN es.block_number
      ELSE NULL
    END AS expires_at_block_number
  FROM entity_state_sum_group_exp es
),

entity_state_diff AS (
  SELECT 
    es.*,

    LAG(es.operation) OVER w AS prev_operation,
    LAG(es.data) OVER w AS prev_data,
    LAG(es.status) OVER w AS prev_status,
    LAG(es.expires_at_block_number) OVER w AS prev_expires_at_block_number

  FROM
    entity_state_final_exp es
  WINDOW w AS (
    PARTITION BY es.entity_key
    ORDER BY es.block_number, es.tx_index, es.op_index, es.op_inserted_at
  )
)

SELECT
  entity_key,
  block_number,
  block_hash,
  transaction_hash,
  tx_index,
  op_index,
  block_timestamp,
  sender,
  operation,
  data,
  prev_data,
  btl,
  status,
  prev_status,
  expires_at_block_number,
  prev_expires_at_block_number
FROM
  entity_state_diff
ORDER BY
  block_number,
  tx_index,
  op_index,
  op_inserted_at;
"#;

        crate::from_sql(manager, sql).await
    }
}
