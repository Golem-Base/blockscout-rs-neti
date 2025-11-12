pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{Statement, TransactionTrait};

mod m20251111_170928_v2;
mod m20251112_143226_add_change_owner;
// mod m20251112_173714_add_constraint_golem_base_operations_check3;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251111_170928_v2::Migration),
            Box::new(m20251112_143226_add_change_owner::Migration),
            // TOOD: This doesn't work due to sea-orm running all migrations in a single
            // transaction. PostgreSQL doesn't like `ALTER TYPE` and the subsequent use of altered
            // type in the same transaction and errors out.
            // THe following migration is most likely not critical.
            //Box::new(m20251112_173714_add_constraint_golem_base_operations_check3::Migration),
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
