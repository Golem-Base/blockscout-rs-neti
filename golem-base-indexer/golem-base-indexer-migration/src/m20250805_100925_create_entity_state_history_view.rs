use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE OR REPLACE VIEW golem_base_entity_history AS
WITH 
entity_state AS (
  SELECT
    o.entity_key,
    t.block_number,
    o.transaction_hash as transaction_hash,
    t.index as tx_index,
    o.index as op_index,
    o.inserted_at as op_inserted_at,
    b.timestamp as block_timestamp,
    o.sender as sender,
    o.operation AS operation,
    o.data as data,
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
WHERE 
(
  data IS DISTINCT FROM prev_data 
  OR status IS DISTINCT FROM prev_status 
  OR expires_at_block_number IS DISTINCT FROM prev_expires_at_block_number
)
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
