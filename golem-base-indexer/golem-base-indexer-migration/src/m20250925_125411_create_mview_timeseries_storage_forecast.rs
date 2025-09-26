use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_timeseries_storage_forecast AS
WITH active_entities AS (
    SELECT 
    DATE_TRUNC('hour', block_timestamp AT TIME ZONE 'UTC' + btl * '2 seconds'::INTERVAL) AS expires_at,
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
)
SELECT 
    expires_at::timestamp AS timestamp,
    SUM(bytes_expiring) OVER (
        ORDER BY expires_at DESC 
        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
    ) - bytes_expiring AS total_storage
FROM hourly_expirations
ORDER BY expires_at;
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_storage_forecast;
        "#;

        crate::from_sql(manager, sql).await
    }
}
