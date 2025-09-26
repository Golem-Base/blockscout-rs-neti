use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE MATERIALIZED VIEW golem_base_leaderboard_top_accounts AS
SELECT
    ROW_NUMBER() OVER(ORDER BY fetched_coin_balance DESC, hash ASC) AS rank,
    hash AS address,
    coalesce(fetched_coin_balance, 0) as balance,
    coalesce(transactions_count, 0) as tx_count
FROM
    addresses
ORDER BY
    fetched_coin_balance DESC,
    hash ASC;
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP MATERIALIZED VIEW IF EXISTS golem_base_leaderboard_top_accounts;
        "#;

        crate::from_sql(manager, sql).await
    }
}
