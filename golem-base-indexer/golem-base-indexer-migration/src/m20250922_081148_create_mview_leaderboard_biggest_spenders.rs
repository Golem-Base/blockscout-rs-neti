use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_leaderboard_biggest_spenders AS
SELECT 
    ROW_NUMBER() OVER(ORDER BY SUM(cumulative_gas_used * gas_price) DESC) as rank,
    from_address_hash as address, 
    CAST(SUM(cumulative_gas_used * gas_price) AS TEXT) as total_fees
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
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_biggest_spenders;
        "#;

        crate::from_sql(manager, sql).await
    }
}
