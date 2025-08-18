use anyhow::{anyhow, Context, Result};
use chrono::Duration;
use golem_base_indexer_entity::{
    golem_base_entities, golem_base_numeric_annotations, golem_base_string_annotations,
    sea_orm_active_enums::GolemBaseEntityStatusType,
};
use sea_orm::{
    prelude::*,
    sea_query::OnConflict,
    sqlx::types::chrono::Utc,
    ActiveValue::{NotSet, Set},
    FromQueryResult, Iterable, QueryOrder, Statement,
};
use tracing::instrument;

use crate::{
    model::entity_history,
    pagination::paginate_try_from,
    repository::sql,
    types::{
        Address, BlockHash, BlockNumber, Bytes, EntitiesFilter, Entity, EntityHistoryFilter,
        EntityKey, EntityStatus, FullEntity, ListEntitiesFilter, OperationData, OperationFilter,
        PaginationMetadata, Timestamp, TxHash,
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
            owner: value.owner.as_slice().try_into()?,
            created_at_tx_hash: value
                .created_at_tx_hash
                .map(|v| v.as_slice().try_into())
                .transpose()?,
            last_updated_at_tx_hash: value.last_updated_at_tx_hash.as_slice().try_into()?,
            expires_at_block_number: value.expires_at_block_number.try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EntityHistoryEntry {
    pub entity_key: EntityKey,
    pub block_number: BlockNumber,
    pub block_hash: BlockHash,
    pub transaction_hash: TxHash,
    pub tx_index: u64,
    pub op_index: u64,
    pub block_timestamp: Timestamp,
    pub sender: Address,
    pub data: Option<Bytes>,
    pub prev_data: Option<Bytes>,
    pub operation: OperationData,
    pub status: EntityStatus,
    pub prev_status: Option<EntityStatus>,
    pub expires_at_block_number: BlockNumber,
    pub prev_expires_at_block_number: Option<BlockNumber>,
    pub btl: Option<u64>,
}

impl TryFrom<entity_history::Model> for EntityHistoryEntry {
    type Error = anyhow::Error;

    fn try_from(value: entity_history::Model) -> Result<Self> {
        Ok(Self {
            entity_key: value.entity_key.as_slice().try_into()?,
            block_number: value.block_number.try_into()?,
            block_hash: value.block_hash.as_slice().try_into()?,
            transaction_hash: value.transaction_hash.as_slice().try_into()?,
            tx_index: value.tx_index.try_into()?,
            op_index: value.op_index.try_into()?,
            block_timestamp: value.block_timestamp.and_utc(),
            sender: value.sender.as_slice().try_into()?,
            operation: value.operation.into(),
            data: value.data.map(|v| v.into()),
            prev_data: value.prev_data.map(|v| v.into()),
            status: value.status.into(),
            prev_status: value.prev_status.map(|v| v.into()),
            expires_at_block_number: value.expires_at_block_number.try_into()?,
            prev_expires_at_block_number: value
                .prev_expires_at_block_number
                .map(|v| v.try_into())
                .transpose()?,
            btl: value.btl.map(|v| v.try_into()).transpose()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn insert_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityCreate,
) -> Result<()> {
    let created_at: Vec<u8> = entity.created_at.as_slice().into();
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key.as_slice().into()),
        data: Set(Some(entity.data.clone().into())),
        status: Set(GolemBaseEntityStatusType::Active),
        owner: Set(entity.sender.as_slice().into()),
        created_at_tx_hash: Set(Some(created_at.clone())),
        expires_at_block_number: Set(entity.expires_at.try_into()?),
        last_updated_at_tx_hash: Set(created_at),
        inserted_at: NotSet,
        updated_at: NotSet,
    };
    golem_base_entities::Entity::insert(model)
        .on_conflict(
            OnConflict::column(golem_base_entities::Column::Key)
                .update_columns([
                    golem_base_entities::Column::CreatedAtTxHash,
                    golem_base_entities::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .with_context(|| format!("Failed to insert entity: {entity:?}"))?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn update_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityUpdate,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key.as_slice().into()),
        data: Set(Some(entity.data.clone().into())),
        owner: Set(entity.sender.as_slice().into()),
        status: Set(GolemBaseEntityStatusType::Active),
        expires_at_block_number: Set(entity.expires_at.try_into()?),
        last_updated_at_tx_hash: Set(entity.updated_at.as_slice().into()),
        updated_at: Set(Utc::now().naive_utc()),
        created_at_tx_hash: NotSet,
        inserted_at: NotSet,
    };

    golem_base_entities::Entity::insert(model)
        .on_conflict(
            OnConflict::column(golem_base_entities::Column::Key)
                .update_columns([
                    golem_base_entities::Column::Data,
                    golem_base_entities::Column::ExpiresAtBlockNumber,
                    golem_base_entities::Column::LastUpdatedAtTxHash,
                    golem_base_entities::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .with_context(|| format!("Failed to update entity: {entity:?}"))?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn delete_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityDelete,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key.as_slice().into()),
        status: Set(entity.status.into()),
        last_updated_at_tx_hash: Set(entity.deleted_at_tx.as_slice().into()),
        owner: Set(entity.sender.as_slice().into()),
        updated_at: Set(Utc::now().naive_utc()),
        data: Set(None),
        expires_at_block_number: Set(entity.deleted_at_block.try_into()?),
        inserted_at: NotSet,
        created_at_tx_hash: NotSet,
    };

    golem_base_entities::Entity::insert(model)
        .on_conflict(
            OnConflict::column(golem_base_entities::Column::Key)
                .update_columns([
                    golem_base_entities::Column::Data,
                    golem_base_entities::Column::Status,
                    golem_base_entities::Column::LastUpdatedAtTxHash,
                    golem_base_entities::Column::UpdatedAt,
                    golem_base_entities::Column::ExpiresAtBlockNumber,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .with_context(|| format!("Failed to delete entity: {entity:?}"))?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn extend_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityExtend,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key.as_slice().into()),
        expires_at_block_number: Set(entity.expires_at.try_into()?),
        last_updated_at_tx_hash: Set(entity.extended_at.as_slice().into()),
        owner: Set(entity.sender.as_slice().into()),
        updated_at: Set(Utc::now().naive_utc()),
        status: Set(GolemBaseEntityStatusType::Active),
        data: NotSet,
        inserted_at: NotSet,
        created_at_tx_hash: NotSet,
    };

    golem_base_entities::Entity::insert(model)
        .on_conflict(
            OnConflict::column(golem_base_entities::Column::Key)
                .update_columns([
                    golem_base_entities::Column::ExpiresAtBlockNumber,
                    golem_base_entities::Column::LastUpdatedAtTxHash,
                    golem_base_entities::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .with_context(|| format!("Failed to extend entity: {entity:?}"))?;
    Ok(())
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

    let secs_per_block = 2;
    let current_block_number: i64 = current_block.number.try_into()?;
    let expires_at_timestamp = current_block.timestamp
        + Duration::seconds(
            (entity.expires_at_block_number - current_block_number) * secs_per_block,
        );

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
        last_updated_at_tx_hash: entity.last_updated_at_tx_hash.as_slice().try_into()?,
        expires_at_block_number: entity.expires_at_block_number.try_into()?,
        expires_at_timestamp,
        owner: entity.owner.as_slice().try_into()?,
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
pub async fn replace_entity<T: ConnectionTrait>(db: &T, entity: Entity) -> Result<()> {
    let entity = golem_base_entities::ActiveModel {
        key: Set(entity.key.as_slice().into()),
        data: Set(entity.data.map(|v| v.into())),
        status: Set(entity.status.into()),
        owner: Set(entity.owner.as_slice().into()),
        created_at_tx_hash: Set(entity.created_at_tx_hash.map(|v| v.as_slice().into())),
        expires_at_block_number: Set(entity.expires_at_block_number.try_into()?),
        last_updated_at_tx_hash: Set(entity.last_updated_at_tx_hash.as_slice().into()),
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
    Ok(())
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
pub async fn get_entity_history<T: ConnectionTrait>(
    db: &T,
    filter: EntityHistoryFilter,
) -> Result<(Vec<EntityHistoryEntry>, PaginationMetadata)> {
    let entity_key: Vec<u8> = filter.entity_key.as_slice().into();

    let paginator = entity_history::Entity::find()
        .filter(entity_history::Column::EntityKey.eq(entity_key))
        .order_by_asc(entity_history::Column::BlockNumber)
        .order_by_asc(entity_history::Column::TransactionHash)
        .order_by_asc(entity_history::Column::TxIndex)
        .order_by_asc(entity_history::Column::OpIndex)
        .paginate(db, filter.pagination.page_size);

    paginate_try_from(paginator, filter.pagination).await
}

#[instrument(skip(db))]
pub async fn get_entity_operation<T: ConnectionTrait>(
    db: &T,
    filter: OperationFilter,
) -> Result<EntityHistoryEntry> {
    let tx_hash: Vec<u8> = filter.tx_hash.as_slice().into();

    entity_history::Entity::find()
        .filter(entity_history::Column::TransactionHash.eq(tx_hash))
        .filter(entity_history::Column::OpIndex.eq(filter.op_index as i64))
        .one(db)
        .await
        .with_context(|| format!("Failed to get entity operation: {filter:?}"))?
        .ok_or_else(|| anyhow!("Entity operation not found"))
        .and_then(|v| v.try_into())
}
