#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"/golem-base-indexer-migration/src/

git checkout -- lib.rs
current=$(perl -ne 'm/(m\d{8}_\d{6}_.*)::Migration/ && print "$1\n"' lib.rs)
all=$(ls -1 | cut -d. -f1 | grep '^m\d')
currentF=$(mktemp)
allF=$(mktemp)
echo "$current" | sort >"$currentF"
echo "$all" | sort >"$allF"
new=$(comm -13 "$currentF" "$allF" | sort)
rm "$currentF" "$allF"

mods=""
migrations=""

for i in $current $new; do
  mods="$mods \
  mod $i;"
  migrations="$migrations \
  Box::new($i::Migration),"
done

cat <<EOF >lib.rs
pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{Statement, TransactionTrait};

$mods

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![$migrations]
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
EOF

just fmt
