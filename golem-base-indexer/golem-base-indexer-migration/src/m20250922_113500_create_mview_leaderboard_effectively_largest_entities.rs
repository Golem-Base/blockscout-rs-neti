use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_leaderboard_effectively_largest_entities AS
select
    entity_key,
    data_size,
    lifespan
from (
    SELECT
        key as entity_key,
        octet_length(data) AS data_size,
        coalesce(expires_at_block_number - createtx.block_number, 0)  AS lifespan
    FROM
        golem_base_entities
    INNER JOIN
        transactions as createtx on golem_base_entities.created_at_tx_hash = createtx.hash
    WHERE 
        golem_base_entities.status = 'active'
) raw
order by
    (data_size * lifespan) desc;
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
