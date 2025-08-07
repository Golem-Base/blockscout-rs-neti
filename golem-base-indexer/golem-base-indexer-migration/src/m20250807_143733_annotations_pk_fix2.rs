use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_string_annotations drop constraint golem_base_string_annotations_pkey;
        alter table golem_base_numeric_annotations drop constraint golem_base_numeric_annotations_pkey;
        create index golem_base_string_annotations_op_idx on golem_base_string_annotations (operation_tx_hash, operation_index);
        create index golem_base_numeric_annotations_op_idx on golem_base_numeric_annotations (operation_tx_hash, operation_index);
        create index golem_base_string_annotations_entity_idx on golem_base_string_annotations (entity_key);
        create index golem_base_numeric_annotations_entity_idx on golem_base_numeric_annotations (entity_key);
        create index golem_base_string_annotations_key_idx on golem_base_string_annotations (key);
        create index golem_base_numeric_annotations_key_idx on golem_base_numeric_annotations (key);
        "#;
        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        alter table golem_base_string_annotations add primary key (operation_tx_hash, operation_index, key);
        alter table golem_base_numeric_annotations add primary key (operation_tx_hash, operation_index, key);

        drop index golem_base_string_annotations_op_idx;
        drop index golem_base_numeric_annotations_op_idx;
        drop index golem_base_string_annotations_entity_idx;
        drop index golem_base_numeric_annotations_entity_idx;
        drop index golem_base_string_annotations_key_idx;
        drop index golem_base_numeric_annotations_key_idx;
        "#;
        crate::from_sql(manager, sql).await
    }
}
