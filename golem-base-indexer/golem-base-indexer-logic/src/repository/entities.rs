use anyhow::{anyhow, Context, Result};
use chrono::Duration;
use golem_base_indexer_entity::{
    golem_base_entities, sea_orm_active_enums::GolemBaseEntityStatusType,
};
use sea_orm::{
    prelude::*,
    sea_query::OnConflict,
    sqlx::types::chrono::Utc,
    ActiveValue::{NotSet, Set},
    QueryOrder,
};
use tracing::instrument;

use crate::types::{
    Address, BlockNumber, Bytes, Entity, EntityKey, EntityStatus, FullEntity, TxHash,
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

#[instrument(name = "repository::entities::insert_entity", skip(db))]
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

#[instrument(name = "repository::entities::update_entity", skip(db))]
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

#[instrument(name = "repository::entities::delete_entity", skip(db))]
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
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .with_context(|| format!("Failed to delete entity: {entity:?}"))?;
    Ok(())
}

#[instrument(name = "repository::entities::extend_entity", skip(db))]
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

#[instrument(name = "repository::entities::get_entity", skip(db))]
pub async fn get_entity<T: ConnectionTrait>(db: &T, key: EntityKey) -> Result<Option<Entity>> {
    let key: Vec<u8> = key.as_slice().into();
    golem_base_entities::Entity::find_by_id(key.clone())
        .one(db)
        .await
        .with_context(|| format!("Failed to get entity: {key:?}"))?
        .map(|v| v.try_into())
        .transpose()
}

#[instrument(name = "repository::entities::list_entities", skip(db))]
pub async fn list_entities<T: ConnectionTrait>(db: &T) -> Result<Vec<Entity>> {
    golem_base_entities::Entity::find()
        .order_by_asc(golem_base_entities::Column::Key)
        .all(db)
        .await
        .context("Failed to list entities")?
        .into_iter()
        .map(|v| v.try_into())
        .collect()
}

#[instrument(name = "repository::entities::get_full_entity", skip(db))]
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
    let expires_at_timestamp = current_block.timestamp.and_utc()
        + Duration::seconds(
            (entity.expires_at_block_number - current_block.number) * secs_per_block,
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
        created_at_block_number: create_block
            .as_ref()
            .map(|v| v.number.try_into())
            .transpose()?,
        created_at_timestamp: create_block.as_ref().map(|v| v.timestamp.and_utc()),
        last_updated_at_tx_hash: entity.last_updated_at_tx_hash.as_slice().try_into()?,
        expires_at_block_number: entity.expires_at_block_number.try_into()?,
        expires_at_timestamp,
        owner: entity.owner.as_slice().try_into()?,
        gas_used: Default::default(), // FIXME when we have gas per operation
        fees_paid: Default::default(), // FIXME when we have gas per operation
    }))
}
