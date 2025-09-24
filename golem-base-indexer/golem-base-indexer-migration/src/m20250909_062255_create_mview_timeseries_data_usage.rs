use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
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
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_data_usage;
        "#;

        crate::from_sql(manager, sql).await
    }
}
