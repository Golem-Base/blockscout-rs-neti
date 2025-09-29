use anyhow::{anyhow, Context, Result};
use golem_base_indexer_entity::{
    golem_base_entities, golem_base_entity_history, golem_base_numeric_annotations,
    golem_base_string_annotations, sea_orm_active_enums::GolemBaseEntityStatusType,
};
use sea_orm::{
    prelude::*,
    sea_query::OnConflict,
    sqlx::types::chrono::Utc,
    ActiveValue::{NotSet, Set},
    Condition, FromQueryResult, Iterable, QueryOrder, Statement,
};
use tracing::instrument;

use crate::{
    golem_base::block_timestamp,
    model::entity_data_size_histogram,
    pagination::{paginate, paginate_try_from},
    repository::sql,
    types::{
        Address, Block, BlockNumber, Bytes, EntitiesFilter, Entity, EntityDataHistogram,
        EntityHistoryEntry, EntityHistoryFilter, EntityKey, EntityStatus, EntityWithExpTimestamp,
        FullEntity, FullOperationIndex, ListEntitiesFilter, OperationFilter, PaginationMetadata,
        TxHash,
    },
};

#[derive(Debug)]
pub struct GolemBaseEntityCreate {
    pub key: EntityKey,
    pub data: Bytes,
    pub sender: Address,
    pub created_at: TxHash,
    pub expires_at: BlockNumber,
}

#[derive(Debug)]
pub struct GolemBaseEntityUpdate {
    pub key: EntityKey,
    pub data: Bytes,
    pub sender: Address,
    pub updated_at: TxHash,
    pub expires_at: BlockNumber,
}

#[derive(Debug)]
pub struct GolemBaseEntityDelete {
    pub key: EntityKey,
    pub sender: Address,
    pub deleted_at_tx: TxHash,
    pub deleted_at_block: BlockNumber,
    pub status: EntityStatus,
}

#[derive(Debug)]
pub struct GolemBaseEntityExtend {
    pub key: EntityKey,
    pub sender: Address,
    pub extended_at: TxHash,
    pub expires_at: BlockNumber,
}

impl From<EntityStatus> for GolemBaseEntityStatusType {
    fn from(value: EntityStatus) -> Self {
        match value {
            EntityStatus::Active => GolemBaseEntityStatusType::Active,
            EntityStatus::Deleted => GolemBaseEntityStatusType::Deleted,
            EntityStatus::Expired => GolemBaseEntityStatusType::Expired,
        }
    }
}

impl From<GolemBaseEntityStatusType> for EntityStatus {
    fn from(value: GolemBaseEntityStatusType) -> Self {
        match value {
            GolemBaseEntityStatusType::Active => EntityStatus::Active,
            GolemBaseEntityStatusType::Deleted => EntityStatus::Deleted,
            GolemBaseEntityStatusType::Expired => EntityStatus::Expired,
        }
    }
}

impl TryFrom<golem_base_entities::Model> for Entity {
    type Error = anyhow::Error;

    fn try_from(value: golem_base_entities::Model) -> Result<Self> {
        Ok(Self {
            key: value.key.as_slice().try_into()?,
            data: value.data.map(|v| v.into()),
            status: value.status.into(),
            owner: value.owner.map(|v| v.as_slice().try_into()).transpose()?,
            created_at_tx_hash: value
                .created_at_tx_hash
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            last_updated_at_tx_hash: value.last_updated_at_tx_hash.as_slice().try_into()?,
            expires_at_block_number: value
                .expires_at_block_number
                .map(TryInto::try_into)
                .transpose()?,
        })
    }
}

impl TryFrom<entity_data_size_histogram::Model> for EntityDataHistogram {
    type Error = anyhow::Error;

    fn try_from(value: entity_data_size_histogram::Model) -> Result<Self> {
        Ok(Self {
            bucket: value.bucket.try_into()?,
            bin_start: value.bin_start.try_into()?,
            bin_end: value.bin_end.try_into()?,
            count: value.count.try_into()?,
        })
    }
}

