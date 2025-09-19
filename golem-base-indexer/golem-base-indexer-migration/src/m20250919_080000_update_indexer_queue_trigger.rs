use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let delete_tx_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
drop trigger if exists golem_base_handle_tx_insert on transactions;
"#,
        );

        let delete_from_queue_empty_input = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
delete from golem_base_pending_transaction_operations
where hash in (
    select hash
    from transactions
    where input = '\x'
);
"#,
        );

        let create_tx_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create trigger golem_base_handle_tx_insert
after insert on transactions
for each row
when (
    new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453')
    and new.block_hash is not null
    and new.status = 1
    and new.input != '\x'
)
execute function golem_base_queue_transaction_processing();
"#,
        );

        let delete_tx_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
drop trigger if exists golem_base_handle_tx_update on transactions;
"#,
        );

        let create_tx_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create trigger golem_base_handle_tx_update
after update on transactions
for each row
when (
    new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453')
    and (old.block_hash is null or old.status = 0)
    and (new.block_hash is not null and new.status = 1)
    and new.input != '\x'::bytea
)
execute function golem_base_queue_transaction_processing();
"#,
        );

        let stmts: Vec<_> = vec![
            delete_tx_insert_trigger,
            delete_from_queue_empty_input,
            create_tx_insert_trigger,
            delete_tx_update_trigger,
            create_tx_update_trigger,
        ];

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let delete_tx_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
drop trigger if exists golem_base_handle_tx_insert on transactions;
"#,
        );
        let create_tx_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create trigger golem_base_handle_tx_insert
after insert on transactions
for each row
when (
    new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453')
    and new.block_hash is not null
    and new.status = 1
)
execute function golem_base_queue_transaction_processing();
"#,
        );

        let delete_tx_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
drop trigger if exists golem_base_handle_tx_update on transactions;
"#,
        );

        let create_tx_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create trigger golem_base_handle_tx_update
after update on transactions
for each row
when (
    new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453')
    and (old.block_hash is null or old.status = 0)
    and (new.block_hash is not null and new.status = 1)
)
execute function golem_base_queue_transaction_processing();
"#,
        );

        let stmts: Vec<_> = vec![
            delete_tx_insert_trigger,
            create_tx_insert_trigger,
            delete_tx_update_trigger,
            create_tx_update_trigger,
        ];

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }
}
