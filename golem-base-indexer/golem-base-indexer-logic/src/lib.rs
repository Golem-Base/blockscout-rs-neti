use alloy_primitives::{TxHash, B256, U256};
use alloy_rlp::Decodable;
use alloy_sol_types::SolValue;
use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use golem_base_indexer_entity::sea_orm_active_enums::GolemBaseOperationType;
use golem_base_sdk::entity::{
    Create, EncodableGolemBaseTransaction, Extend, GolemBaseDelete, Update,
};
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};
use serde::Deserialize;
use serde_with::serde_as;
use std::{sync::Arc, time};
use tokio::time::sleep;
use tracing::instrument;

use crate::{
    golem_base::entity_key,
    repository::{
        annotations::{
            GolemBaseAnnotationsDeactivate, GolemBaseNumericAnnotation, GolemBaseStringAnnotation,
        },
        entities::{
            GolemBaseEntityCreate, GolemBaseEntityDelete, GolemBaseEntityExtend,
            GolemBaseEntityUpdate,
        },
        logs::Log,
        operations::GolemBaseOperationCreate,
        transactions::Tx,
    },
};

mod golem_base;
pub mod repository;
mod well_known;

#[serde_as]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerSettings {
    pub concurrency: usize,

    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    pub polling_interval: time::Duration,

    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    pub restart_delay: time::Duration,
}

impl Default for IndexerSettings {
    fn default() -> Self {
        Self {
            concurrency: 10,
            restart_delay: time::Duration::from_secs(60),
            polling_interval: time::Duration::from_secs(1),
        }
    }
}

pub struct Indexer {
    db: Arc<DatabaseConnection>,
    settings: IndexerSettings,
}

// FIXME integration tests
// FIXME only process txs that didn't revert
// FIXME what about chain reorgs (use debug_setHead for testing)
// FIXME refactor whole logic to use some defined, sane `types` for inter-module and inter-crate calls
// FIXME cleanup logging
// FIXME separate Expired state
// FIXME test what happens when DB connection fails
// FIXME only process non-pending transactions
impl Indexer {
    pub fn new(db: Arc<DatabaseConnection>, settings: IndexerSettings) -> Self {
        Self { db, settings }
    }

