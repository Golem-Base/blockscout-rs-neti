use anyhow::Result;
use golem_base_indexer_entity::{golem_base_numeric_annotations, golem_base_string_annotations};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
};
use tracing::instrument;

pub type GolemBaseStringAnnotation = GolemBaseGenericAnnotation<String>;
pub type GolemBaseNumericAnnotation = GolemBaseGenericAnnotation<u64>;

// FIXME something that will debug vec<u8> to hex?
#[derive(Debug)]
pub struct GolemBaseGenericAnnotation<T: std::fmt::Debug> {
    pub entity_key: Vec<u8>,
    pub operation_tx_hash: Vec<u8>,
    pub operation_index: i64,
    pub key: String,
    pub value: T,
    pub active: bool,
}

#[derive(Debug)]
pub struct GolemBaseAnnotationsDeactivate {
    pub entity_key: Vec<u8>,
}

#[instrument(
    name = "repository::annotations::insert_string_annotation",
    skip(db),
    level = "info"
)]
pub async fn insert_string_annotation<T: ConnectionTrait>(
    db: &T,
    annotation: GolemBaseStringAnnotation,
) -> Result<()> {
    golem_base_string_annotations::ActiveModel {
        entity_key: Set(annotation.entity_key),
        operation_tx_hash: Set(annotation.operation_tx_hash),
        operation_index: Set(annotation.operation_index),
        active: Set(annotation.active),
        key: Set(annotation.key),
        value: Set(annotation.value),
        inserted_at: NotSet,
    }
    .insert(db)
    .await?;
    Ok(())
}

#[instrument(
    name = "repository::annotations::insert_numeric_annotation",
    skip(db),
    level = "info"
)]
pub async fn insert_numeric_annotation<T: ConnectionTrait>(
    db: &T,
    annotation: GolemBaseNumericAnnotation,
) -> Result<()> {
    golem_base_numeric_annotations::ActiveModel {
        entity_key: Set(annotation.entity_key),
        operation_tx_hash: Set(annotation.operation_tx_hash),
        operation_index: Set(annotation.operation_index),
        active: Set(annotation.active),
        key: Set(annotation.key),
        value: Set(annotation.value.into()),
        inserted_at: NotSet,
    }
    .insert(db)
    .await?;
    Ok(())
}

#[instrument(
    name = "repository::annotations::deactivate_annotations",
    skip(db),
    level = "info"
)]
pub async fn deactivate_annotations<T: ConnectionTrait>(
    db: &T,
    deactivate: GolemBaseAnnotationsDeactivate,
) -> Result<()> {
    golem_base_string_annotations::Entity::update_many()
        .col_expr(
            golem_base_string_annotations::Column::Active,
            Expr::value(false),
        )
        .filter(golem_base_string_annotations::Column::EntityKey.eq(deactivate.entity_key.clone()))
        .exec(db)
        .await?;

    golem_base_numeric_annotations::Entity::update_many()
        .col_expr(
            golem_base_numeric_annotations::Column::Active,
            Expr::value(false),
        )
        .filter(golem_base_numeric_annotations::Column::EntityKey.eq(deactivate.entity_key))
        .exec(db)
        .await?;
    Ok(())
}