impl EntityWithExpTimestamp {
    pub fn try_new(value: golem_base_entities::Model, reference_block: &Block) -> Result<Self> {
        let entity_base: Entity = value.try_into()?;
        let expires_at_timestamp = entity_base
            .expires_at_block_number
            .and_then(|v| block_timestamp(v, reference_block));

        Ok(Self {
            key: entity_base.key,
            data: entity_base.data,
            owner: entity_base.owner,
            status: entity_base.status,
            created_at_tx_hash: entity_base.created_at_tx_hash,
            last_updated_at_tx_hash: entity_base.last_updated_at_tx_hash,
            expires_at_block_number: entity_base.expires_at_block_number,
            expires_at_timestamp,
        })
    }
}

impl EntityHistoryEntry {
    fn try_new(value: golem_base_entity_history::Model) -> Result<Self> {
        let reference_block = Block {
            hash: value.block_hash.as_slice().try_into()?,
            number: value.block_number.try_into()?,
            timestamp: value.block_timestamp.and_utc(),
        };
        let expires_at_block_number: Option<BlockNumber> = value
            .expires_at_block_number
            .map(|v| v.try_into())
            .transpose()?;
        let prev_expires_at_block_number: Option<BlockNumber> = value
            .prev_expires_at_block_number
            .map(|v| v.try_into())
            .transpose()?;

        let expires_at_timestamp =
            expires_at_block_number.and_then(|v| block_timestamp(v, &reference_block));

        let prev_expires_at_timestamp =
            prev_expires_at_block_number.and_then(|expires_at_block_number| {
                block_timestamp(expires_at_block_number, &reference_block)
            });

        Ok(Self {
            entity_key: value.entity_key.as_slice().try_into()?,
            block_number: value.block_number.try_into()?,
            block_hash: value.block_hash.as_slice().try_into()?,
            transaction_hash: value.transaction_hash.as_slice().try_into()?,
            tx_index: value.tx_index.try_into()?,
            op_index: value.op_index.try_into()?,
            block_timestamp: value.block_timestamp.and_utc(),
            owner: value.owner.map(|v| v.as_slice().try_into()).transpose()?,
            sender: value.sender.as_slice().try_into()?,
            operation: value.operation.into(),
            data: value.data.map(|v| v.into()),
            prev_data: value.prev_data.map(|v| v.into()),
            status: value.status.into(),
            prev_status: value.prev_status.map(|v| v.into()),
            expires_at_block_number,
            prev_expires_at_block_number,
            expires_at_timestamp,
            prev_expires_at_timestamp,
            btl: value.btl.map(|v| v.try_into()).transpose()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn get_entity<T: ConnectionTrait>(db: &T, key: EntityKey) -> Result<Option<Entity>> {
    let key: Vec<u8> = key.as_slice().into();
    golem_base_entities::Entity::find_by_id(key.clone())
        .one(db)
        .await
        .with_context(|| format!("Failed to get entity: {key:?}"))?
        .map(|v| v.try_into())
        .transpose()
}

fn filtered_entities(filter: EntitiesFilter) -> Select<golem_base_entities::Entity> {
    let mut q = golem_base_entities::Entity::find().order_by_asc(golem_base_entities::Column::Key);

    if let Some(status) = filter.status {
        let status: GolemBaseEntityStatusType = status.into();
        q = q.filter(golem_base_entities::Column::Status.eq(status));
    }

    if let Some(ann) = filter.string_annotation {
        q = q
            .left_join(golem_base_string_annotations::Entity)
            .filter(golem_base_string_annotations::Column::Key.eq(ann.key))
            .filter(golem_base_string_annotations::Column::Value.eq(ann.value));
    }

    if let Some(ann) = filter.numeric_annotation {
        q = q
            .left_join(golem_base_numeric_annotations::Entity)
            .filter(golem_base_numeric_annotations::Column::Key.eq(ann.key))
            .filter(golem_base_numeric_annotations::Column::Value.eq(ann.value));
    }

    if let Some(owner) = filter.owner {
        let owner: Vec<u8> = owner.as_slice().into();
        q = q.filter(golem_base_entities::Column::Owner.eq(owner));
    }

    q
}

#[instrument(skip(db))]
pub async fn list_entities<T: ConnectionTrait>(
    db: &T,
    filter: ListEntitiesFilter,
) -> Result<(Vec<Entity>, PaginationMetadata)> {
    let q = filtered_entities(filter.entities_filter);
    let paginator = q.paginate(db, filter.pagination.page_size);

    paginate_try_from(paginator, filter.pagination).await
}

#[instrument(skip(db))]
pub async fn count_entities<T: ConnectionTrait>(db: &T, filter: EntitiesFilter) -> Result<u64> {
    let q = filtered_entities(filter);
    q.count(db).await.context("Failed to count entities")
}

#[instrument(skip(db))]
pub async fn get_full_entity<T: ConnectionTrait>(
    db: &T,
    key: EntityKey,
) -> Result<Option<FullEntity>> {
    let dbkey: Vec<u8> = key.as_slice().into();
    let entity = golem_base_entities::Entity::find_by_id(dbkey.clone())
        .one(db)
        .await
        .with_context(|| format!("Failed to get entity: {key:?}"))?;
    let entity = if let Some(v) = entity {
        v
    } else {
        return Ok(None);
    };

    let current_block = super::blockscout::get_current_block(db)
        .await?
        .ok_or(anyhow!("No blocks indexed yet"))?;
    let create_operation = super::operations::find_create_operation(db, key).await?;
    let create_block = match create_operation {
        Some(ref op) => super::blockscout::get_block(db, op.metadata.block_hash).await?,
        None => None,
    };

    let expires_at_timestamp = entity
        .expires_at_block_number
        .and_then(|v| block_timestamp(v as u64, &current_block));

    let latest_operation = super::operations::find_latest_operation(db, key)
        .await?
        .ok_or(anyhow!("Entity with no operations"))?;
    let latest_op_block = super::blockscout::get_block(db, latest_operation.metadata.block_hash)
        .await?
        .ok_or(anyhow!("Operation with invalid block"))?;

    Ok(Some(FullEntity {
        key: entity.key.as_slice().try_into()?,
        data: entity.data.map(|v| v.into()),
        status: entity.status.into(),
        created_at_tx_hash: entity
            .created_at_tx_hash
            .map(|v| v.as_slice().try_into())
            .transpose()?,
        created_at_operation_index: create_operation.as_ref().map(|v| v.metadata.index),
        created_at_block_number: create_block.as_ref().map(|v| v.number),
        created_at_timestamp: create_block.as_ref().map(|v| v.timestamp),
        updated_at_tx_hash: latest_operation.metadata.tx_hash,
        updated_at_operation_index: latest_operation.metadata.index,
        updated_at_block_number: latest_op_block.number,
        updated_at_timestamp: latest_op_block.timestamp,
        expires_at_block_number: entity
            .expires_at_block_number
            .map(TryInto::try_into)
            .transpose()?,
        expires_at_timestamp,
        owner: entity.owner.map(|v| v.as_slice().try_into()).transpose()?,
        gas_used: Default::default(), // FIXME when we have gas per operation
        fees_paid: Default::default(), // FIXME when we have gas per operation
    }))
}

#[instrument(skip(db))]
pub async fn find_by_tx_hash<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<Vec<Entity>> {
    let db_tx_hash: Vec<u8> = tx_hash.as_slice().into();
    golem_base_entities::Model::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql::FIND_ENTITIES_BY_TX_HASH,
        [db_tx_hash.into()],
    ))
    .all(db)
    .await
    .with_context(|| format!("Failed to find entities by tx hash: {tx_hash}"))?
    .into_iter()
    .map(|v| v.try_into())
    .collect()
}

#[instrument(skip(db))]
pub async fn drop_entity<T: ConnectionTrait>(db: &T, entity: EntityKey) -> Result<()> {
    let entity: Vec<u8> = entity.as_slice().into();
    golem_base_entities::Entity::delete_by_id(entity)
        .exec(db)
        .await
        .context("Failed to drop entity")?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn get_latest_entity_history_entry<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
    older_than: Option<FullOperationIndex>,
) -> Result<Option<EntityHistoryEntry>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();

    let mut q = golem_base_entity_history::Entity::find()
        .filter(golem_base_entity_history::Column::EntityKey.eq(entity_key));

    if let Some(FullOperationIndex {
        block_number,
        tx_index,
        op_index,
    }) = older_than
    {
        use golem_base_entity_history::Column;
        q = q.filter(
            Condition::any()
                .add(Column::BlockNumber.lt(block_number))
                .add(
                    Column::BlockNumber
                        .eq(block_number)
                        .and(Column::TxIndex.lt(tx_index)),
                )
                .add(
                    Column::BlockNumber
                        .eq(block_number)
                        .and(Column::TxIndex.eq(tx_index))
                        .and(Column::OpIndex.lt(op_index)),
                ),
        )
    }

    q.order_by_desc(golem_base_entity_history::Column::BlockNumber)
        .order_by_desc(golem_base_entity_history::Column::TxIndex)
        .order_by_desc(golem_base_entity_history::Column::OpIndex)
        .one(db)
        .await
        .context("Failed to get latest history entry")?
        .map(EntityHistoryEntry::try_new)
        .transpose()
}

#[instrument(skip(db))]
pub async fn get_oldest_entity_history_entry<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
    newer_than: FullOperationIndex,
) -> Result<Option<EntityHistoryEntry>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();

    use golem_base_entity_history::Column;
    let FullOperationIndex {
        block_number,
        tx_index,
        op_index,
    } = newer_than;
    golem_base_entity_history::Entity::find()
        .filter(golem_base_entity_history::Column::EntityKey.eq(entity_key))
        .filter(
            Condition::any()
                .add(Column::BlockNumber.gt(block_number))
                .add(
                    Column::BlockNumber
                        .eq(block_number)
                        .and(Column::TxIndex.gt(tx_index)),
                )
                .add(
                    Column::BlockNumber
                        .eq(block_number)
                        .and(Column::TxIndex.eq(tx_index))
                        .and(Column::OpIndex.gt(op_index)),
                ),
        )
        .order_by_asc(golem_base_entity_history::Column::BlockNumber)
        .order_by_asc(golem_base_entity_history::Column::TxIndex)
        .order_by_asc(golem_base_entity_history::Column::OpIndex)
        .one(db)
        .await
        .context("Failed to get oldest history entry")?
        .map(EntityHistoryEntry::try_new)
        .transpose()
}

