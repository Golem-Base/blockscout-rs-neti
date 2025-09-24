use anyhow::{Context, Result};
use golem_base_indexer_entity::golem_base_entity_locks;
use sea_orm::{prelude::*, ActiveValue::Set};
use tracing::instrument;

use crate::types::EntityKey;

#[must_use]
pub struct Guard(EntityKey);

#[instrument(skip(db))]
pub async fn clear<T: ConnectionTrait>(db: &T) -> Result<()> {
    golem_base_entity_locks::Entity::delete_many()
        .exec(db)
        .await
        .context("Failed to clear locks")?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn lock<T: ConnectionTrait>(db: &T, key: EntityKey) -> Result<Guard> {
    let model = golem_base_entity_locks::ActiveModel {
        key: Set(key.as_slice().into()),
    };

    golem_base_entity_locks::Entity::insert(model)
        .exec(db)
        .await
        .context("Failed to lock entity")?;

    Ok(Guard(key))
}

impl Guard {
    #[instrument(skip(self, db), fields(key=?self.0))]
    pub async fn unlock<T: ConnectionTrait>(self, db: &T) -> Result<()> {
        let key: Vec<u8> = self.0.as_slice().into();
        golem_base_entity_locks::Entity::delete_by_id(key)
            .exec(db)
            .await
            .context("Failed to unlock entity")?;
        Ok(())
    }
}