    #[instrument(name = "indexer", skip_all, level = "info")]
    pub async fn start(self) -> Result<()> {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.tick().await {
                    tracing::error!(
                        ?e,
                        "Failed to index storage txs, exiting (will be restarted)..."
                    );
                    return;
                };
                sleep(self.settings.polling_interval).await;
            }
        });
        Ok(())
    }

    #[instrument(name = "indexer::tick", skip_all, level = "info")]
    pub async fn tick(&self) -> Result<()> {
        repository::transactions::stream_unprocessed_tx_hashes(&*self.db)
            .await?
            .map(Ok)
            .try_for_each_concurrent(self.settings.concurrency, |tx| async move {
                self.handle_tx(tx).await
            })
            .await
    }

    #[instrument(name = "indexer::handle_tx", skip(self), level = "info")]
    async fn handle_tx(&self, tx_hash: TxHash) -> Result<()> {
        let txn = self.db.begin().await?;

        let tx = repository::transactions::get_tx(&txn, tx_hash).await?;
        let tx = tx.ok_or(anyhow!("Somehow tx disappeared from the DB"))?;
        let storagetx = match EncodableGolemBaseTransaction::decode(&mut tx.input.as_slice()) {
            Ok(storagetx) => storagetx,
            Err(e) => {
                tracing::warn!(?tx_hash, ?e, "Storage tx with undecodable data");
                return Ok(());
            }
        };

        // following operations are a good candidate for optimization when needed
        // possible improvements include parallelization and batching
        let mut idx = 0;
        for create in storagetx.creates {
            self.handle_create(&txn, &tx, create, idx).await?;
            idx += 1;
        }
        for update in storagetx.updates {
            self.handle_update(&txn, &tx, update, idx).await?;
            idx += 1;
        }
        for delete in storagetx.deletes {
            self.handle_delete(&txn, &tx, delete, idx).await?;
            idx += 1;
        }
        for extend in storagetx.extensions {
            self.handle_extend(&txn, &tx, extend, idx).await?;
            idx += 1;
        }

        // FIXME what if blockscout lags with populating logs?
        let logs = repository::logs::get_tx_logs(
            &txn,
            tx_hash.as_slice().into(),
            well_known::GOLEM_BASE_STORAGE_ENTITY_BTL_EXTENDED
                .as_slice()
                .into(),
        )
        .await?;

        for extend_log in logs {
            self.handle_extend_log(&txn, &tx, extend_log).await?;
        }

        txn.commit().await?;

        Ok(())
    }

    #[instrument(name = "indexer::handle_create", skip(self, tx), fields(tx_hash = ?tx.hash), level = "info")]
    async fn handle_create(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        create: Create,
        idx: i64,
    ) -> Result<()> {
        let key = entity_key(tx.hash, &create.data, idx).to_vec();
        let data: Vec<u8> = create.data.into();

        let latest_update = self
            .is_latest_update(txn, key.clone(), tx.hash, idx)
            .await?;

        repository::operations::insert_operation(
            txn,
            GolemBaseOperationCreate {
                entity_key: key.clone(),
                sender: tx.from_address_hash.as_slice().into(),
                operation: GolemBaseOperationType::Create,
                data: Some(data.clone()),
                btl: Some(create.btl.into()),
                transaction_hash: tx.hash.as_slice().into(),
                block_hash: tx.block_hash.as_slice().into(),
                index: idx,
            },
        )
        .await?;

        // will only update `created_at_tx_hash` if already exists
        repository::entities::insert_entity(
            txn,
            GolemBaseEntityCreate {
                key: key.clone(),
                data,
                created_at_tx_hash: tx.hash.as_slice().into(),
                expires_at_block_number: tx
                    .block_number
                    .saturating_add(create.btl)
                    .try_into()
                    .unwrap_or(i64::MAX),
            },
        )
        .await?;

        for annotation in create.string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                GolemBaseStringAnnotation {
                    entity_key: key.clone(),
                    operation_tx_hash: tx.hash.as_slice().into(),
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                    active: latest_update,
                },
            )
            .await?;
        }

        for annotation in create.numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                GolemBaseNumericAnnotation {
                    entity_key: key.clone(),
                    operation_tx_hash: tx.hash.as_slice().into(),
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                    active: latest_update,
                },
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(name = "indexer::handle_update", skip(self, tx), fields(tx_hash = ?tx.hash), level = "info")]
    async fn handle_update(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        update: Update,
        idx: i64,
    ) -> Result<()> {
        let data: Vec<u8> = update.data.into();
        let latest_update = self
            .is_latest_update(txn, update.entity_key.as_slice().into(), tx.hash, idx)
            .await?;

        repository::operations::insert_operation(
            txn,
            GolemBaseOperationCreate {
                entity_key: update.entity_key.as_slice().into(),
                sender: tx.from_address_hash.as_slice().into(),
                operation: GolemBaseOperationType::Update,
                data: Some(data.clone()),
                btl: Some(update.btl.into()),
                transaction_hash: tx.hash.as_slice().into(),
                block_hash: tx.block_hash.as_slice().into(),
                index: idx,
            },
        )
        .await?;

        if latest_update {
            repository::entities::update_entity(
                txn,
                GolemBaseEntityUpdate {
                    key: update.entity_key.as_slice().into(),
                    data,
                    updated_at_tx_hash: tx.hash.as_slice().into(),
                    expires_at_block_number: tx
                        .block_number
                        .saturating_add(update.btl)
                        .try_into()
                        .unwrap_or(i64::MAX),
                },
            )
            .await?;

            repository::annotations::deactivate_annotations(
                txn,
                GolemBaseAnnotationsDeactivate {
                    entity_key: update.entity_key.as_slice().into(),
                },
            )
            .await?;
        }

        for annotation in update.string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                GolemBaseStringAnnotation {
                    entity_key: update.entity_key.as_slice().into(),
                    operation_tx_hash: tx.hash.as_slice().into(),
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                    active: latest_update,
                },
            )
            .await?;
        }

        for annotation in update.numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                GolemBaseNumericAnnotation {
                    entity_key: update.entity_key.as_slice().into(),
                    operation_tx_hash: tx.hash.as_slice().into(),
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                    active: latest_update,
                },
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(name = "indexer::handle_delete", skip(self, tx), fields(tx_hash = ?tx.hash), level = "info")]
    async fn handle_delete(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        delete: GolemBaseDelete,
        idx: i64,
    ) -> Result<()> {
        repository::operations::insert_operation(
            txn,
            GolemBaseOperationCreate {
                entity_key: delete.as_slice().into(),
                sender: tx.from_address_hash.as_slice().into(),
                operation: GolemBaseOperationType::Delete,
                data: None,
                btl: None,
                transaction_hash: tx.hash.as_slice().into(),
                block_hash: tx.block_hash.as_slice().into(),
                index: idx,
            },
        )
        .await?;

        repository::entities::delete_entity(
            txn,
            GolemBaseEntityDelete {
                key: delete.as_slice().into(),
                deleted_at_tx_hash: tx.hash.as_slice().into(),
                deleted_at_block_number: tx.block_number.try_into()?,
            },
        )
        .await?;

        repository::annotations::deactivate_annotations(
            txn,
            GolemBaseAnnotationsDeactivate {
                entity_key: delete.as_slice().into(),
            },
        )
        .await?;
        Ok(())
    }

    #[instrument(name = "indexer::handle_extend", skip(self, tx), fields(tx_hash = ?tx.hash), level = "info")]
    async fn handle_extend(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        extend: Extend,
        idx: i64,
    ) -> Result<()> {
        repository::operations::insert_operation(
            txn,
            GolemBaseOperationCreate {
                entity_key: extend.entity_key.as_slice().into(),
                sender: tx.from_address_hash.as_slice().into(),
                operation: GolemBaseOperationType::Extend,
                data: None,
                btl: Some(extend.number_of_blocks.into()),
                transaction_hash: tx.hash.as_slice().into(),
                block_hash: tx.block_hash.as_slice().into(),
                index: idx,
            },
        )
        .await?;

        // updating entity expiration is handled based on events

        Ok(())
    }

    async fn is_latest_update(
        &self,
        txn: &DatabaseTransaction,
        entity_key: Vec<u8>,
        tx_hash: B256,
        operation_index: i64,
    ) -> Result<bool> {
        let latest_stored_update =
            repository::operations::get_latest_update(txn, entity_key).await?;
        let latest_stored_update = if let Some(update) = latest_stored_update {
            update
        } else {
            return Ok(true);
        };

        let candidate_update = repository::transactions::get_tx(txn, tx_hash).await?;
        let candidate_update = if let Some(tx) = candidate_update {
            tx
        } else {
            tracing::warn!(tx=?tx_hash, "Transaction disappeared from the database");
            return Ok(true);
        };

        let candidate_update = (
            candidate_update.block_number.try_into().unwrap_or(i64::MAX),
            candidate_update.index,
            operation_index,
        );

        Ok(candidate_update > latest_stored_update)
    }

    #[instrument(name = "indexer::handle_extend_log", skip(self, tx, log), fields(tx_hash = ?tx.hash), level = "info")]
    async fn handle_extend_log(&self, txn: &DatabaseTransaction, tx: &Tx, log: Log) -> Result<()> {
        // extends are handled after updates, so if it's in the same tx, we need to process it
        let entity_key = if let Some(k) = log.second_topic {
            k
        } else {
            tracing::warn!("Extend Log event with no second topic?");
            return Ok(());
        };

        let latest_update = self
            .is_latest_update(txn, entity_key.clone(), tx.hash, i64::MAX)
            .await?;

        if latest_update {
            type EventArgs = (U256, U256);
            let (_, expires_at_block_number) = if let Ok(res) =
                EventArgs::abi_decode(log.data.as_slice())
            {
                res
            } else {
                tracing::warn!(data=?log.data, "Invalid GolemBaseStorageEntityBTLExtended event data encountered");
                return Ok(());
            };
            repository::entities::extend_entity(
                txn,
                GolemBaseEntityExtend {
                    key: entity_key,
                    extended_at_tx_hash: tx.hash.as_slice().into(),
                    expires_at_block_number: expires_at_block_number.try_into().unwrap_or(i64::MAX),
                },
            )
            .await?;
        }

        Ok(())
    }
}
