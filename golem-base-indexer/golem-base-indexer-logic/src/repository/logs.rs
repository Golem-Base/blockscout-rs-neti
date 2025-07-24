use super::sql::{FIND_LATEST_LOG, GET_LOGS};
use crate::{
    types::{EntityKey, Log, TxHash},
    well_known,
};
use alloy_primitives::B256;
use anyhow::{Context, Result};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use tracing::instrument;

#[derive(FromQueryResult)]
#[allow(dead_code)]
pub struct DbLog {
    pub data: Vec<u8>,
    pub index: i32,
    pub first_topic: Option<Vec<u8>>,
    pub second_topic: Option<Vec<u8>>,
    pub third_topic: Option<Vec<u8>>,
    pub fourth_topic: Option<Vec<u8>>,
    pub transaction_hash: Vec<u8>,
}

impl TryFrom<DbLog> for Log {
    type Error = anyhow::Error;

    fn try_from(v: DbLog) -> Result<Self> {
        Ok(Self {
            data: v.data.into(),
            index: v.index.try_into()?,
            first_topic: v.first_topic.map(|v| v.as_slice().try_into()).transpose()?,
            second_topic: v
                .second_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            third_topic: v.third_topic.map(|v| v.as_slice().try_into()).transpose()?,
            fourth_topic: v
                .fourth_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            tx_hash: v.transaction_hash.as_slice().try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn get_tx_logs<T: ConnectionTrait>(
    db: &T,
    tx_hash: TxHash,
    signature: B256,
) -> Result<Vec<Log>> {
    DbLog::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        GET_LOGS,
        [tx_hash.as_slice().into(), signature.as_slice().into()],
    ))
    .all(db)
    .await
    .with_context(|| format!("Failed to get tx logs - tx={tx_hash}, signature={signature}"))?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

#[instrument(skip(db))]
pub async fn find_latest_extend_log<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Option<Log>> {
    let signature = well_known::GOLEM_BASE_STORAGE_ENTITY_BTL_EXTENDED;
    DbLog::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        FIND_LATEST_LOG,
        [signature.as_slice().into(), entity_key.as_slice().into()],
    ))
    .one(db)
    .await?
    .map(TryInto::try_into)
    .transpose()
}
