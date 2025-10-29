use crate::types::{BlockHash, Log, LogIndex, TxHash};
use anyhow::{Context, Result};
use optimism_children_indexer_entity::logs as EntityLogs;
use sea_orm::{prelude::*, Statement};
use tracing::instrument;

impl TryFrom<LogIndex> for (i32, Vec<u8>, Vec<u8>) {
    type Error = anyhow::Error;

    fn try_from(value: LogIndex) -> Result<Self> {
        Ok((
            value.index.try_into()?,
            value.transaction_hash.as_slice().into(),
            value.block_hash.as_slice().into(),
        ))
    }
}

impl TryFrom<EntityLogs::Model> for Log {
    type Error = anyhow::Error;

    fn try_from(value: EntityLogs::Model) -> Result<Self> {
        Ok(Self {
            data: value.data.into(),
            index: value.index.try_into()?,
            first_topic: value
                .first_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            second_topic: value
                .second_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            third_topic: value
                .third_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            fourth_topic: value
                .fourth_topic
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            tx_hash: value.transaction_hash.as_slice().try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn get_log<T: ConnectionTrait>(db: &T, log: LogIndex) -> Result<Option<Log>> {
    let id: (i32, Vec<u8>, Vec<u8>) = log.try_into()?;
    EntityLogs::Entity::find_by_id(id)
        .one(db)
        .await?
        .map(TryInto::try_into)
        .transpose()
}

#[instrument(skip(db))]
pub async fn finish_log_processing<T: ConnectionTrait>(
    db: &T,
    tx_hash: TxHash,
    block_hash: BlockHash,
    index: u64,
) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    let block_hash: Vec<u8> = block_hash.as_slice().into();
    let index: i64 = index.try_into()?;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from optimism_children_pending_logs where transaction_hash = $1 and block_hash = $2 and index = $3",
        [tx_hash.into(), block_hash.into(), index.into()],
    ))
    .await
    .context("Failed to finish tx cleanup - logs")?;
    Ok(())
}
