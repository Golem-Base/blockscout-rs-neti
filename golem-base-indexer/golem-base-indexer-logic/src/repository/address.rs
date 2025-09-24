use anyhow::{Context, Result};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use tracing::instrument;

use crate::types::{
    Address, AddressActivity, AddressEntitiesCount, AddressLeaderboardRanks, AddressTxsCount,
};

use super::sql;

#[derive(FromQueryResult)]
pub struct FullOperationIndex {
    pub block_number: i32,
    pub transaction_index: i32,
    pub operation_index: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbAddressEntitiesCount {
    pub total_entities: i64,
    pub size_of_active_entities: i64,
    pub active_entities: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbAddressTxsCount {
    pub total_transactions: i64,
    pub failed_transactions: i64,
}

#[derive(Debug, FromQueryResult)]
struct DbAddressActivity {
    pub first_seen_timestamp: Option<chrono::NaiveDateTime>,
    pub last_seen_timestamp: Option<chrono::NaiveDateTime>,
    pub first_seen_block: Option<i32>,
    pub last_seen_block: Option<i32>,
}

#[derive(Debug, FromQueryResult)]
struct DbAddressLeaderboardRanks {
    pub biggest_spenders: Option<i64>,
    pub entities_created: Option<i64>,
    pub entities_owned: Option<i64>,
    pub data_owned: Option<i64>,
}

impl From<DbAddressActivity> for AddressActivity {
    fn from(v: DbAddressActivity) -> Self {
        Self {
            first_seen_timestamp: v.first_seen_timestamp.map(|v| v.and_utc()),
            last_seen_timestamp: v.last_seen_timestamp.map(|v| v.and_utc()),
            first_seen_block: v.first_seen_block.map(|v| v as u64),
            last_seen_block: v.last_seen_block.map(|v| v as u64),
        }
    }
}

impl TryFrom<DbAddressEntitiesCount> for AddressEntitiesCount {
    type Error = anyhow::Error;

    fn try_from(value: DbAddressEntitiesCount) -> Result<Self> {
        Ok(Self {
            total_entities: value.total_entities.try_into()?,
            size_of_active_entities: value.size_of_active_entities.try_into()?,
            active_entities: value.active_entities.try_into()?,
        })
    }
}

impl TryFrom<DbAddressTxsCount> for AddressTxsCount {
    type Error = anyhow::Error;

    fn try_from(value: DbAddressTxsCount) -> Result<Self> {
        Ok(Self {
            total_transactions: value.total_transactions.try_into()?,
            failed_transactions: value.failed_transactions.try_into()?,
        })
    }
}

impl TryFrom<DbAddressLeaderboardRanks> for AddressLeaderboardRanks {
    type Error = anyhow::Error;

    fn try_from(value: DbAddressLeaderboardRanks) -> Result<Self> {
        Ok(Self {
            biggest_spenders: value.biggest_spenders.unwrap_or(0).try_into()?,
            entities_created: value.entities_created.unwrap_or(0).try_into()?,
            entities_owned: value.entities_owned.unwrap_or(0).try_into()?,
            data_owned: value.data_owned.unwrap_or(0).try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn count_entities<T: ConnectionTrait>(
    db: &T,
    owner: Address,
) -> Result<AddressEntitiesCount> {
    let res = DbAddressEntitiesCount::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::COUNT_ENTITIES_BY_OWNER,
        [owner.as_slice().into()],
    ))
    .one(db)
    .await
    .context("Failed to count entities by address")?
    .expect("Count will always return a row")
    .try_into()?;

    Ok(res)
}

#[instrument(skip(db))]
pub async fn count_txs<T: ConnectionTrait>(db: &T, owner: Address) -> Result<AddressTxsCount> {
    let res = DbAddressTxsCount::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::COUNT_TRANSACTIONS_BY_OWNER,
        [owner.as_slice().into()],
    ))
    .one(db)
    .await
    .context("Failed to count txs by address")?
    .expect("Count will always return a row")
    .try_into()?;

    Ok(res)
}

#[instrument(skip(db))]
pub async fn get_address_activity<T: ConnectionTrait>(
    db: &T,
    owner: Address,
) -> Result<AddressActivity> {
    let address_activity = DbAddressActivity::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::GET_ADDRESS_ACTIVITY,
        [owner.as_slice().into()],
    ))
    .one(db)
    .await
    .context("Failed to get address activity")?
    .expect("Address activity will always return a row");

    Ok(address_activity.into())
}

#[instrument(skip(db))]
pub async fn get_address_leaderboard_ranks<T: ConnectionTrait>(
    db: &T,
    owner: Address,
) -> Result<AddressLeaderboardRanks> {
    let ranks = DbAddressLeaderboardRanks::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::ADDRESS_LEADERBOARD_RANKS,
        [owner.as_slice().into()],
    ))
    .one(db)
    .await
    .context("Failed to get address leaderboard ranks")?
    .expect("Address leaderboard ranks will always return a row")
    .try_into()?;

    Ok(ranks)
}
