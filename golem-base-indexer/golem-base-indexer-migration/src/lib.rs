pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{Statement, TransactionTrait};

mod m20251111_170928_v2;
mod m20251113_115456_add_content_type;
mod m20251114_120643_add_golem_base_operations_block_number_idx;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251111_170928_v2::Migration),
            Box::new(m20251113_115456_add_content_type::Migration),
            Box::new(m20251114_120643_add_golem_base_operations_block_number_idx::Migration),
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
