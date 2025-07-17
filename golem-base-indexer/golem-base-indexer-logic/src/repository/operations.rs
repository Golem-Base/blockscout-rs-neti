use anyhow::Result;
use golem_base_indexer_entity::{
    golem_base_operations, sea_orm_active_enums::GolemBaseOperationType,
};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
    DbBackend, FromQueryResult, Statement,
};
use tracing::instrument;

use super::sql;

#[derive(FromQueryResult)]
pub struct FullOperationIndex {
    pub block_number: i64,
    pub transaction_index: i32,
    pub operation_index: i64,
}

#[derive(Debug)]
pub struct GolemBaseOperationCreate {
    pub entity_key: Vec<u8>,
    pub sender: Vec<u8>,
    pub operation: GolemBaseOperationType,
    pub data: Option<Vec<u8>>,
    pub btl: Option<Decimal>,
    pub transaction_hash: Vec<u8>,
    pub block_hash: Vec<u8>,
    pub index: i64,
}

#[instrument(
    name = "repository::operations::insert_operation",
    skip(db),
    level = "info"
)]
pub async fn insert_operation<T: ConnectionTrait>(
    db: &T,
    op: GolemBaseOperationCreate,
) -> Result<()> {
    golem_base_operations::ActiveModel {
        entity_key: Set(op.entity_key),
        sender: Set(op.sender),
        operation: Set(op.operation),
        data: Set(op.data),
        btl: Set(op.btl),
        transaction_hash: Set(op.transaction_hash),
        block_hash: Set(op.block_hash),
        index: Set(op.index),
        inserted_at: NotSet,
    }
    .insert(db)
    .await?;
    Ok(())
}

#[instrument(
    name = "repository::operations::get_latest_update",
    skip(db),
    level = "info"
)]
pub async fn get_latest_update<T: ConnectionTrait>(
    db: &T,
    entity_key: Vec<u8>,
) -> Result<Option<(i64, i32, i64)>> {
    let res = FullOperationIndex::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::GET_LATEST_UPDATE_OPERATION_INDEX,
        [entity_key.into()],
    ))
    .one(db)
    .await?;

    Ok(res.map(|v| (v.block_number, v.transaction_index, v.operation_index)))
}
