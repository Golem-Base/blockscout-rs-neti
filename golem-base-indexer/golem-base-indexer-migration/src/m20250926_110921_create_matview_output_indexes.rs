use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

const LEADERBOARDS_MAT_VIEWS: &[&str] = &[
    "golem_base_leaderboard_biggest_spenders",
    "golem_base_leaderboard_entities_owned",
    "golem_base_leaderboard_data_owned",
    "golem_base_leaderboard_largest_entities",
    "golem_base_leaderboard_effectively_largest_entities",
    "golem_base_leaderboard_entities_created",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut stmts: Vec<_> = vec![];

        for view in LEADERBOARDS_MAT_VIEWS {
            let sql = &format!("CREATE UNIQUE INDEX {view}_output_index ON {view} (rank);");
            let stmt = Statement::from_string(DatabaseBackend::Postgres, sql);
            stmts.push(stmt);
        }

        stmts.push(Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
CREATE UNIQUE INDEX golem_base_timeseries_data_usage_output_index
ON golem_base_timeseries_data_usage (timestamp)
"#,
        ));

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut stmts: Vec<_> = vec![];

        for view in LEADERBOARDS_MAT_VIEWS {
            let sql = &format!("DROP INDEX IF EXISTS {view}_output_index ON {view} (rank);");
            let stmt = Statement::from_string(DatabaseBackend::Postgres, sql);
            stmts.push(stmt);
        }

        stmts.push(Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
DROP INDEX IF EXISTS golem_base_timeseries_data_usage_output_index
"#,
        ));

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }
}
