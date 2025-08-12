use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE OR REPLACE VIEW golem_base_entity_history AS
WITH 
entity_state_raw AS (
  SELECT
    o.entity_key,
    t.block_number,
    o.transaction_hash AS transaction_hash,
    t.index AS tx_index,
    o.index AS op_index,
    o.inserted_at AS op_inserted_at,
    b.timestamp AS block_timestamp,
    o.sender AS sender,
    o.operation AS operation,
    o.data AS original_data,
    o.btl AS btl,

    CASE
      WHEN o.operation = 'delete' AND t.to_address_hash = '\x0000000000000000000000000000000060138453'
        THEN 'expired'
      WHEN o.operation = 'delete' THEN 'deleted'
      ELSE 'active'
    END AS status,

    CASE 
      WHEN o.operation IN ('create', 'update', 'extend') 
        THEN t.block_number + o.btl
      WHEN o.operation = 'delete' 
        THEN t.block_number
    END AS expires_at_block_number

  FROM golem_base_operations o
  JOIN transactions t ON o.transaction_hash = t.hash
  JOIN blocks b ON t.block_number = b.number
),

entity_state AS (
  SELECT
    esr.*,
    COALESCE(esr.original_data, latest_data.data) AS data
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

entity_state_diff AS (
  SELECT 
    es.*,

    LAG(es.operation) OVER w AS prev_operation,
    LAG(es.data) OVER w AS prev_data,
    LAG(es.status) OVER w AS prev_status,
    LAG(expires_at_block_number) OVER w AS prev_expires_at_block_number

  FROM
    entity_state es
  WINDOW w AS (
    PARTITION BY es.entity_key
    ORDER BY es.block_number, es.tx_index, es.op_index, es.op_inserted_at
  )
) 

SELECT
  entity_key,
  block_number,
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

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            DROP VIEW IF EXISTS golem_base_entity_history;
        "#;

        crate::from_sql(manager, sql).await
    }
}
