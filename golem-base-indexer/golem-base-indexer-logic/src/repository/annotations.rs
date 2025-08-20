use anyhow::{Context, Result};
use golem_base_indexer_entity::{golem_base_numeric_annotations, golem_base_string_annotations};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
};
use tracing::instrument;

use crate::types::{
    EntityKey, FullNumericAnnotation, FullStringAnnotation, NumericAnnotation, StringAnnotation,
};

impl From<golem_base_string_annotations::Model> for StringAnnotation {
    fn from(value: golem_base_string_annotations::Model) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl TryFrom<golem_base_numeric_annotations::Model> for NumericAnnotation {
    type Error = anyhow::Error;

    fn try_from(value: golem_base_numeric_annotations::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            key: value.key,
            value: value.value.try_into()?,
        })
    }
}

impl TryFrom<FullStringAnnotation> for golem_base_string_annotations::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(value: FullStringAnnotation) -> Result<Self> {
        Ok(Self {
            id: NotSet,
            entity_key: Set(value.entity_key.as_slice().into()),
            operation_tx_hash: Set(value.operation_tx_hash.as_slice().into()),
            operation_index: Set(value.operation_index.try_into()?),
            key: Set(value.annotation.key),
            value: Set(value.annotation.value),
            inserted_at: NotSet,
            active: NotSet,
        })
    }
}

impl TryFrom<FullNumericAnnotation> for golem_base_numeric_annotations::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(value: FullNumericAnnotation) -> Result<Self> {
        Ok(Self {
            id: NotSet,
            entity_key: Set(value.entity_key.as_slice().into()),
            operation_tx_hash: Set(value.operation_tx_hash.as_slice().into()),
            operation_index: Set(value.operation_index.try_into()?),
            key: Set(value.annotation.key),
            value: Set(value.annotation.value.into()),
            inserted_at: NotSet,
            active: NotSet,
        })
    }
}

#[instrument(skip(db))]
pub async fn insert_string_annotation<T: ConnectionTrait>(
    db: &T,
    annotation: FullStringAnnotation,
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

#[instrument(skip(db))]
pub async fn insert_numeric_annotation<T: ConnectionTrait>(
    db: &T,
    annotation: FullNumericAnnotation,
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

#[instrument(skip(db))]
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
        Err(e) => {
            return Err(e).context("Deactivating string annotations");
        }
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
        Err(e) => {
            return Err(e).context("Deactivating numeric annotations");
        }
    };

    Ok(())
}

#[instrument(skip(db))]
pub async fn find_active_string_annotations<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Vec<StringAnnotation>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    Ok(golem_base_string_annotations::Entity::find()
        .filter(golem_base_string_annotations::Column::EntityKey.eq(entity_key))
        .filter(golem_base_string_annotations::Column::Active.eq(true))
        .all(db)
        .await
        .context("Finding active string annotations")?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[instrument(skip(db))]
pub async fn find_active_numeric_annotations<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Vec<NumericAnnotation>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    golem_base_numeric_annotations::Entity::find()
        .filter(golem_base_numeric_annotations::Column::EntityKey.eq(entity_key))
        .filter(golem_base_numeric_annotations::Column::Active.eq(true))
        .all(db)
        .await
        .context("Finding active numeric annotations")?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
}
