pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{Statement, TransactionTrait};

mod m20220101_000001_create_table;
mod m20251029_185826_create_optimism_children_l3_chains;
mod m20251105_083000_create_optimism_children_l3_deposits;
mod m20251210_071929_create_optimism_children_l3_withdrawals;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20251029_185826_create_optimism_children_l3_chains::Migration),
            Box::new(m20251105_083000_create_optimism_children_l3_deposits::Migration),
            Box::new(m20251210_071929_create_optimism_children_l3_withdrawals::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("optimism_children_indexer_migrations").into_iden()
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
