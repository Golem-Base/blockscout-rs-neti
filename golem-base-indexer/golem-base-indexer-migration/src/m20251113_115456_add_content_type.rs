use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(r#"
        ALTER TABLE golem_base_entity_history ADD COLUMN IF NOT EXISTS content_type VARCHAR, ADD COLUMN IF NOT EXISTS prev_content_type VARCHAR;
        ALTER TABLE golem_base_operations ADD COLUMN IF NOT EXISTS content_type VARCHAR;
        ALTER TABLE golem_base_entities ADD COLUMN IF NOT EXISTS content_type VARCHAR;
        "#).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(r#"
        ALTER TABLE golem_base_entity_history DROP COLUMN IF EXISTS content_type, DROP COLUMN IF EXISTS prev_content_type;
        ALTER TABLE golem_base_operations DROP COLUMN IF EXISTS content_type;
        ALTER TABLE golem_base_entities DROP COLUMN IF EXISTS content_type;
        "#).await?;

        Ok(())
    }
}
