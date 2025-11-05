use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("
        CREATE TABLE optimism_children_l3_deposits (
            id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            chain_id BIGINT NOT NULL REFERENCES optimism_children_l3_chains(chain_id) ON DELETE CASCADE,
            block_hash BYTEA NOT NULL,
            tx_hash BYTEA NOT NULL,
            source_hash BYTEA NOT NULL,
            success BOOLEAN NOT NULL,
            inserted_at TIMESTAMP DEFAULT NOW() NOT NULL
        );

        CREATE INDEX idx_optimism_children_l3_deposits_source_hash
            ON optimism_children_l3_deposits(source_hash);

        CREATE INDEX idx_optimism_children_l3_deposits_chain_id_success
            ON optimism_children_l3_deposits(chain_id)
            WHERE success = true;
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
