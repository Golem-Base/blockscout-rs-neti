use anyhow::{Context, Result};
use golem_base_indexer_entity::{golem_base_numeric_annotations, golem_base_string_annotations};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
};
use tracing::instrument;

use crate::types::{EntityKey, NumericAnnotation, StringAnnotation};

impl TryFrom<StringAnnotation> for golem_base_string_annotations::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(value: StringAnnotation) -> Result<Self> {
        Ok(Self {
            entity_key: Set(value.entity_key.as_slice().into()),
            operation_tx_hash: Set(value.operation_tx_hash.as_slice().into()),
            operation_index: Set(value.operation_index.try_into()?),
            key: Set(value.key),
            value: Set(value.value),
            inserted_at: NotSet,
            active: NotSet,
        })
    }
}

impl TryFrom<NumericAnnotation> for golem_base_numeric_annotations::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(value: NumericAnnotation) -> Result<Self> {
        Ok(Self {
            entity_key: Set(value.entity_key.as_slice().into()),
            operation_tx_hash: Set(value.operation_tx_hash.as_slice().into()),
            operation_index: Set(value.operation_index.try_into()?),
            key: Set(value.key),
            value: Set(value.value.into()),
            inserted_at: NotSet,
            active: NotSet,
        })
    }
}

#[instrument(name = "repository::annotations::insert_string_annotation", skip(db))]
pub async fn insert_string_annotation<T: ConnectionTrait>(
    db: &T,
    annotation: StringAnnotation,
    active: bool,
) -> Result<()> {
    golem_base_string_annotations::ActiveModel {
        active: Set(active),
        ..annotation.clone().try_into()?
    }
    .insert(db)
    .await
    .with_context(|| {
        format!("Failed to insert string annotation {annotation:?} (active: {active})")
    })?;

    Ok(())
}

#[instrument(name = "repository::annotations::insert_numeric_annotation", skip(db))]
pub async fn insert_numeric_annotation<T: ConnectionTrait>(
    db: &T,
    annotation: NumericAnnotation,
    active: bool,
) -> Result<()> {
    golem_base_numeric_annotations::ActiveModel {
        active: Set(active),
        ..annotation.clone().try_into()?
    }
    .insert(db)
    .await
    .with_context(|| {
        format!("Failed to insert numeric annotation {annotation:?} (active: {active})")
    })?;

    Ok(())
}

#[instrument(name = "repository::annotations::deactivate_annotations", skip(db))]
pub async fn deactivate_annotations<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<()> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();

    let res = golem_base_string_annotations::Entity::update_many()
        .col_expr(
            golem_base_string_annotations::Column::Active,
            Expr::value(false),
        )
        .filter(golem_base_string_annotations::Column::EntityKey.eq(entity_key.clone()))
        .exec(db)
        .await;

    match res {
        Ok(_) => {}
        Err(DbErr::RecordNotUpdated) => {}
        Err(e) => return Err(e.into()),
    };

    let res = golem_base_numeric_annotations::Entity::update_many()
        .col_expr(
            golem_base_numeric_annotations::Column::Active,
            Expr::value(false),
        )
        .filter(golem_base_numeric_annotations::Column::EntityKey.eq(entity_key))
        .exec(db)
        .await;

    match res {
        Ok(_) => {}
        Err(DbErr::RecordNotUpdated) => {}
        Err(e) => return Err(e.into()),
    };

    Ok(())
}