#[instrument(skip(db))]
pub async fn get_entity_history<T: ConnectionTrait>(
    db: &T,
    filter: EntityHistoryFilter,
) -> Result<(Vec<EntityHistoryEntry>, PaginationMetadata)> {
    let entity_key: Vec<u8> = filter.entity_key.as_slice().into();

    let paginator = golem_base_entity_history::Entity::find()
        .filter(golem_base_entity_history::Column::EntityKey.eq(entity_key))
        .order_by_asc(golem_base_entity_history::Column::BlockNumber)
        .order_by_asc(golem_base_entity_history::Column::TxIndex)
        .order_by_asc(golem_base_entity_history::Column::OpIndex)
        .paginate(db, filter.pagination.page_size);

    let (items, pagination_metadata) = paginate(paginator, filter.pagination).await?;

    Ok((
        items
            .into_iter()
            .map(EntityHistoryEntry::try_new)
            .collect::<Result<Vec<_>>>()?,
        pagination_metadata,
    ))
}

#[instrument(skip(db))]
pub async fn get_entity_operation<T: ConnectionTrait>(
    db: &T,
    filter: OperationFilter,
) -> Result<Option<EntityHistoryEntry>> {
    let tx_hash: Vec<u8> = filter.tx_hash.as_slice().into();

    golem_base_entity_history::Entity::find()
        .filter(golem_base_entity_history::Column::TransactionHash.eq(tx_hash))
        .filter(golem_base_entity_history::Column::OpIndex.eq(filter.op_index as i64))
        .one(db)
        .await
        .with_context(|| format!("Failed to get entity operation: {filter:?}"))?
        .map(EntityHistoryEntry::try_new)
        .transpose()
}

