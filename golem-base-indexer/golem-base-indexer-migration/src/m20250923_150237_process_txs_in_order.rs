use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            alter table golem_base_pending_transaction_operations add column block_number bigint, add column index bigint;
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
    insert into golem_base_pending_transaction_operations (hash, block_number, index) values (new.hash, new.block_number, new.index);
    return new;
end;
$$
"#,
        );

        let sql2 = r#"
            update golem_base_pending_transaction_operations pendings set block_number = (select block_number from transactions where pendings.hash = transactions.hash), index = (select index from transactions where pendings.hash = transactions.hash) where block_number is null;
            alter table golem_base_pending_transaction_operations alter column block_number set not null, alter column index set not null;
            alter table golem_base_pending_transaction_operations
                drop constraint golem_base_pending_transaction_operations_pkey,
                add primary key(block_number, index, hash);
        "#;

        let mut stmts: Vec<_> = sql
            .split(';')
            .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
            .collect();
        stmts.push(create_function_new_tx);
        stmts.append(
            &mut sql2
                .split(';')
                .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
                .collect(),
        );

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
            alter table golem_base_pending_transaction_operations
                drop constraint golem_base_pending_transaction_operations_pkey,
                add primary key(hash);
            alter table golem_base_pending_transaction_operations drop column block_number, drop column index;
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

        let mut stmts: Vec<_> = vec![create_function_new_tx];
        stmts.append(
            &mut sql
                .split(';')
                .map(|s| Statement::from_string(DatabaseBackend::Postgres, s))
                .collect(),
        );

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }
}
