use anyhow::{anyhow, Context, Result};
use golem_base_indexer_entity::{
    golem_base_numeric_annotations, golem_base_operations, golem_base_string_annotations,
    sea_orm_active_enums::GolemBaseOperationType,
};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
    DbBackend, FromQueryResult, QueryOrder, QuerySelect, Statement,
};
use tracing::instrument;

use crate::types::{
    BlockNumber, EntityKey, Operation, OperationData, OperationMetadata, OperationsCount,
    OperationsCounterFilter, OperationsFilter, PaginationMetadata, TxHash,
};

use super::sql;

#[derive(FromQueryResult)]
pub struct FullOperationIndex {
    pub block_number: i32,
    pub transaction_index: i32,
    pub operation_index: i64,
}

#[derive(Debug)]
pub struct DbOperationsFilter {
    pub page: u64,
    pub page_size: u64,
    pub entity_key: Option<Vec<u8>>,
    pub sender: Option<Vec<u8>>,
    pub operation_type: Option<GolemBaseOperationType>,
    pub block_hash: Option<Vec<u8>>,
    pub transaction_hash: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct DbOperationsCounterFilter {
    pub entity_key: Option<Vec<u8>>,
    pub sender: Option<Vec<u8>>,
    pub block_hash: Option<Vec<u8>>,
    pub transaction_hash: Option<Vec<u8>>,
}

#[derive(Debug, FromQueryResult)]
struct OperationGroupCount {
    operation: GolemBaseOperationType,
    count: i64,
}
impl Default for OperationsCount {
    fn default() -> Self {
        Self {
            create_count: 0,
            update_count: 0,
            delete_count: 0,
            extend_count: 0,
        }
    }
}

impl From<Vec<OperationGroupCount>> for OperationsCount {
    fn from(rows: Vec<OperationGroupCount>) -> Self {
        let mut counts = Self::default();

        for row in rows {
            match row.operation {
                GolemBaseOperationType::Create => counts.create_count = row.count as u64,
                GolemBaseOperationType::Update => counts.update_count = row.count as u64,
                GolemBaseOperationType::Delete => counts.delete_count = row.count as u64,
                GolemBaseOperationType::Extend => counts.extend_count = row.count as u64,
            }
        }

        counts
    }
}

impl From<OperationsFilter> for DbOperationsFilter {
    fn from(v: OperationsFilter) -> Self {
        Self {
            page: v.page.into(),
            page_size: v.page_size.into(),
            entity_key: v.entity_key.map(|key| key.as_slice().into()),
            sender: v.sender.map(|s| s.as_slice().into()),
            block_hash: v.block_hash.map(|hash| hash.as_slice().into()),
            transaction_hash: v.transaction_hash.map(|hash| hash.as_slice().into()),
            operation_type: v.operation_type.map(|op| match op {
                OperationData::Create(_, _) => GolemBaseOperationType::Create,
                OperationData::Update(_, _) => GolemBaseOperationType::Update,
                OperationData::Delete => GolemBaseOperationType::Delete,
                OperationData::Extend(_) => GolemBaseOperationType::Extend,
            }),
        }
    }
}

impl From<OperationsCounterFilter> for DbOperationsCounterFilter {
    fn from(v: OperationsCounterFilter) -> Self {
        Self {
            entity_key: v.entity_key.map(|key| key.as_slice().into()),
            sender: v.sender.map(|s| s.as_slice().into()),
            block_hash: v.block_hash.map(|hash| hash.as_slice().into()),
            transaction_hash: v.transaction_hash.map(|hash| hash.as_slice().into()),
        }
    }
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
                recipient: v.recipient.as_slice().try_into()?,
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
            recipient: Set(md.recipient.as_slice().into()),
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
pub async fn list_operations<T: ConnectionTrait>(
    db: &T,
    filter: DbOperationsFilter,
) -> Result<(Vec<Operation>, PaginationMetadata)> {
    let mut query = golem_base_operations::Entity::find();

    if let Some(sender) = filter.sender {
        query = query.filter(golem_base_operations::Column::Sender.eq(sender));
    }
    if let Some(operation_type) = filter.operation_type {
        query = query.filter(golem_base_operations::Column::Operation.eq(operation_type));
    }
    if let Some(block_hash) = filter.block_hash {
        query = query.filter(golem_base_operations::Column::BlockHash.eq(block_hash));
    }
    if let Some(transaction_hash) = filter.transaction_hash {
        query = query.filter(golem_base_operations::Column::TransactionHash.eq(transaction_hash));
    }

    let paginator = query
        .order_by_asc(golem_base_operations::Column::TransactionHash)
        .order_by_asc(golem_base_operations::Column::Index)
        .paginate(db, filter.page_size);

    let total_items = paginator
        .num_items()
        .await
        .context("Failed to count total items")?;
    let total_pages = paginator
        .num_pages()
        .await
        .context("Failed to count total pages")?;

    let page_index = filter.page.saturating_sub(1);
    let operations = paginator
        .fetch_page(page_index)
        .await
        .context("Failed to fetch paged operations")?
        .into_iter()
        .map(Operation::try_from)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to map operations")?;

    let pagination = PaginationMetadata {
        page: filter.page,
        page_size: filter.page_size,
        total_pages,
        total_items,
    };

    Ok((operations, pagination))
}

#[instrument(skip(db))]
pub async fn count_operations<T: ConnectionTrait>(
    db: &T,
    filter: DbOperationsCounterFilter,
) -> Result<OperationsCount> {
    let mut query = golem_base_operations::Entity::find().select_only();

    if let Some(sender) = filter.sender {
        query = query.filter(golem_base_operations::Column::Sender.eq(sender));
    }
    if let Some(block_hash) = filter.block_hash {
        query = query.filter(golem_base_operations::Column::BlockHash.eq(block_hash));
    }
    if let Some(transaction_hash) = filter.transaction_hash {
        query = query.filter(golem_base_operations::Column::TransactionHash.eq(transaction_hash));
    }
    if let Some(entity_key) = filter.entity_key {
        query = query.filter(golem_base_operations::Column::EntityKey.eq(entity_key));
    }

    let rows: Vec<OperationGroupCount> = query
        .column(golem_base_operations::Column::Operation)
        .expr_as(Expr::cust("COUNT(*)"), "count")
        .group_by(golem_base_operations::Column::Operation)
        .into_model::<OperationGroupCount>()
        .all(db)
        .await
        .context("Failed to count operations")?;

    Ok(rows.into())
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

#[instrument(skip(db))]
pub async fn find_delete_operation<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Option<Operation>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    golem_base_operations::Entity::find()
        .filter(golem_base_operations::Column::EntityKey.eq(entity_key))
        .filter(golem_base_operations::Column::Operation.eq(GolemBaseOperationType::Delete))
        .one(db)
        .await
        .context("Failed to find delete operation")?
        .map(Operation::try_from)
        .transpose()
}

#[instrument(skip(db))]
pub async fn find_latest_update_operation<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Option<Operation>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    golem_base_operations::Model::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql::FIND_LATEST_UPDATE_OPERATION,
        [entity_key.into()],
    ))
    .one(db)
    .await
    .context("Failed to find latest update operation")?
    .map(Operation::try_from)
    .transpose()
}

#[instrument(skip(db))]
pub async fn delete_by_tx_hash<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<()> {
    let db_tx_hash: Vec<u8> = tx_hash.as_slice().into();
    golem_base_string_annotations::Entity::delete_many()
        .filter(golem_base_string_annotations::Column::OperationTxHash.eq(db_tx_hash.clone()))
        .exec(db)
        .await?;
    golem_base_numeric_annotations::Entity::delete_many()
        .filter(golem_base_numeric_annotations::Column::OperationTxHash.eq(db_tx_hash.clone()))
        .exec(db)
        .await?;
    golem_base_operations::Entity::delete_many()
        .filter(golem_base_operations::Column::TransactionHash.eq(db_tx_hash))
        .exec(db)
        .await?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn find_latest_operation<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Option<Operation>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    golem_base_operations::Model::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql::FIND_LATEST_OPERATION,
        [entity_key.into()],
    ))
    .one(db)
    .await
    .context("Failed to find latest operation")?
    .map(Operation::try_from)
    .transpose()
}
