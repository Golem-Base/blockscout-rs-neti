use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
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
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_entities_owned;
        "#;

        crate::from_sql(manager, sql).await
    }
}
