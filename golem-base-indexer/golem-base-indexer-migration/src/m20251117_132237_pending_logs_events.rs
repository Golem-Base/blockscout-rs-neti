use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        -- Create table and index
        CREATE TABLE golem_base_pending_logs_events (
            transaction_hash BYTEA NOT NULL,
            block_hash BYTEA NOT NULL,
            index INTEGER NOT NULL,
            block_number INTEGER NOT NULL,
            PRIMARY KEY (transaction_hash, block_hash, index)
        );
        CREATE INDEX golem_base_pending_logs_events_block_number_idx ON golem_base_pending_logs_events (block_number);

        -- Add cost to existing tables
        ALTER TABLE golem_base_operations ADD COLUMN IF NOT EXISTS cost NUMERIC(100, 0) DEFAULT 0;
        ALTER TABLE golem_base_entity_history ADD COLUMN IF NOT EXISTS cost NUMERIC(100, 0) DEFAULT 0;

        -- Create trigger for queuing event logs
        CREATE OR REPLACE FUNCTION golem_base_queue_logs_events() RETURNS trigger
            LANGUAGE plpgsql
        AS $$
        BEGIN
            INSERT INTO golem_base_pending_logs_events (transaction_hash, block_hash, index, block_number)
                VALUES (NEW.transaction_hash, NEW.block_hash, NEW.index, NEW.block_number) 
                ON CONFLICT DO NOTHING;
            RETURN NEW;
        END;
        $$;

        -- Create trigger for INSERT on logs table
        CREATE TRIGGER golem_base_handle_logs_events_insert
            AFTER INSERT ON logs FOR EACH ROW
            WHEN (
                NEW.address_hash = '\x00000000000000000000000000000061726b6976' AND
                (
                    NEW.first_topic = '\x73dc52f9255c70375a8835a75fca19be3d9f6940536cccf5a7bc414368b389fa' OR
                    NEW.first_topic = '\x7e0bc9bab49e941b50c40ff21a415b0917df8caa9a3c3e85d6b8cfda94b52ff9' OR
                    NEW.first_topic = '\x0a5f98a4e3c7ac5f503e302ccd21b6132f04d51b89c5e02487c89ab3b7c6d60b'
                ) AND 
                NEW.block_number IS NOT NULL
            ) EXECUTE FUNCTION golem_base_queue_logs_events();

        -- Create trigger for UPDATE on logs table
        CREATE TRIGGER golem_base_handle_logs_events_update
            AFTER UPDATE ON logs FOR EACH ROW
            WHEN (
                NEW.address_hash = '\x00000000000000000000000000000061726b6976' AND
                (
                    NEW.first_topic = '\x73dc52f9255c70375a8835a75fca19be3d9f6940536cccf5a7bc414368b389fa' OR
                    NEW.first_topic = '\x7e0bc9bab49e941b50c40ff21a415b0917df8caa9a3c3e85d6b8cfda94b52ff9' OR
                    NEW.first_topic = '\x0a5f98a4e3c7ac5f503e302ccd21b6132f04d51b89c5e02487c89ab3b7c6d60b'
                ) AND
                NEW.block_number IS NOT NULL AND
                OLD.block_number IS NULL
            ) EXECUTE FUNCTION golem_base_queue_logs_events();

        -- Backfill queue
        INSERT INTO golem_base_pending_logs_events (transaction_hash, block_hash, index, block_number)
        SELECT 
            transaction_hash,
            block_hash,
            index,
            block_number
        FROM logs
        WHERE
            address_hash = '\x00000000000000000000000000000061726b6976' AND
            (
                    first_topic = '\x73dc52f9255c70375a8835a75fca19be3d9f6940536cccf5a7bc414368b389fa' OR
                    first_topic = '\x7e0bc9bab49e941b50c40ff21a415b0917df8caa9a3c3e85d6b8cfda94b52ff9' OR
                    first_topic = '\x0a5f98a4e3c7ac5f503e302ccd21b6132f04d51b89c5e02487c89ab3b7c6d60b'
            ) AND
            block_number IS NOT NULL
        ON CONFLICT DO NOTHING;
"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        -- Drop triggers
        DROP TRIGGER IF EXISTS golem_base_handle_logs_events_update ON logs;
        DROP TRIGGER IF EXISTS golem_base_handle_logs_events_insert ON logs;

        -- Drop functions
        DROP FUNCTION IF EXISTS golem_base_queue_logs_events();

        -- Remove cost from existing tables
        ALTER TABLE golem_base_operations DROP COLUMN IF EXISTS cost;
        ALTER TABLE golem_base_entity_history DROP COLUMN IF EXISTS cost;

        -- Drop table
        DROP TABLE IF EXISTS golem_base_pending_logs_events;
"#,
        )
        .await?;

        Ok(())
    }
}