#[instrument(skip(db))]
pub async fn insert_history_entry<T: ConnectionTrait>(
    db: &T,
    entry: EntityHistoryEntry,
) -> Result<()> {
    let entry = golem_base_entity_history::ActiveModel {
        entity_key: Set(entry.entity_key.as_slice().into()),
        block_number: Set(entry.block_number.try_into()?),
        block_hash: Set(entry.block_hash.as_slice().into()),
        transaction_hash: Set(entry.transaction_hash.as_slice().into()),
        tx_index: Set(entry.tx_index.try_into()?),
        op_index: Set(entry.op_index.try_into()?),
        block_timestamp: Set(entry.block_timestamp.naive_utc()),
        owner: Set(entry.owner.map(|v| v.as_slice().into())),
        sender: Set(entry.sender.as_slice().into()),
        operation: Set(entry.operation.into()),
        data: Set(entry.data.map(|v| v.into())),
        prev_data: Set(entry.prev_data.map(|v| v.into())),
        btl: Set(entry.btl.map(|v| v.into())),
        status: Set(entry.status.into()),
        prev_status: Set(entry.prev_status.map(|v| v.into())),
        expires_at_block_number: Set(entry
            .expires_at_block_number
            .map(|v| v.try_into())
            .transpose()?),
        prev_expires_at_block_number: Set(entry
            .prev_expires_at_block_number
            .map(|v| v.try_into())
            .transpose()?),
    };
    golem_base_entity_history::Entity::insert(entry)
        .exec(db)
        .await
        .context("Failed to insert history entry")?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn delete_history<T: ConnectionTrait>(db: &T, entity: EntityKey) -> Result<()> {
    let entity: Vec<u8> = entity.as_slice().into();
    golem_base_entity_history::Entity::delete_many()
        .filter(golem_base_entity_history::Column::EntityKey.eq(entity))
        .exec(db)
        .await
        .context("Failed to delete entity history")?;
    Ok(())
}
#[instrument(skip(db))]
pub async fn refresh_entity_based_on_history<T: ConnectionTrait>(
    db: &T,
    key: EntityKey,
) -> Result<()> {
    let latest_entry = get_latest_entity_history_entry(db, key, None).await?;

    if let Some(latest_entry) = latest_entry {
        let create_op = super::operations::find_create_operation(db, key).await?;

        let entity = golem_base_entities::ActiveModel {
            key: Set(key.as_slice().into()),
            data: Set(latest_entry.data.map(Into::into)),
            status: Set(latest_entry.status.into()),
            owner: Set(latest_entry.owner.map(|v| v.as_slice().into())),
            created_at_tx_hash: Set(create_op.map(|v| v.metadata.tx_hash.as_slice().into())),
            last_updated_at_tx_hash: Set(latest_entry.transaction_hash.as_slice().into()),
            expires_at_block_number: Set(latest_entry
                .expires_at_block_number
                .map(TryInto::try_into)
                .transpose()?),
            inserted_at: NotSet,
            updated_at: Set(Utc::now().naive_utc()),
        };
        golem_base_entities::Entity::insert(entity)
            .on_conflict(
                OnConflict::column(golem_base_entities::Column::Key)
                    .update_columns(golem_base_entities::Column::iter())
                    .to_owned(),
            )
            .exec(db)
            .await
            .context("Failed to replace entity")?;
    } else {
        drop_entity(db, key).await?;
    }

    Ok(())
}

#[instrument(skip(db))]
pub async fn get_entity_size_data_histogram<T: ConnectionTrait>(
    db: &T,
) -> Result<Vec<EntityDataHistogram>> {
    entity_data_size_histogram::Entity::find()
        .order_by_asc(entity_data_size_histogram::Column::Bucket)
        .all(db)
        .await
        .context("Failed to get entity size data histogram")?
        .into_iter()
        .map(EntityDataHistogram::try_from)
        .collect::<Result<Vec<_>>>()
}
