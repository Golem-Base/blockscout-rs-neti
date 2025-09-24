use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_leaderboard_data_owned AS
SELECT
    ROW_NUMBER() OVER(ORDER BY SUM(LENGTH(data)) DESC) AS rank,
    owner AS address,
    SUM(LENGTH(data)) AS data_size
FROM 
    golem_base_entities
WHERE 
    owner IS NOT NULL
    AND status = 'active'
GROUP BY 
    owner
ORDER BY 
    data_size DESC;
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_data_owned;
        "#;

        crate::from_sql(manager, sql).await
    }
}
