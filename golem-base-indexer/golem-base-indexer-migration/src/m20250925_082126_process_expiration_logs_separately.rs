use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // FIXME move data between queues
        let sql = r#"
            create table golem_base_pending_logs_operations (
                transaction_hash bytea not null,
                block_hash bytea not null,
                index int not null,
                block_number int not null,

                primary key (transaction_hash, block_hash, index)
            );
            create index on golem_base_pending_logs_operations (block_number);
        "#;

        let create_tx_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger golem_base_handle_tx_insert
after insert on transactions
for each row
when (
    new.to_address_hash = '\x0000000000000000000000000000000060138453'
    and new.block_hash is not null
    and new.status = 1
    and new.input != '\x'
)
execute function golem_base_queue_transaction_processing();
        "#,
        );

        let create_tx_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger golem_base_handle_tx_update
after update on transactions
for each row
when (
    new.to_address_hash = '\x0000000000000000000000000000000060138453'
    and (old.block_hash is null or old.status = 0)
    and (new.block_hash is not null and new.status = 1)
    and new.input != '\x'::bytea
)
execute function golem_base_queue_transaction_processing();
"#,
        );

        let create_logs_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger golem_base_handle_logs_insert
after insert on logs
for each row
when (
    new.address_hash = '\x0000000000000000000000000000000060138453'
    and new.first_topic = '\x0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93'
    and new.block_number is not null
)
execute function golem_base_queue_logs_processing();
        "#,
        );

        let create_logs_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger golem_base_handle_logs_update
after update on logs
for each row
when (
    new.address_hash = '\x0000000000000000000000000000000060138453'
    and new.first_topic = '\x0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93'
    and new.block_number is not null
    and old.block_number is null
)
execute function golem_base_queue_logs_processing();
"#,
        );

        let create_function_process_log = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function golem_base_queue_logs_processing()
    returns trigger
    language plpgsql
as
$$
declare
    v_address_hash bytea;
begin
    select to_address_hash into v_address_hash from transactions where hash = new.transaction_hash;
    if v_address_hash = '\x4200000000000000000000000000000000000015' then
        insert into golem_base_pending_logs_operations (transaction_hash, block_hash, index, block_number)
            values (new.transaction_hash, new.block_hash, new.index, new.block_number) on conflict do nothing;
    end if;
    return new;
end;
$$
"#,
        );

        let create_function_new_tx = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function golem_base_queue_transaction_processing()
    returns trigger
    language plpgsql
as
$$
begin
    insert into golem_base_pending_transaction_operations (hash, block_number, index) values (new.hash, new.block_number, new.index) on conflict do nothing;
    return new;
end;
$$
"#,
        );

        let mut stmts: Vec<_> = sql
            .split(';')
            .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
            .collect();
        stmts.append(&mut vec![
            create_function_process_log,
            create_logs_insert_trigger,
            create_logs_update_trigger,
            create_tx_insert_trigger,
            create_tx_update_trigger,
            create_function_new_tx,
        ]);

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            drop trigger golem_base_handle_logs_insert on logs;
            drop trigger golem_base_handle_logs_update on logs;
            drop function golem_base_queue_logs_processing;
            drop table golem_base_pending_logs_operations;
        "#;

        let create_tx_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger golem_base_handle_tx_insert
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

        let create_tx_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger golem_base_handle_tx_update
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

        let create_function_new_tx = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function golem_base_queue_transaction_processing()
    returns trigger
    language plpgsql
as
$$
begin
    insert into golem_base_pending_transaction_operations (hash, block_number, index) values (new.hash, new.block_number, new.index);
    return new;
end;
$$
"#,
        );

        let mut stmts: Vec<_> = sql
            .split(';')
            .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
            .collect();
        stmts.append(&mut vec![
            create_tx_insert_trigger,
            create_tx_update_trigger,
            create_function_new_tx,
        ]);

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }
}
