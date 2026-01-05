use std::ops::Add;

use anyhow::{Context, Result};
use chrono::Duration;
use golem_base_indexer_entity::blocks;
use sea_orm::{prelude::*, DbBackend, FromQueryResult, QueryOrder, QuerySelect, Statement};
use tracing::instrument;

use crate::types::{
    BlockConsensusInfo, BlockEntitiesCount, BlockNewData, BlockNumber, BlockStorageDiff,
    BlockStorageUsage, ConsensusBlocksInfo,
};

use super::sql;

#[derive(Debug, FromQueryResult)]
struct DbBlockEntitiesCount {
    pub create_count: i64,
    pub update_count: i64,
    pub expire_count: i64,
    pub delete_count: i64,
    pub extend_count: i64,
    pub changeowner_count: i64,
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
            changeowner_count: value.changeowner_count.try_into()?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbBlockNumber {
    pub block_number: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbBlockStorageDiff {
    pub block_number: i64,
    pub storage_diff: i64,
}

impl TryFrom<DbBlockStorageDiff> for BlockStorageDiff {
    type Error = anyhow::Error;

    fn try_from(value: DbBlockStorageDiff) -> Result<Self> {
        Ok(Self {
            block_number: value.block_number.try_into()?,
            storage_diff: value.storage_diff,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbBlockTotalStorageUsage {
    pub block_number: i64,
    pub storage_usage: i64,
}

impl TryFrom<DbBlockTotalStorageUsage> for BlockStorageUsage {
    type Error = anyhow::Error;

    fn try_from(value: DbBlockTotalStorageUsage) -> Result<Self> {
        Ok(Self {
            block_number: value.block_number.try_into()?,
            storage_usage: value.storage_usage.try_into()?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbBlockNewData {
    pub new_data: i64,
}

impl TryFrom<DbBlockNewData> for BlockNewData {
    type Error = anyhow::Error;

    fn try_from(value: DbBlockNewData) -> Result<Self> {
        Ok(Self {
            new_data: value.new_data.try_into()?,
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
pub async fn total_storage_usage<T: ConnectionTrait>(
    db: &T,
    block_number: BlockNumber,
) -> Result<Option<BlockStorageUsage>> {
    DbBlockTotalStorageUsage::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::TOTAL_STORAGE_USAGE_BY_BLOCK,
        [block_number.into()],
    ))
    .one(db)
    .await
    .context("Failed to get storage usage by block")?
    .map(TryInto::try_into)
    .transpose()
}

#[instrument(skip(db))]
pub async fn new_data<T: ConnectionTrait>(
    db: &T,
    block_number: BlockNumber,
) -> Result<BlockNewData> {
    DbBlockNewData::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::NEW_DATA_BY_BLOCK,
        [block_number.into()],
    ))
    .one(db)
    .await
    .context("Failed to get storage usage by block")?
    .expect("Block new data query will always return a row")
    .try_into()
}

#[instrument(skip(db))]
pub async fn latest_block_number<T: ConnectionTrait>(db: &T) -> Result<Option<BlockNumber>> {
    blocks::Entity::find()
        .select_only()
        .column(blocks::Column::Number)
        .filter(blocks::Column::Consensus.eq(true))
        .order_by_desc(blocks::Column::Number)
        .into_tuple()
        .one(db)
        .await?
        .map(|v: (i64,)| v.0.try_into())
        .transpose()
        .map_err(Into::into)
}

#[instrument(skip(db))]
pub async fn oldest_unprocessed_stats<T: ConnectionTrait>(db: &T) -> Result<Option<BlockNumber>> {
    Ok(DbBlockNumber::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        sql::OLDEST_UNPROCESSED_BLOCK_STATS,
    ))
    .one(db)
    .await
    .context("Failed to get oldest_unprocessed_block")?
    .map(|v| v.block_number.try_into())
    .transpose()?)
}

#[instrument(skip(db, blocks))]
pub async fn storage_diff<T: ConnectionTrait>(
    db: &T,
    blocks: impl Iterator<Item = BlockNumber>,
) -> Result<Vec<BlockStorageDiff>> {
    let blocks: Vec<i64> = blocks
        .map(|v| v.try_into())
        .collect::<Result<Vec<_>, _>>()?;
    DbBlockStorageDiff::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::STORAGE_DIFF_BY_BLOCK,
        [blocks.into()],
    ))
    .all(db)
    .await
    .context("Failed to get storage diff")?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

#[instrument(skip(db, updates))]
pub async fn update_stats<T: ConnectionTrait>(
    db: &T,
    updates: impl Iterator<Item = BlockStorageUsage>,
) -> Result<()> {
    let updates: Vec<(i64, i64)> = updates
        .map(|v| -> Result<_> {
            let bn: i64 = v.block_number.try_into()?;
            let storage_usage: i64 = v.storage_usage.try_into()?;
            Ok((bn, storage_usage))
        })
        .collect::<Result<_>>()?;

    let values_placeholders = (0..updates.len())
        .map(|i| format!("(${}, ${})", i * 2 + 1, i * 2 + 2))
        .collect::<Vec<_>>()
        .join(",");

    let query = format!(
        "{} {values_placeholders} {}",
        sql::UPDATE_BLOCK_STATS_PREFIX,
        sql::UPDATE_BLOCK_STATS_SUFFIX
    );
    let values: Vec<sea_orm::Value> = updates
        .into_iter()
        .flat_map(|v| vec![v.0.into(), v.1.into()].into_iter())
        .collect();

    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        query,
        values,
    ))
    .await?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn mark_stats_dirty<T: ConnectionTrait>(db: &T, block_number: BlockNumber) -> Result<()> {
    let block_number: i64 = block_number.try_into()?;
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::MARK_STATS_DIRTY,
        [block_number.into()],
    ))
    .await?;

    Ok(())
}

pub fn consensus_info(
    block_number: BlockNumber,
    blocks_info: ConsensusBlocksInfo,
) -> Result<BlockConsensusInfo> {
    let (status, expected_safe_at_timestamp) = if block_number <= blocks_info.finalized.block_number
    {
        ("finalized".to_string(), None)
    } else if block_number <= blocks_info.safe.block_number {
        ("safe".to_string(), None)
    } else if block_number <= blocks_info.latest.block_number {
        (
            "unsafe".to_string(),
            Some(blocks_info.safe.timestamp.add(Duration::minutes(20))),
        )
    } else {
        // Requested block number is greater than latest
        ("unknown".to_string(), None)
    };

    Ok(BlockConsensusInfo {
        status,
        expected_safe_at_timestamp,
    })
}
