use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_table("transactions").await? {
            return Err(DbErr::Migration(
                "Table transactions does not exist in the database".to_string(),
            ));
        }

        let sql = r#"
create table golem_base_pending_transaction_cleanups (
    hash bytea not null primary key references transactions (hash),
    inserted_at timestamp NOT NULL DEFAULT (now())
);
alter table golem_base_operations add column recipient bytea not null;
drop trigger golem_base_queue_transaction_processing on transactions;
drop function golem_base_queue_transaction_processing;
        "#;

        let create_function_new_tx = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function golem_base_queue_transaction_processing()
    returns trigger
    language plpgsql
as
$$
begin
    insert into golem_base_pending_transaction_operations (hash) values (new.hash);
    return new;
end;
$$
"#,
        );

        let create_function_dropped_tx = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function golem_base_queue_transaction_cleanup()
    returns trigger
    language plpgsql
as
$$
begin
    insert into golem_base_pending_transaction_cleanups (hash) values (new.hash);
    return new;
end;
$$
"#,
        );

        let create_insert_trigger = Statement::from_string(
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

        let create_update_trigger = Statement::from_string(
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

        let create_cleanup_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create trigger golem_base_handle_tx_update_for_cleanup
after update on transactions
for each row
when (
    new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453')
    and new.block_hash is null
    and old.block_hash is not null
)
execute function golem_base_queue_transaction_cleanup();
"#,
        );

        let mut stmts: Vec<_> = sql
            .split(';')
            .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
            .collect();
        stmts.push(create_function_new_tx);
        stmts.push(create_function_dropped_tx);
        stmts.push(create_update_trigger);
        stmts.push(create_insert_trigger);
        stmts.push(create_cleanup_trigger);

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
            drop trigger if exists golem_base_handle_tx_create on transactions;
            drop trigger if exists golem_base_handle_tx_update on transactions;
            drop trigger if exists golem_base_handle_tx_update_for_cleanup on transactions;
            drop function if exists golem_base_queue_transaction_processing;
            drop function if exists golem_base_queue_transaction_cleanup;
            drop table if exists golem_base_pending_transaction_cleanups;
            alter table golem_base_operations drop column recipient bytea not null;
        "#;
        let create_function = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function golem_base_queue_transaction_processing()
    returns trigger
    language plpgsql
as
$$
begin
    if new.to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453') then
        insert into golem_base_pending_transaction_operations (hash) values (new.hash);
    end if;
    return new;
end;
$$
"#,
        );

        let create_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create trigger golem_base_queue_transaction_processing
after insert on transactions
for each row
execute function golem_base_queue_transaction_processing();
"#,
        );
        let mut stmts: Vec<_> = sql
            .split(';')
            .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
            .collect();
        stmts.push(create_function);
        stmts.push(create_trigger);

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }
}
