use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("
        CREATE TABLE optimism_children_l3_withdrawals (
            id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            chain_id BIGINT NOT NULL REFERENCES optimism_children_l3_chains(chain_id) ON DELETE CASCADE,
            block_number BIGINT NOT NULL,
            block_hash BYTEA NOT NULL,
            tx_hash BYTEA NOT NULL,
            nonce NUMERIC(100, 0) NOT NULL,
            sender BYTEA NOT NULL,
            target BYTEA NOT NULL,
            value NUMERIC(100, 0) NOT NULL,
            gas_limit NUMERIC(100, 0) NOT NULL,
            data BYTEA NOT NULL,
            withdrawal_hash BYTEA NOT NULL,
            inserted_at TIMESTAMP DEFAULT NOW() NOT NULL
        );
        ")
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE optimism_children_l3_withdrawals")
            .await?;

        Ok(())
    }
}
