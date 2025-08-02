use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_string_annotations drop constraint golem_base_string_annotations_pkey;
        alter table golem_base_numeric_annotations drop constraint golem_base_numeric_annotations_pkey;
        alter table golem_base_string_annotations add primary key (operation_tx_hash, operation_index, key);
        alter table golem_base_numeric_annotations add primary key (operation_tx_hash, operation_index, key);
        "#;
        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_string_annotations drop constraint golem_base_string_annotations_pkey;
        alter table golem_base_numeric_annotations drop constraint golem_base_numeric_annotations_pkey;
        alter table golem_base_string_annotations add primary key (entity_key, operation_tx_hash, operation_index);
        alter table golem_base_numeric_annotations add primary key (entity_key, operation_tx_hash, operation_index);
        "#;
        crate::from_sql(manager, sql).await
    }
}
