use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Add changeowner constraint on `golem_base_operations`
        db.execute_unprepared(r#"ALTER TABLE golem_base_operations ADD CONSTRAINT golem_base_operations_check3 CHECK (((operation <> 'changeowner'::golem_base_operation_type) OR ((data IS NULL) AND (btl IS NULL))));"#).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Drop changeowner constraint on `golem_base_operations`
        db.execute_unprepared(r#"ALTER TABLE golem_base_operations DROP CONSTRAINT IF EXISTS golem_base_operations_check3;"#).await?;

        Ok(())
    }
}
