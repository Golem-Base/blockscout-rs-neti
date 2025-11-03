use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
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

CREATE UNIQUE INDEX golem_base_timeseries_entity_count_timestamp_idx
ON golem_base_timeseries_entity_count (timestamp);
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS golem_base_timeseries_entity_count_timestamp_idx;
DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_entity_count;
        "#;

        crate::from_sql(manager, sql).await
    }
}
