use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(r#"
        -- Deposits
        ALTER TABLE optimism_children_l3_deposits ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL;
        ALTER TABLE optimism_children_transaction_deposited_events_v0 ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL;

        -- Withdrawals
        ALTER TABLE optimism_children_l3_withdrawals ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL;
        ALTER TABLE optimism_children_withdrawal_proven_events ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL;
        ALTER TABLE optimism_children_withdrawal_finalized_events ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL;
        "#).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        -- Deposits
        ALTER TABLE optimism_children_l3_deposits DROP COLUMN IF EXISTS block_timestamp;
        ALTER TABLE optimism_children_transaction_deposited_events_v0 DROP COLUMN IF EXISTS block_timestamp;

        -- Withdrawals
        ALTER TABLE optimism_children_l3_withdrawals DROP COLUMN IF EXISTS block_timestamp;
        ALTER TABLE optimism_children_withdrawal_proven_events DROP COLUMN IF EXISTS block_timestamp;
        ALTER TABLE optimism_children_withdrawal_finalized_events DROP COLUMN IF EXISTS block_timestamp;
        "#,
        )
        .await?;

        Ok(())
    }
}
