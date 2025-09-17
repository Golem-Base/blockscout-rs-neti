pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{Statement, TransactionTrait};

mod m20220101_000001_create_table;
mod m20250802_103147_annotations_pk_fix;
mod m20250807_143733_annotations_pk_fix2;
mod m20250811_084027_annotations_pk_fix3;
mod m20250811_091505_operations_reference_blocks;
mod m20250812_100925_create_entity_state_history_view;
mod m20250818_181205_nullable_entity_owner;
mod m20250827_115015_fix_tracking_expirations_in_view;
mod m20250904_082310_add_golem_base_events_abi;
mod m20250909_062255_create_golem_base_timeseries;
mod m20250915_140948_optimize_history_view;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250802_103147_annotations_pk_fix::Migration),
            Box::new(m20250807_143733_annotations_pk_fix2::Migration),
            Box::new(m20250811_084027_annotations_pk_fix3::Migration),
            Box::new(m20250811_091505_operations_reference_blocks::Migration),
            Box::new(m20250812_100925_create_entity_state_history_view::Migration),
            Box::new(m20250818_181205_nullable_entity_owner::Migration),
            Box::new(m20250827_115015_fix_tracking_expirations_in_view::Migration),
            Box::new(m20250904_082310_add_golem_base_events_abi::Migration),
            Box::new(m20250909_062255_create_golem_base_timeseries::Migration),
            Box::new(m20250915_140948_optimize_history_view::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("golem_base_indexer_migrations").into_iden()
    }
}

pub async fn from_sql(manager: &SchemaManager<'_>, content: &str) -> Result<(), DbErr> {
    let stmts: Vec<&str> = content.split(';').collect();
    let txn = manager.get_connection().begin().await?;

    for st in stmts {
        txn.execute(Statement::from_string(
            manager.get_database_backend(),
            st.to_string(),
        ))
        .await
        .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
    }
    txn.commit().await
}
