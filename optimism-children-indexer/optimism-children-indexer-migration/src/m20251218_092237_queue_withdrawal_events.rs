use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        create or replace trigger optimism_children_handle_logs_insert
        after insert on logs
        for each row
        when (
            new.first_topic in (
                '\xb3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32',
                '\x67a6208cfcc0801d50f6cbe764733f4fddf66ac0b04442061a8a8c0cb6b63f62',
                '\xdb5c7652857aa163daadd670e116628fb42e869d8ac4251ef8971d9e5727df1b'
            )
            and new.block_number is not null
        )
        execute function optimism_children_queue_logs_processing();

        create or replace trigger optimism_children_handle_logs_update
        after update on logs
        for each row
        when (
            new.first_topic in (
                '\xb3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32',
                '\x67a6208cfcc0801d50f6cbe764733f4fddf66ac0b04442061a8a8c0cb6b63f62',
                '\xdb5c7652857aa163daadd670e116628fb42e869d8ac4251ef8971d9e5727df1b'
            )
            and new.block_number is not null
            and old.block_number is null
        )
        execute function optimism_children_queue_logs_processing();

        -- Backfill existing logs
        insert into optimism_children_pending_logs (transaction_hash, block_hash, index, block_number)
        select transaction_hash, block_hash, index, block_number from logs
        where
            first_topic in (
                '\x67a6208cfcc0801d50f6cbe764733f4fddf66ac0b04442061a8a8c0cb6b63f62',
                '\xdb5c7652857aa163daadd670e116628fb42e869d8ac4251ef8971d9e5727df1b'
            )
            and block_number is not null
        on conflict do nothing;
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        create or replace trigger optimism_children_handle_logs_insert
        after insert on logs
        for each row
        when (
            new.first_topic = '\xb3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32'
            and new.block_number is not null
        )
        execute function optimism_children_queue_logs_processing();

        create or replace trigger optimism_children_handle_logs_update
        after update on logs
        for each row
        when (
            new.first_topic = '\xb3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32'
            and new.block_number is not null
            and old.block_number is null
        )
        execute function optimism_children_queue_logs_processing();
        "#,
        )
        .await?;

        Ok(())
    }
}
