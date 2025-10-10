use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
drop materialized view golem_base_timeseries_operation_count;
create materialized view golem_base_timeseries_operation_count as
select 
    date_trunc('hour', block_timestamp) as timestamp,
    operation,
    count(*) as operation_count
from golem_base_entity_history
group by 1, 2
order by 1;

create unique index golem_base_timeseries_operation_count_output_index
on golem_base_timeseries_operation_count (operation, timestamp);
        "#;

        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
drop materialized view golem_base_timeseries_operation_count;
create materialized view golem_base_timeseries_operation_count as
select 
    date_trunc('hour', block_timestamp) as timestamp,
    count(*) as operation_count
from golem_base_entity_history
group by 1
order by 1;

create unique index golem_base_timeseries_operation_count_output_index
on golem_base_timeseries_operation_count (timestamp)
"#;

        crate::from_sql(manager, sql).await
    }
}
