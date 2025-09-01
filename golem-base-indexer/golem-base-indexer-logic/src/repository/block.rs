use anyhow::{Context, Result};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use tracing::instrument;

use crate::types::{BlockEntitiesCount, BlockNumber, BlockStorageUsage};

use super::sql;

#[derive(Debug, FromQueryResult)]
struct DbBlockEntitiesCount {
    pub create_count: i64,
    pub update_count: i64,
    pub expire_count: i64,
    pub delete_count: i64,
    pub extend_count: i64,
}

impl TryFrom<DbBlockEntitiesCount> for BlockEntitiesCount {
    type Error = anyhow::Error;

    fn try_from(value: DbBlockEntitiesCount) -> Result<Self> {
        Ok(Self {
            create_count: value.create_count.try_into()?,
            update_count: value.update_count.try_into()?,
            expire_count: value.expire_count.try_into()?,
            delete_count: value.delete_count.try_into()?,
            extend_count: value.extend_count.try_into()?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbBlockStorageUsage {
    pub block_bytes: i64,
    pub total_bytes: i64,
}

impl TryFrom<DbBlockStorageUsage> for BlockStorageUsage {
    type Error = anyhow::Error;

    fn try_from(value: DbBlockStorageUsage) -> Result<Self> {
        Ok(Self {
            block_bytes: value.block_bytes.try_into()?,
            total_bytes: value.total_bytes.try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn count_entities<T: ConnectionTrait>(
    db: &T,
    block_number: BlockNumber,
) -> Result<BlockEntitiesCount> {
    DbBlockEntitiesCount::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::COUNT_ENTITIES_BY_BLOCK,
        [block_number.into()],
    ))
    .one(db)
    .await
    .context("Failed to count entities by block")?
    .expect("Entity counts will always return a row")
    .try_into()
}

#[instrument(skip(db))]
pub async fn storage_usage<T: ConnectionTrait>(
    db: &T,
    block_number: BlockNumber,
) -> Result<BlockStorageUsage> {
    DbBlockStorageUsage::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::STORAGE_USAGE_BY_BLOCK,
        [block_number.into()],
    ))
    .one(db)
    .await
    .context("Failed to get storage usage by block")?
    .expect("Storage usage will always return a row")
    .try_into()
}
