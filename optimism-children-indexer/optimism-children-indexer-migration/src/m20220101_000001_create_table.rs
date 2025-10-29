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
create table optimism_children_pending_logs (
    transaction_hash bytea not null references transactions (hash),
    block_hash bytea not null references blocks (hash),
    index int not null,
    block_number int not null,
    primary key (transaction_hash, block_hash, index)
);
        "#;

        let create_logs_insert_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger optimism_children_handle_logs_insert
after insert on logs
for each row
when (
    new.first_topic = '\x0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93' -- FIXME
    and new.block_number is not null
)
execute function optimism_children_queue_logs_processing();
        "#,
        );

        let create_logs_update_trigger = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace trigger optimism_children_handle_logs_update
after update on logs
for each row
when (
    new.first_topic = '\x0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93' -- FIXME
    and new.block_number is not null
    and old.block_number is null
)
execute function optimism_children_queue_logs_processing();
"#,
        );

        let create_function_process_log = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
create or replace function optimism_children_queue_logs_processing()
    returns trigger
    language plpgsql
as
$$
declare
    v_address_hash bytea;
begin

    insert into optimism_children_pending_logs (transaction_hash, block_hash, index, block_number)
        values (new.transaction_hash, new.block_hash, new.index, new.block_number) on conflict do nothing;

    return new;
end;
$$
"#,
        );

        let copy_data = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
insert into optimism_children_pending_logs (transaction_hash, block_hash, index, block_number)
select transaction_hash, block_hash, index, block_number from logs
where
    first_topic = '\x0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93' -- FIXME
    and block_number is not null;
"#,
        );

        let mut stmts: Vec<_> = sql
            .split(';')
            .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
            .collect();
        stmts.push(create_function_process_log);
        stmts.push(create_logs_update_trigger);
        stmts.push(create_logs_insert_trigger);
        stmts.push(copy_data);

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
            drop trigger if exists optimism_children_handle_logs_update on logs;
            drop trigger if exists optimism_children_handle_logs_insert on logs;
            drop function if exists optimism_children_queue_logs_processing;
            drop table if exists optimism_children_pending_logs;
        "#;
        crate::from_sql(manager, sql).await
    }
}
