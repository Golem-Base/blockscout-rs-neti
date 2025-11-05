use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("
        CREATE TABLE optimism_children_l3_deposits (
            id SERIAL PRIMARY KEY,
            chain_id BIGINT NOT NULL REFERENCES optimism_children_l3_chains(chain_id) ON DELETE CASCADE,
            block_hash BYTEA NOT NULL,
            tx_hash BYTEA NOT NULL,
            source_hash BYTEA NOT NULL,
            status BOOLEAN NOT NULL,
            created_at TIMESTAMP DEFAULT NOW()
        );

        CREATE INDEX idx_l3_deposits_chain_id ON optimism_children_l3_deposits(chain_id);
        CREATE INDEX idx_l3_deposits_tx_hash ON optimism_children_l3_deposits(tx_hash);
        CREATE INDEX idx_l3_deposits_chain_status ON optimism_children_l3_deposits(chain_id, status);
        ")
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE optimism_children_l3_deposits")
            .await?;

        Ok(())
    }
}
