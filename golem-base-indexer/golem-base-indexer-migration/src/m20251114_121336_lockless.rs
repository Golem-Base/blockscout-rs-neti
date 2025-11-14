use sea_orm_migration::prelude::*;

use crate::from_sql;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE golem_base_entities_to_reindex (
    key bytea NOT NULL
);
create index golem_base_entities_to_reindex_key on golem_base_entities_to_reindex (key);
alter table golem_base_string_annotations drop constraint golem_base_string_annotations_entity_key_fkey;
alter table golem_base_numeric_annotations drop constraint golem_base_numeric_annotations_entity_key_fkey;
drop table golem_base_entity_locks;

create index golem_base_string_annotations_entity_active_idx on golem_base_string_annotations (entity_key) where active;
create index golem_base_numeric_annotations_entity_active_idx on golem_base_numeric_annotations (entity_key) where active;

"#;
        from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
drop TABLE golem_base_entities_to_reindex;
alter table golem_base_string_annotations add constraint golem_base_string_annotations_entity_key_fkey foreign key (entity_key) references golem_base_entities(key);
alter table golem_base_numeric_annotations add constraint golem_base_numeric_annotations_entity_key_fkey foreign key (entity_key) references golem_base_entities(key);
CREATE TABLE golem_base_entity_locks (
    key bytea NOT NULL primary key
);

drop index golem_base_string_annotations_entity_active_idx on golem_base_string_annotations;
drop index golem_base_numeric_annotations_entity_active_idx on golem_base_numeric_annotations;
"#;
        from_sql(manager, sql).await
    }
}
