use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        CREATE TABLE optimism_children_withdrawal_proven_events
            (
                transaction_hash BYTEA NOT NULL REFERENCES transactions (hash),
                block_hash       BYTEA NOT NULL REFERENCES blocks (hash),
                index            INTEGER NOT NULL,
                block_number     INT NOT NULL,
                withdrawal_hash  BYTEA NOT NULL,
                "from"           BYTEA NOT NULL,
                "to"             BYTEA NOT NULL,

                PRIMARY KEY (transaction_hash, block_hash, index),
                FOREIGN KEY (transaction_hash, block_hash, index) REFERENCES logs (
                transaction_hash, block_hash, index)
            );

        CREATE TABLE optimism_children_withdrawal_finalized_events
            (
                transaction_hash BYTEA NOT NULL REFERENCES transactions (hash),
                block_hash       BYTEA NOT NULL REFERENCES blocks (hash),
                index            INTEGER NOT NULL,
                block_number     INT NOT NULL,
                withdrawal_hash  BYTEA NOT NULL,
                success          BOOLEAN NOT NULL,

                PRIMARY KEY (transaction_hash, block_hash, index),
                FOREIGN KEY (transaction_hash, block_hash, index) REFERENCES logs (
                transaction_hash, block_hash, index)
            );
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        DROP TABLE IF EXISTS optimism_children_withdrawal_proven_events;
        DROP TABLE IF EXISTS optimism_children_withdrawal_finalized_events;
        "#,
        )
        .await?;

        Ok(())
    }
}
