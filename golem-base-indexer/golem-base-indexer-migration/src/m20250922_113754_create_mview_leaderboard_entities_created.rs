use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_leaderboard_entities_created AS
SELECT
    ROW_NUMBER() OVER(ORDER BY COUNT(*) DESC, MIN(inserted_at) ASC) as rank,
    sender as address,
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
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_entities_created;
        "#;

        crate::from_sql(manager, sql).await
    }
}
