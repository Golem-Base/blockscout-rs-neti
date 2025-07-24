use blockscout_service_launcher::{test_database::TestDbGuard, test_server};
use golem_base_indexer_server::Settings;
use migration::{
    from_sql, Alias, DbErr, DynIden, IntoIden, MigrationName, MigrationTrait, MigratorTrait,
    SchemaManager,
};
use reqwest::Url;
use sea_orm::{ConnectionTrait, Statement, TransactionTrait};

pub struct TestMigrator;
#[async_trait::async_trait]
impl MigratorTrait for TestMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let before: Vec<Box<dyn MigrationTrait>> = vec![Box::new(TestMigrationBefore)];
        before
            .into_iter()
            .chain(migration::Migrator::migrations())
            .collect()
    }
    fn migration_table_name() -> DynIden {
        Alias::new("golem_base_indexer_migrations").into_iden()
    }
}

pub struct TestMigrationBefore;

impl MigrationName for TestMigrationBefore {
    fn name(&self) -> &str {
        "test_migration_before_0"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for TestMigrationBefore {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        from_sql(manager, include_str!("../fixtures/blockscout_tables.sql")).await?;
        Ok(())
    }
}

pub async fn init_db(db_prefix: &str, test_name: &str) -> TestDbGuard {
    let db_name = format!("{db_prefix}_{test_name}");
    TestDbGuard::new::<TestMigrator>(db_name.as_str()).await
}

pub async fn init_golem_base_indexer_server<F>(db: TestDbGuard, settings_setup: F) -> Url
where
    F: Fn(Settings) -> Settings,
{
    tracing_subscriber::fmt::init();

    let (settings, base) = {
        let mut settings = Settings::default(db.db_url());
        let (server_settings, base) = test_server::get_test_server_settings();
        settings.server = server_settings;
        settings.metrics.enabled = false;

        (settings_setup(settings), base)
    };

    let client = db.client();
    test_server::init_server(
        || golem_base_indexer_server::run_server(client, settings),
        &base,
    )
    .await;
    base
}

#[allow(dead_code)]
pub async fn load_data<T: TransactionTrait + ConnectionTrait>(db: &T, content: &str) {
    let stmts: Vec<&str> = content.split(';').collect();
    let txn = db.begin().await.unwrap();

    for st in stmts {
        txn.execute(Statement::from_string(
            db.get_database_backend(),
            st.to_string(),
        ))
        .await
        .unwrap();
    }
    txn.commit().await.unwrap()
}
