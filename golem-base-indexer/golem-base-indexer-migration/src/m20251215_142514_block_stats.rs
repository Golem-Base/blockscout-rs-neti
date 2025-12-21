use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            create table golem_base_block_stats (
                block_number bigint not null primary key,
                storage_usage bigint not null,
                is_dirty boolean not null default false
            );
            create index on golem_base_block_stats (is_dirty, block_number);
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            drop table golem_base_block_stats;
        "#,
        )
        .await?;

        Ok(())
    }
}
