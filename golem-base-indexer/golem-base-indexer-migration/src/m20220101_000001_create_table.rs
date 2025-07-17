use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_table("blocks").await? {
            return Err(DbErr::Migration(
                "Table blocks does not exist in the database".to_string(),
            ));
        }
        if !manager.has_table("transactions").await? {
            return Err(DbErr::Migration(
                "Table transactions does not exist in the database".to_string(),
            ));
        }

        let sql = r#"
            create type golem_base_operation_type as enum (
                create,
                update,
                delete,
                extend
            );

            create type golem_base_entity_status_type as enum (
                active,
                deleted,
                expired
            );

            create table golem_base_operations (
                entity_hash bytea not null,
                sender bytea not null,
                operation golem_base_operation_type not null,
                data bytea,
                btl int,

                transaction_hash bytea not null,
                block_number int not null,
                block_hash bytea not null,
                index int not null,

                inserted_at timestamp NOT NULL DEFAULT (now()),

                primary key(transaction_hash, index)
            )

            -- for fetching all operations for given entity
            create index on golem_base_operations (entity_hash);

            -- for fetching all operations for given owner
            create index on golem_base_operations (sender);

            -- for fetching all operations in given tx
            create index on golem_base_operations (transaction_hash);

            -- for fetching all operations in given block. use block_number to also be able to fetch it chronologically
            create index on golem_base_operations (block_number);

            -- for fetching all operations in given block
            create index on golem_base_operations (block_hash);

            create table golem_base_entities (
                hash bytea not null primary key,
                data bytea not null,
                status golem_base_entity_status not null,

                -- block numbers
                created_at int not null,
                expires_at int not null,
                last_updated_at int not null,

                inserted_at timestamp NOT NULL DEFAULT (now()),
                updated_at timestamp NOT NULL DEFAULT (now()),
            )

            create table golem_base_string_annotations (
                entity_hash bytea not null references golem_base_entities (entity_hash),
                operation_txhash bytea not null references golem_base_operations (transaction_hash),
                operation_index bytea not null references golem_base_operations (index),
                active bool not null default 't',

                key text not null,
                value text not null
                inserted_at timestamp NOT NULL DEFAULT (now()),

                primary key (entity_hash, operation_txhash, operation_index)
            )

            create table golem_base_numeric_annotations (
                entity_hash bytea not null references golem_base_entities (entity_hash),
                operation_txhash bytea not null references golem_base_operations (transaction_hash),
                operation_index bytea not null references golem_base_operations (index),
                active bool not null default 't',

                key text not null,
                value numeric(21,0) not null -- we must fit uint64
                inserted_at timestamp NOT NULL DEFAULT (now()),

                primary key (entity_hash, operation_txhash, operation_index)
            )
        "#;
        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            drop table golem_base_entity;
            drop table golem_base_string_annotations;
            drop table golem_base_numeric_annotations;
            drop table golem_base_operations;
            drop type golem_base_operation_type;
            drop type golem_base_entity_status_type;
        "#;
        crate::from_sql(manager, sql).await
    }
}
