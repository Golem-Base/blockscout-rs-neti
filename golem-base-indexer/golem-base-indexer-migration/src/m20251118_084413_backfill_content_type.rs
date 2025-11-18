use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
        UPDATE golem_base_operations
        SET content_type = ''
        WHERE 
            content_type IS NULL
            AND operation IN ('create', 'update');

        UPDATE golem_base_entities
        SET content_type = ''
        WHERE
            content_type IS NULL
            AND status = 'active';

        UPDATE golem_base_entity_history
        SET content_type = ''
        WHERE
            status = 'active';
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // NOTE: Intentionally no rollback for this migration

        Ok(())
    }
}
