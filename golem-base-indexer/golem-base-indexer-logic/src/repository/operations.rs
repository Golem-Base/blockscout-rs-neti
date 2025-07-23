use anyhow::{anyhow, Context, Result};
use golem_base_indexer_entity::{
    golem_base_operations, sea_orm_active_enums::GolemBaseOperationType,
};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
    DbBackend, FromQueryResult, QueryOrder, Statement,
};
use tracing::instrument;

use crate::types::{BlockNumber, EntityKey, Operation, OperationData, OperationMetadata, TxHash};

use super::sql;

#[derive(FromQueryResult)]
pub struct FullOperationIndex {
    pub block_number: i32,
    pub transaction_index: i32,
    pub operation_index: i64,
}

impl TryFrom<golem_base_operations::Model> for Operation {
    type Error = anyhow::Error;

    fn try_from(v: golem_base_operations::Model) -> Result<Self> {
        let data = match v.operation {
            GolemBaseOperationType::Create => OperationData::create(
                v.data
                    .ok_or(anyhow!("Update operation in db with no data"))?
                    .into(),
                v.btl
                    .ok_or(anyhow!("Update operation in db with no btl"))?
                    .try_into()?,
            ),
            GolemBaseOperationType::Update => OperationData::update(
                v.data
                    .ok_or(anyhow!("Update operation in db with no data"))?
                    .into(),
                v.btl
                    .ok_or(anyhow!("Update operation in db with no btl"))?
                    .try_into()?,
            ),
            GolemBaseOperationType::Delete => OperationData::delete(),
            GolemBaseOperationType::Extend => OperationData::extend(
                v.btl
                    .ok_or(anyhow!("Extend operation in db with no btl"))?
                    .try_into()?,
            ),
        };
        Ok(Self {
            operation: data,
            metadata: OperationMetadata {
                entity_key: v.entity_key.as_slice().try_into()?,
                sender: v.sender.as_slice().try_into()?,
                tx_hash: v.transaction_hash.as_slice().try_into()?,
                block_hash: v.block_hash.as_slice().try_into()?,
                index: v.index.try_into()?,
            },
        })
    }
}

impl From<&OperationData> for GolemBaseOperationType {
    fn from(value: &OperationData) -> Self {
        match value {
            OperationData::Create(_, _) => GolemBaseOperationType::Create,
            OperationData::Update(_, _) => GolemBaseOperationType::Update,
            OperationData::Delete => GolemBaseOperationType::Delete,
            OperationData::Extend(_) => GolemBaseOperationType::Extend,
        }
    }
}

impl TryFrom<Operation> for golem_base_operations::ActiveModel {
    type Error = anyhow::Error;
    fn try_from(op: Operation) -> std::result::Result<Self, Self::Error> {
        let md = op.metadata;
        Ok(Self {
            entity_key: Set(md.entity_key.as_slice().into()),
            sender: Set(md.sender.as_slice().into()),
            operation: Set((&op.operation).into()),
            data: Set(op.operation.data().map(|v| v.to_owned().into())),
            btl: Set(op.operation.btl().map(Into::into)),
            transaction_hash: Set(md.tx_hash.as_slice().into()),
            block_hash: Set(md.block_hash.as_slice().into()),
            index: Set(md.index.try_into()?),
            inserted_at: NotSet,
        })
    }
}

#[instrument(skip(db))]
pub async fn insert_operation<T: ConnectionTrait>(db: &T, op: Operation) -> Result<()> {
    golem_base_operations::ActiveModel {
        ..op.clone().try_into()?
    }
    .insert(db)
    .await
    .with_context(|| format!("Failed to insert operation: {op:?}"))?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn get_latest_update<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Option<(BlockNumber, u64, u64)>> {
    let res = FullOperationIndex::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::GET_LATEST_UPDATE_OPERATION_INDEX,
        [entity_key.as_slice().into()],
    ))
    .one(db)
    .await
    .with_context(|| format!("Failed to get latest update: {entity_key}"))?;

    res.map(|v| -> Result<_> {
        Ok((
            v.block_number.try_into()?,
            v.transaction_index.try_into()?,
            v.operation_index.try_into()?,
        ))
    })
    .transpose()
    .with_context(|| format!("Failed to get latest update: {entity_key}"))
}

#[instrument(skip(db))]
pub async fn get_operation<T: ConnectionTrait>(
    db: &T,
    tx_hash: TxHash,
    index: u64,
) -> Result<Option<Operation>> {
    golem_base_operations::Entity::find_by_id((tx_hash.as_slice().into(), index.try_into()?))
        .one(db)
        .await
        .with_context(|| format!("Failed to get operation. tx_hash={tx_hash}, index={index}"))?
        .map(|v| {
            v.try_into().with_context(|| {
                format!("Failed to convert operation. tx_hash={tx_hash}, index={index}")
            })
        })
        .transpose()
}

#[instrument(skip(db))]
pub async fn list_operations<T: ConnectionTrait>(db: &T) -> Result<Vec<Operation>> {
    golem_base_operations::Entity::find()
        .order_by_asc(golem_base_operations::Column::TransactionHash)
        .order_by_asc(golem_base_operations::Column::Index)
        .all(db)
        .await
        .context("Failed to list operations")?
        .into_iter()
        .map(Operation::try_from)
        .collect()
}

#[instrument(skip(db))]
pub async fn find_create_operation<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Option<Operation>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    golem_base_operations::Entity::find()
        .filter(golem_base_operations::Column::EntityKey.eq(entity_key))
        .filter(golem_base_operations::Column::Operation.eq(GolemBaseOperationType::Create))
        .one(db)
        .await
        .context("Failed to find create operation")?
        .map(Operation::try_from)
        .transpose()
}
