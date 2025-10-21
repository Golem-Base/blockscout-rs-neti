use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            delete from golem_base_pending_transaction_operations
            where hash in (
                select hash
                from transactions
                where
                    from_address_hash = '\xdeaddeaddeaddeaddeaddeaddeaddeaddead0001' and
                    to_address_hash = '\x4200000000000000000000000000000000000015'
            )
        "#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
