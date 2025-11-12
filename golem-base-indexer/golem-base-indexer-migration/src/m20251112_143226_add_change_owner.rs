use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        -- Add changeowner to golem_base_operation_type
        ALTER TYPE golem_base_operation_type ADD VALUE IF NOT EXISTS 'changeowner';

        -- Add owner column
        ALTER TABLE golem_base_operations ADD COLUMN owner bytea;
        -- Copy current sender to owner
        UPDATE golem_base_operations SET owner = sender WHERE owner IS NULL;
        -- Set owner to NOT NULL
        ALTER TABLE golem_base_operations ALTER COLUMN owner SET NOT NULL;
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        -- Drop owner column
        ALTER TABLE golem_base_operations DROP COLUMN IF EXISTS owner;
        "#,
        )
        .await?;

        Ok(())
    }
}
