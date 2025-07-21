use anyhow::Result;
use golem_base_indexer_entity::{
    golem_base_entities, sea_orm_active_enums::GolemBaseEntityStatusType,
};
use sea_orm::{
    prelude::*,
    sea_query::OnConflict,
    sqlx::types::chrono::Utc,
    ActiveValue::{NotSet, Set},
};
use tracing::instrument;

#[derive(Debug)]
pub struct GolemBaseEntityCreate {
    pub key: Vec<u8>,
    pub data: Vec<u8>,
    pub created_at_tx_hash: Vec<u8>,
    pub expires_at_block_number: i64,
}

#[derive(Debug)]
pub struct GolemBaseEntityUpdate {
    pub key: Vec<u8>,
    pub data: Vec<u8>,
    pub updated_at_tx_hash: Vec<u8>,
    pub expires_at_block_number: i64,
}

#[derive(Debug)]
pub struct GolemBaseEntityDelete {
    pub key: Vec<u8>,
    pub deleted_at_tx_hash: Vec<u8>,
    pub deleted_at_block_number: i64,
}

#[derive(Debug)]
pub struct GolemBaseEntityExtend {
    pub key: Vec<u8>,
    pub extended_at_tx_hash: Vec<u8>,
    pub expires_at_block_number: i64,
}

#[instrument(name = "repository::entities::insert_entity", skip(db), level = "info")]
pub async fn insert_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityCreate,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key),
        data: Set(Some(entity.data)),
        status: Set(GolemBaseEntityStatusType::Active),
        created_at_tx_hash: Set(Some(entity.created_at_tx_hash.clone())),
        expires_at_block_number: Set(entity.expires_at_block_number),
        last_updated_at_tx_hash: Set(entity.created_at_tx_hash),
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
        .inspect_err(|e| panic!("WTF"))?;
    Ok(())
}

#[instrument(name = "repository::entities::update_entity", skip(db), level = "info")]
pub async fn update_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityUpdate,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key),
        data: Set(Some(entity.data)),
        expires_at_block_number: Set(entity.expires_at_block_number),
        last_updated_at_tx_hash: Set(entity.updated_at_tx_hash),
        updated_at: Set(Utc::now().naive_utc()),
        status: Set(GolemBaseEntityStatusType::Active),
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
        .inspect_err(|e| panic!("WTF"))?;

    Ok(())
}

#[instrument(name = "repository::entities::delete_entity", skip(db), level = "info")]
pub async fn delete_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityDelete,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key),
        status: Set(GolemBaseEntityStatusType::Deleted),
        last_updated_at_tx_hash: Set(entity.deleted_at_tx_hash),
        updated_at: Set(Utc::now().naive_utc()),
        data: NotSet,
        expires_at_block_number: Set(entity.deleted_at_block_number),
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
        .inspect_err(|e| panic!("WTF"))?;
    Ok(())
}

#[instrument(name = "repository::entities::extend_entity", skip(db), level = "info")]
pub async fn extend_entity<T: ConnectionTrait>(
    db: &T,
    entity: GolemBaseEntityExtend,
) -> Result<()> {
    let model = golem_base_entities::ActiveModel {
        key: Set(entity.key),
        expires_at_block_number: Set(entity.expires_at_block_number),
        last_updated_at_tx_hash: Set(entity.extended_at_tx_hash),
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
        .inspect_err(|e| tracing::error!(?e, "WTF"))
        .inspect_err(|e| panic!("WTF"))?;
    Ok(())
}

#[instrument(name = "repository::entities::get_entity", skip(db), level = "info")]
pub async fn get_entity<T: ConnectionTrait>(
    db: &T,
    key: Vec<u8>,
) -> Result<Option<golem_base_entities::Model>> {
    Ok(golem_base_entities::Entity::find_by_id(key).one(db).await?)
}

#[instrument(name = "repository::entities::list_entities", skip(db), level = "info")]
pub async fn list_entities<T: ConnectionTrait>(db: &T) -> Result<Vec<golem_base_entities::Model>> {
    Ok(golem_base_entities::Entity::find().all(db).await?)
}
