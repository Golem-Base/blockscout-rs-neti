use sea_orm_migration::{
    prelude::*,
    sea_orm::{DatabaseBackend, Statement, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let create_view = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
CREATE MATERIALIZED VIEW golem_base_entity_data_size_histogram AS
WITH entities AS (
    SELECT 
        OCTET_LENGTH(data) as size
    FROM golem_base_entities
    WHERE 
        status = 'active' 
        AND data IS NOT NULL
),
params AS (
    SELECT
        COALESCE(MIN(size), 0) AS minv,
        COALESCE(MAX(size), 0) AS maxv,
        
        COUNT(*) as total
    FROM
        entities e
),
steps AS (
    SELECT 
        p.minv,
        p.maxv,
        p.total,
        CEIL((p.maxv - p.minv + 1)::numeric / 10)::bigint AS step
    FROM params p
),
buckets AS (
    SELECT 
        gs.bucket,
        (s.minv + (gs.bucket - 1) * s.step) AS bin_start,

        CASE WHEN s.total = 0 
        THEN 
            (s.minv + gs.bucket * s.step)
        ELSE 
            LEAST(s.minv + gs.bucket * s.step - 1, s.maxv)
        END AS bin_end,

        s.total
    FROM steps s
    CROSS JOIN generate_series(1, 10) AS gs(bucket)
),
counts AS (
    SELECT 
        LEAST( 
            10,
            GREATEST(
                1, 
                ((size - s.minv) / s.step) + 1
            )
        ) as bucket,
        COUNT(*) as count
    FROM entities e, steps s
    GROUP BY 1
)

SELECT
    b.bucket,
    b.bin_start,
    b.bin_end,
    COALESCE(c.count, 0) AS count
FROM buckets b
LEFT JOIN counts c USING (bucket)
ORDER BY b.bucket;
"#,
        );

        let create_index = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
CREATE INDEX IF NOT EXISTS golem_base_entity_active_data_size_index
ON golem_base_entities (OCTET_LENGTH(data))
WHERE 
    status = 'active' 
    AND data IS NOT NULL
        "#,
        );

        let create_output_index = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
CREATE UNIQUE INDEX golem_base_entity_active_data_size_output_index
ON golem_base_entity_data_size_histogram (bucket);
"#,
        );

        let init_view = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
REFRESH MATERIALIZED VIEW golem_base_entity_data_size_histogram;
"#,
        );

        let stmts: Vec<_> = vec![create_view, create_index, create_output_index, init_view];

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let drop_index = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
DROP INDEX IF EXISTS golem_base_entity_active_data_size_index;
"#,
        );
        let drop_output_index = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
DROP INDEX IF EXISTS golem_base_entity_active_data_size_output_index;
"#,
        );

        let drop_view = Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
DROP MATERIALIZED VIEW golem_base_entity_data_size_histogram;
"#,
        );

        let stmts: Vec<_> = vec![drop_index, drop_output_index, drop_view];

        let txn = manager.get_connection().begin().await?;

        for st in stmts {
            txn.execute(st.clone())
                .await
                .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
        }
        txn.commit().await
    }
}
