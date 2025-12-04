use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        ALTER TABLE golem_base_entity_history ADD COLUMN IF NOT EXISTS total_cost NUMERIC(100, 0) DEFAULT 0;
        ALTER TABLE golem_base_entities ADD COLUMN IF NOT EXISTS cost NUMERIC(100, 0) DEFAULT 0;
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        ALTER TABLE golem_base_entity_history DROP COLUMN IF EXISTS total_cost;
        ALTER TABLE golem_base_entities DROP COLUMN IF EXISTS cost;
        "#,
        )
        .await?;

        Ok(())
    }
}
