use anyhow::{Context, Result};
use golem_base_indexer_entity::{golem_base_numeric_annotations, golem_base_string_annotations};
use sea_orm::{
    prelude::*,
    ActiveValue::{NotSet, Set},
    DbBackend, FromQueryResult, Statement,
};
use tracing::instrument;

use crate::{
    repository::sql,
    types::{
        EntityKey, FullNumericAttribute, FullStringAttribute, NumericAttribute,
        NumericAttributeWithRelations, StringAttribute, StringAttributeWithRelations, TxHash,
    },
};

#[derive(FromQueryResult)]
struct DbStringAttributeWithRelations {
    pub key: String,
    pub value: String,
    pub related_entities: i64,
}

#[derive(FromQueryResult)]
struct DbNumericAttributeWithRelations {
    pub key: String,
    pub value: Decimal,
    pub related_entities: i64,
}

impl TryFrom<DbStringAttributeWithRelations> for StringAttributeWithRelations {
    type Error = anyhow::Error;
    fn try_from(value: DbStringAttributeWithRelations) -> Result<Self> {
        Ok(Self {
            attribute: StringAttribute {
                key: value.key,
                value: value.value,
            },
            related_entities: value.related_entities.try_into()?,
        })
    }
}

impl TryFrom<DbNumericAttributeWithRelations> for NumericAttributeWithRelations {
    type Error = anyhow::Error;

    fn try_from(value: DbNumericAttributeWithRelations) -> Result<Self> {
        Ok(Self {
            attribute: NumericAttribute {
                key: value.key,
                value: value.value.try_into()?,
            },
            related_entities: value.related_entities.try_into()?,
        })
    }
}

impl From<golem_base_string_annotations::Model> for StringAttribute {
    fn from(value: golem_base_string_annotations::Model) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl TryFrom<golem_base_numeric_annotations::Model> for NumericAttribute {
    type Error = anyhow::Error;

    fn try_from(value: golem_base_numeric_annotations::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            key: value.key,
            value: value.value.try_into()?,
        })
    }
}

impl TryFrom<FullStringAttribute> for golem_base_string_annotations::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(value: FullStringAttribute) -> Result<Self> {
        Ok(Self {
            id: NotSet,
            entity_key: Set(value.entity_key.as_slice().into()),
            operation_tx_hash: Set(value.operation_tx_hash.as_slice().into()),
            operation_index: Set(value.operation_index.try_into()?),
            key: Set(value.attribute.key),
            value: Set(value.attribute.value),
            inserted_at: NotSet,
            active: NotSet,
        })
    }
}

impl TryFrom<FullNumericAttribute> for golem_base_numeric_annotations::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(value: FullNumericAttribute) -> Result<Self> {
        Ok(Self {
            id: NotSet,
            entity_key: Set(value.entity_key.as_slice().into()),
            operation_tx_hash: Set(value.operation_tx_hash.as_slice().into()),
            operation_index: Set(value.operation_index.try_into()?),
            key: Set(value.attribute.key),
            value: Set(value.attribute.value.into()),
            inserted_at: NotSet,
            active: NotSet,
        })
    }
}

#[instrument(skip(db))]
pub async fn insert_string_attribute<T: ConnectionTrait>(
    db: &T,
    attribute: FullStringAttribute,
    active: bool,
) -> Result<()> {
    golem_base_string_annotations::ActiveModel {
        active: Set(active),
        ..attribute.clone().try_into()?
    }
    .insert(db)
    .await
    .with_context(|| {
        format!("Failed to insert string attribute {attribute:?} (active: {active})")
    })?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn insert_numeric_attribute<T: ConnectionTrait>(
    db: &T,
    attribute: FullNumericAttribute,
    active: bool,
) -> Result<()> {
    golem_base_numeric_annotations::ActiveModel {
        active: Set(active),
        ..attribute.clone().try_into()?
    }
    .insert(db)
    .await
    .with_context(|| {
        format!("Failed to insert numeric attribute {attribute:?} (active: {active})")
    })?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn deactivate_attributes<T: ConnectionTrait>(
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
            return Err(e).context("Deactivating string attributes");
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
            return Err(e).context("Deactivating numeric attributes");
        }
    };

    Ok(())
}

#[instrument(skip(db))]
pub async fn find_active_string_attributes<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Vec<StringAttributeWithRelations>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    DbStringAttributeWithRelations::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::GET_STRING_ANNOTATIONS_WITH_RELATIONS,
        [entity_key.into()],
    ))
    .all(db)
    .await
    .context("Finding active string attributes")?
    .into_iter()
    .map(TryInto::<StringAttributeWithRelations>::try_into)
    .collect::<Result<Vec<_>>>()
}

#[instrument(skip(db))]
pub async fn find_active_numeric_attributes<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
) -> Result<Vec<NumericAttributeWithRelations>> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    DbNumericAttributeWithRelations::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::GET_NUMERIC_ANNOTATIONS_WITH_RELATIONS,
        [entity_key.into()],
    ))
    .all(db)
    .await
    .context("Finding active numeric attributes")?
    .into_iter()
    .map(TryInto::<NumericAttributeWithRelations>::try_into)
    .collect::<Result<Vec<_>>>()
}

#[instrument(skip(db))]
pub async fn activate_attributes<T: ConnectionTrait>(
    db: &T,
    entity_key: EntityKey,
    index: (TxHash, u64),
) -> Result<()> {
    let entity_key: Vec<u8> = entity_key.as_slice().into();
    let tx_hash: Vec<u8> = index.0.as_slice().into();

    let res = golem_base_string_annotations::Entity::update_many()
        .col_expr(
            golem_base_string_annotations::Column::Active,
            Expr::value(true),
        )
        .filter(golem_base_string_annotations::Column::EntityKey.eq(entity_key.clone()))
        .filter(golem_base_string_annotations::Column::OperationTxHash.eq(tx_hash.clone()))
        .filter(golem_base_string_annotations::Column::OperationIndex.eq(index.1))
        .exec(db)
        .await;

    match res {
        Ok(_) => {}
        Err(DbErr::RecordNotUpdated) => {}
        Err(e) => {
            return Err(e).context("Deactivating string attributes");
        }
    };

    let res = golem_base_numeric_annotations::Entity::update_many()
        .col_expr(
            golem_base_numeric_annotations::Column::Active,
            Expr::value(true),
        )
        .filter(golem_base_numeric_annotations::Column::EntityKey.eq(entity_key))
        .filter(golem_base_numeric_annotations::Column::OperationTxHash.eq(tx_hash))
        .filter(golem_base_numeric_annotations::Column::OperationIndex.eq(index.1))
        .exec(db)
        .await;

    match res {
        Ok(_) => {}
        Err(DbErr::RecordNotUpdated) => {}
        Err(e) => {
            return Err(e).context("Deactivating numeric attributes");
        }
    };

    Ok(())
}
