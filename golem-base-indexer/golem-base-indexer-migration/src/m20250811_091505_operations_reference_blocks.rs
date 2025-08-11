use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_operations
            add constraint operations_blocks_hash_fkey foreign key (block_hash) references blocks(hash)
            on delete cascade;
        "#;
        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_operations drop constraint operations_blocks_hash_fkey;
        "#;
        crate::from_sql(manager, sql).await
    }
}
