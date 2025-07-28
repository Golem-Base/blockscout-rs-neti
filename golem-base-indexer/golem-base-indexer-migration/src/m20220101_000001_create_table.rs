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
create type golem_base_operation_type as enum (
    'create',
    'update',
    'delete',
    'extend'
);

create type golem_base_entity_status_type as enum (
    'active',
    'deleted',
    'expired'
);

create table golem_base_operations (
    entity_key bytea not null,
    sender bytea not null,
    recipient bytea not null,
    operation golem_base_operation_type not null,
    data bytea,
    btl numeric(21,0), -- we must fit uint64

    block_hash bytea not null,
    transaction_hash bytea not null,
    index bigint not null,

    inserted_at timestamp NOT NULL DEFAULT (now()),

    primary key(transaction_hash, index),

    -- create & update ops must set data & btl
    check(operation != 'create' or operation != 'update' or (data is not null and btl is not null)),

    -- delete ops must not set data & btl
    check(operation != 'delete' or (data is null and btl is null)),

    -- extend ops must not set data & must set btl
    check(operation != 'extend' or (data is null and btl is not null))
);

-- for fetching all operations for given entity
create index on golem_base_operations (entity_key);

-- for fetching all operations for given owner
create index on golem_base_operations (sender);

-- for fetching all operations in given tx
create index on golem_base_operations (transaction_hash);

-- for fetching all operations in given block
create index on golem_base_operations (block_hash);

create table golem_base_entities (
    key bytea not null primary key,
    data bytea,
    status golem_base_entity_status_type not null,
    owner bytea not null,

    created_at_tx_hash bytea,
    last_updated_at_tx_hash bytea not null,
    expires_at_block_number bigint not null,

    inserted_at timestamp NOT NULL DEFAULT (now()),
    updated_at timestamp NOT NULL DEFAULT (now())
);

create table golem_base_string_annotations (
    entity_key bytea not null references golem_base_entities (key),
    operation_tx_hash bytea not null,
    operation_index bigint not null,
    active bool not null default 't',

    key text not null,
    value text not null,
    inserted_at timestamp NOT NULL DEFAULT (now()),

    primary key (entity_key, operation_tx_hash, operation_index),
    foreign key (operation_tx_hash, operation_index) references golem_base_operations (transaction_hash, index)
);

create table golem_base_numeric_annotations (
    entity_key bytea not null references golem_base_entities (key),
    operation_tx_hash bytea not null,
    operation_index bigint not null,
    active bool not null default 't',

    key text not null,
    value numeric(21,0) not null, -- we must fit uint64
    inserted_at timestamp NOT NULL DEFAULT (now()),

    primary key (entity_key, operation_tx_hash, operation_index),
    foreign key (operation_tx_hash, operation_index) references golem_base_operations (transaction_hash, index)
);

create table golem_base_pending_transaction_operations (
    hash bytea not null primary key references transactions (hash),
    inserted_at timestamp NOT NULL DEFAULT (now())
);

create table golem_base_pending_transaction_cleanups (
    hash bytea not null primary key references transactions (hash),
    inserted_at timestamp NOT NULL DEFAULT (now())
);
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

        let copy_data = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
insert into golem_base_pending_transaction_operations (hash)
select hash from transactions
where
    to_address_hash in ('\x4200000000000000000000000000000000000015', '\x0000000000000000000000000000000060138453')
    and block_hash is not null
    and status = 1;
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
            drop trigger if exists golem_base_handle_tx_create on transactions;
            drop trigger if exists golem_base_handle_tx_update on transactions;
            drop trigger if exists golem_base_handle_tx_update_for_cleanup on transactions;
            drop function if exists golem_base_queue_transaction_cleanup;
            drop function if exists golem_base_queue_transaction_processing;
            drop table if exists golem_base_pending_transaction_operations;
            drop table if exists golem_base_string_annotations;
            drop table if exists golem_base_numeric_annotations;
            drop table if exists golem_base_entities;
            drop table if exists golem_base_operations;
            drop table if exists golem_base_pending_transaction_cleanups;
            drop type if exists golem_base_operation_type;
            drop type if exists golem_base_entity_status_type;
        "#;
        crate::from_sql(manager, sql).await
    }
}
