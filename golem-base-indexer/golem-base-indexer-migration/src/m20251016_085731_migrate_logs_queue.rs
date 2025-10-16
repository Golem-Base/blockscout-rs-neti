use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            insert into golem_base_pending_logs_operations (transaction_hash, block_hash, index, block_number)
            select
                txs.hash,
                txs.block_hash,
                logs.index,
                txs.block_number
            from golem_base_pending_transaction_operations tx_queue
            inner join transactions txs on tx_queue.hash = txs.hash
            inner join logs on txs.hash = logs.transaction_hash
            where txs.to_address_hash = '\x4200000000000000000000000000000000000015';

            delete from golem_base_pending_transaction_operations txs where exists (select 1 from golem_base_pending_logs_operations logs where txs.hash = logs.transaction_hash);
        "#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
