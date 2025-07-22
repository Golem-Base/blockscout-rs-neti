use anyhow::{Context, Result};
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

use crate::types::{BlockNumber, Bytes, EntityKey, TxHash};

#[derive(Debug)]
pub struct GolemBaseEntityCreate {
    pub key: EntityKey,
    pub data: Bytes,
    pub created_at: TxHash,
    pub expires_at: BlockNumber,
}

#[derive(Debug)]
pub struct GolemBaseEntityUpdate {
    pub key: EntityKey,
    pub data: Bytes,
    pub updated_at: TxHash,
    pub expires_at: BlockNumber,
}

#[derive(Debug)]
pub struct GolemBaseEntityDelete {
    pub key: EntityKey,
    pub deleted_at_tx: TxHash,
    pub deleted_at_block: BlockNumber,
}

#[derive(Debug)]
pub struct GolemBaseEntityExtend {
    pub key: EntityKey,
    pub extended_at: TxHash,
    pub expires_at: BlockNumber,
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
        status: Set(GolemBaseEntityStatusType::Deleted),
        last_updated_at_tx_hash: Set(entity.deleted_at_tx.as_slice().into()),
        updated_at: Set(Utc::now().naive_utc()),
        data: NotSet,
        expires_at_block_number: Set(entity.deleted_at_block.try_into()?),
        inserted_at: NotSet,
        created_at_tx_hash: NotSet,
    };

    golem_base_entities::Entity::insert(model)
        .on_conflict(
            OnConflict::column(golem_base_entities::Column::Key)
                .update_columns([
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
        updated_at: Set(Utc::now().naive_utc()),
        status: Set(GolemBaseEntityStatusType::Active),
        data: NotSet,
        inserted_at: NotSet,
        created_at_tx_hash: NotSet,
    };
    // FIXME are all fields we skip optional???
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
pub async fn get_entity<T: ConnectionTrait>(
    db: &T,
    key: Vec<u8>,
) -> Result<Option<golem_base_entities::Model>> {
    golem_base_entities::Entity::find_by_id(key.clone())
        .one(db)
        .await
        .with_context(|| format!("Failed to get entity: {key:?}"))
}

#[instrument(name = "repository::entities::list_entities", skip(db))]
pub async fn list_entities<T: ConnectionTrait>(db: &T) -> Result<Vec<golem_base_entities::Model>> {
    golem_base_entities::Entity::find()
        .order_by_asc(golem_base_entities::Column::Key)
        .all(db)
        .await
        .context("Failed to list entities")
}
