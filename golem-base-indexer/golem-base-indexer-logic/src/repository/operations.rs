use anyhow::{anyhow, Context, Result};
use golem_base_indexer_entity::{
    blocks, golem_base_numeric_annotations, golem_base_operations, golem_base_string_annotations,
    sea_orm_active_enums::GolemBaseOperationType,
};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
    DbBackend, FromQueryResult, QueryOrder, QuerySelect, Statement,
};
use tracing::instrument;

use crate::{
    pagination::paginate_try_from,
    types::{
        BlockNumber, BlockNumberOrHashFilter, EntityKey, ListOperationsFilter, Operation,
        OperationData, OperationMetadata, OperationView, OperationsCount, OperationsFilter,
        PaginationMetadata, PaginationParams, TxHash,
    },
};

use super::sql;

#[derive(FromQueryResult)]
pub struct FullOperationIndex {
    pub block_number: i32,
    pub transaction_index: i32,
    pub operation_index: i64,
}

#[derive(Debug)]
enum DbBlockNumberOrHash {
    Number(i32),
    Hash(Vec<u8>),
}

#[derive(Debug)]
struct DbListOperationsFilter {
    pub pagination: PaginationParams,
    pub operation_type: Option<GolemBaseOperationType>,
    pub operations_filter: DbOperationsFilter,
}

#[derive(Debug)]
struct DbOperationsFilter {
    pub entity_key: Option<Vec<u8>>,
    pub sender: Option<Vec<u8>>,
    pub block_number_or_hash: Option<DbBlockNumberOrHash>,
    pub transaction_hash: Option<Vec<u8>>,
}

#[derive(Debug, FromQueryResult)]
struct OperationGroupCount {
    operation: GolemBaseOperationType,
    count: i64,
}

impl TryFrom<BlockNumberOrHashFilter> for DbBlockNumberOrHash {
    type Error = anyhow::Error;
    fn try_from(value: BlockNumberOrHashFilter) -> Result<Self> {
        Ok(match value {
            BlockNumberOrHashFilter::Number(number) => Self::Number(number.try_into()?),
            BlockNumberOrHashFilter::Hash(hash) => Self::Hash(hash.as_slice().into()),
        })
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

impl TryFrom<ListOperationsFilter> for DbListOperationsFilter {
    type Error = anyhow::Error;

    fn try_from(v: ListOperationsFilter) -> Result<Self> {
        Ok(Self {
            pagination: v.pagination,
            operation_type: v.operation_type.map(|op| match op {
                OperationData::Create(_, _) => GolemBaseOperationType::Create,
                OperationData::Update(_, _) => GolemBaseOperationType::Update,
                OperationData::Delete => GolemBaseOperationType::Delete,
                OperationData::Extend(_) => GolemBaseOperationType::Extend,
            }),
            operations_filter: v.operations_filter.try_into()?,
        })
    }
}

impl TryFrom<OperationsFilter> for DbOperationsFilter {
    type Error = anyhow::Error;

    fn try_from(v: OperationsFilter) -> Result<Self> {
        Ok(Self {
            entity_key: v.entity_key.map(|key| key.as_slice().into()),
            sender: v.sender.map(|s| s.as_slice().into()),
            block_number_or_hash: v.block_number_or_hash.map(TryInto::try_into).transpose()?,
            transaction_hash: v.transaction_hash.map(|hash| hash.as_slice().into()),
        })
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

impl TryFrom<(golem_base_operations::Model, Option<blocks::Model>)> for OperationView {
    type Error = anyhow::Error;

    fn try_from(v: (golem_base_operations::Model, Option<blocks::Model>)) -> Result<Self> {
        let (op, block) = v;

        Ok(Self {
            op: Operation::try_from(op.clone())?,
            block_number: block
                .ok_or(anyhow!("missing block number"))?
                .number
                .try_into()?,
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

impl From<GolemBaseOperationType> for OperationData {
    fn from(value: GolemBaseOperationType) -> Self {
        match value {
            GolemBaseOperationType::Create => OperationData::Create(Vec::new().into(), 0),
            GolemBaseOperationType::Update => OperationData::Update(Vec::new().into(), 0),
            GolemBaseOperationType::Delete => OperationData::Delete,
            GolemBaseOperationType::Extend => OperationData::Extend(0),
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

fn filtered_operations(
    filter: DbOperationsFilter,
    cond_join_blocks: bool,
) -> Select<golem_base_operations::Entity> {
    let mut q = golem_base_operations::Entity::find();

    if let Some(key) = filter.entity_key {
        q = q.filter(golem_base_operations::Column::EntityKey.eq(key));
    }

    if let Some(sender) = filter.sender {
        q = q.filter(golem_base_operations::Column::Sender.eq(sender));
    }

    q = match filter.block_number_or_hash {
        Some(DbBlockNumberOrHash::Number(number)) if cond_join_blocks => q
            .join(
                sea_orm::JoinType::LeftJoin,
                golem_base_operations::Relation::Blocks.def(),
            )
            .filter(blocks::Column::Number.eq(number)),
        Some(DbBlockNumberOrHash::Number(number)) => q.filter(blocks::Column::Number.eq(number)),
        Some(DbBlockNumberOrHash::Hash(hash)) => {
            q.filter(golem_base_operations::Column::BlockHash.eq(hash))
        }
        _ => q,
    };
    if let Some(transaction_hash) = filter.transaction_hash {
        q = q.filter(golem_base_operations::Column::TransactionHash.eq(transaction_hash));
    }
    q
}

#[instrument(skip(db))]
pub async fn list_operations<T: ConnectionTrait>(
    db: &T,
    filter: ListOperationsFilter,
) -> Result<(Vec<OperationView>, PaginationMetadata)> {
    let filter: DbListOperationsFilter = filter.try_into()?;
    let mut query = filtered_operations(filter.operations_filter, false);

    if let Some(operation_type) = filter.operation_type {
        query = query.filter(golem_base_operations::Column::Operation.eq(operation_type));
    }
    let query_with_blocks = query
        .join(
            sea_orm::JoinType::LeftJoin,
            golem_base_operations::Relation::Blocks.def(),
        )
        .select_also(blocks::Entity);

    let paginator = query_with_blocks
        .order_by_asc(golem_base_operations::Column::TransactionHash)
        .order_by_asc(golem_base_operations::Column::Index)
        .paginate(db, filter.pagination.page_size);

    paginate_try_from(paginator, filter.pagination).await
}

#[instrument(skip(db))]
pub async fn count_operations<T: ConnectionTrait>(
    db: &T,
    filter: OperationsFilter,
) -> Result<OperationsCount> {
    let query = filtered_operations(filter.clone().try_into()?, true);

    let rows: Vec<OperationGroupCount> = query
        .select_only()
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
