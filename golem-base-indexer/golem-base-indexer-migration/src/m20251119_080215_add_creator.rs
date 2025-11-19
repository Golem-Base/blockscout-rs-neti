use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        ALTER TABLE golem_base_entities ADD COLUMN IF NOT EXISTS creator bytea;
        "#,
        )
        .await?;

        // For each entity, find the CREATE operation and set creator to its sender
        db.execute_unprepared(
            r#"
        UPDATE golem_base_entities e
        SET creator = (
            SELECT o.sender
            FROM golem_base_operations o
            WHERE o.entity_key = e.key
                AND o.operation = 'create'
            LIMIT 1
        )
        WHERE creator IS NULL;
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        ALTER TABLE golem_base_entities DROP COLUMN IF EXISTS creator;
        "#,
        )
        .await?;

        Ok(())
    }
}
