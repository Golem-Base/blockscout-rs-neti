use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_leaderboard_effectively_largest_entities AS
SELECT
    ROW_NUMBER() OVER(ORDER BY (data_size * lifespan) DESC) AS rank,
    entity_key,
    data_size,
    lifespan
FROM (
    SELECT
        key AS entity_key,
        OCTET_LENGTH(data) AS data_size,
        COALESCE(expires_at_block_number - createtx.block_number, 0)  AS lifespan
    FROM
        golem_base_entities
    INNER JOIN
        transactions AS createtx ON golem_base_entities.created_at_tx_hash = createtx.hash
    WHERE 
        golem_base_entities.status = 'active' AND
        data IS NOT NULL
) raw
ORDER BY
    (data_size * lifespan) DESC;
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_effectively_largest_entities;
        "#;

        crate::from_sql(manager, sql).await
    }
}
