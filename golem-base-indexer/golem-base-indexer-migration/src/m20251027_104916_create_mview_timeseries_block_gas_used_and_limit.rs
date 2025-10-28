use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_timeseries_block_gas_used_and_limit AS
SELECT 
    number AS block_number,
    gas_used::BIGINT,
    gas_limit::BIGINT,
    CASE 
        WHEN gas_limit > 0 
        THEN ROUND((gas_used::numeric / gas_limit::numeric) * 100, 2)::DOUBLE PRECISION
        ELSE 0
    END AS gas_usage_percentage
FROM blocks
WHERE consensus = true
ORDER BY number ASC;

CREATE UNIQUE INDEX golem_base_timeseries_block_gas_used_and_limit_output_index
ON golem_base_timeseries_block_gas_used_and_limit (block_number);
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS golem_base_timeseries_block_gas_used_and_limit_output_index;
DROP MATERIALIZED VIEW IF EXISTS golem_base_timeseries_block_gas_used_and_limit;
        "#;

        crate::from_sql(manager, sql).await
    }
}
