use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE UNIQUE INDEX golem_base_timeseries_storage_forecast_output_index
ON golem_base_timeseries_storage_forecast (timestamp)
"#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS golem_base_timeseries_storage_forecast_output_index
"#;

        crate::from_sql(manager, sql).await
    }
}
