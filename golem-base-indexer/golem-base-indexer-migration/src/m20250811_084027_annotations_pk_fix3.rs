use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_string_annotations add column id serial primary key;
        alter table golem_base_numeric_annotations add column id serial primary key;
        "#;
        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_string_annotations drop column id;
        alter table golem_base_numeric_annotations drop column id;
        "#;
        crate::from_sql(manager, sql).await
    }
}
