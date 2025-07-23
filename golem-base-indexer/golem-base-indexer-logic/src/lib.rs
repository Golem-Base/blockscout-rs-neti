use alloy_primitives::U256;
use alloy_rlp::Decodable;
use alloy_sol_types::SolValue;
use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
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
    repository::entities::{
        GolemBaseEntityCreate, GolemBaseEntityDelete, GolemBaseEntityExtend, GolemBaseEntityUpdate,
    },
    types::{
        EntityKey, EntityStatus, Log, NumericAnnotation, Operation, OperationData,
        OperationMetadata, StringAnnotation, Tx, TxHash,
    },
};

mod golem_base;
pub mod repository;
pub mod types;
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
// FIXME what about chain reorgs (use debug_setHead for testing)
// FIXME we have enums from entity crate leaking
// FIXME cleanup logging
// FIXME separate Expired state
// FIXME test what happens when DB connection fails
// FIXME only process non-pending transactions
impl Indexer {
    pub fn new(db: Arc<DatabaseConnection>, settings: IndexerSettings) -> Self {
        Self { db, settings }
    }

    #[instrument(skip_all)]
    pub async fn run(self) -> Result<()> {
        loop {
            self.tick().await.inspect_err(|e| {
                tracing::error!(
                    ?e,
                    "Failed to index storage txs, exiting (will be restarted)..."
                )
            })?;
            sleep(self.settings.polling_interval).await;
        }
    }

    #[instrument(skip_all)]
    pub async fn tick(&self) -> Result<()> {
        repository::transactions::stream_unprocessed_tx_hashes(&*self.db)
            .await?
            .map(Ok)
            .try_for_each_concurrent(self.settings.concurrency, |tx| async move {
                self.handle_tx(tx).await
            })
            .await
    }

    #[instrument(skip(self))]
    async fn handle_tx(&self, tx_hash: TxHash) -> Result<()> {
        tracing::info!("Processing tx");

        let txn = self.db.begin().await?;

        let tx = repository::transactions::get_tx(&txn, tx_hash).await?;
        let tx = tx.ok_or(anyhow!("Somehow tx disappeared from the DB"))?;

        if tx.to_address_hash == well_known::GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS {
            let storagetx = match EncodableGolemBaseTransaction::decode(&mut &*tx.input) {
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
            for delete in storagetx.deletes {
                self.handle_delete(&txn, &tx, delete, idx).await?;
                idx += 1;
            }
            for update in storagetx.updates {
                self.handle_update(&txn, &tx, update, idx).await?;
                idx += 1;
            }
            for extend in storagetx.extensions {
                self.handle_extend(&txn, &tx, extend, idx).await?;
                idx += 1;
            }
        }

        if tx.to_address_hash == well_known::L1_BLOCK_CONTRACT_ADDRESS {
            // FIXME what if blockscout lags with populating logs?
            let logs = repository::logs::get_tx_logs(
                &txn,
                tx_hash,
                well_known::GOLEM_BASE_STORAGE_ENTITY_DELETED,
            )
            .await?;

            for delete_log in logs {
                self.handle_expire_log(&txn, &tx, delete_log).await?;
            }
        }

        // FIXME what if blockscout lags with populating logs?
        let logs = repository::logs::get_tx_logs(
            &txn,
            tx_hash,
            well_known::GOLEM_BASE_STORAGE_ENTITY_BTL_EXTENDED,
        )
        .await?;

        for extend_log in logs {
            self.handle_extend_log(&txn, &tx, extend_log).await?;
        }

        repository::transactions::finish_tx_processing(&txn, tx_hash).await?;
        txn.commit().await?;

        Ok(())
    }

    #[instrument(skip(self, tx), fields(tx_hash = ?tx.hash))]
    async fn handle_create(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        create: Create,
        idx: u64,
    ) -> Result<()> {
        let key = entity_key(tx.hash, create.data.clone(), idx);
        tracing::info!("Processing Create operation for entity 0x{key:x}");

        let latest_update = self.is_latest_update(txn, key, tx.hash, idx).await?;

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: key,
                    sender: tx.from_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: idx,
                },
                operation: OperationData::create(create.data.clone(), create.btl),
            },
        )
        .await?;

        repository::entities::insert_entity(
            txn,
            GolemBaseEntityCreate {
                key,
                data: create.data,
                created_at: tx.hash,
                expires_at: tx.block_number.saturating_add(create.btl),
            },
        )
        .await?;

        for annotation in create.string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                StringAnnotation {
                    entity_key: key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                },
                latest_update,
            )
            .await?;
        }

        for annotation in create.numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                NumericAnnotation {
                    entity_key: key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                },
                latest_update,
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip(self, tx), fields(tx_hash = ?tx.hash))]
    async fn handle_update(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        update: Update,
        idx: u64,
    ) -> Result<()> {
        tracing::info!(
            "Processing Update operation for entity 0x{:x}",
            update.entity_key
        );

        let latest_update = self
            .is_latest_update(txn, update.entity_key, tx.hash, idx)
            .await?;

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: update.entity_key,
                    sender: tx.from_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: idx,
                },
                operation: OperationData::update(update.data.clone(), update.btl),
            },
        )
        .await?;

        if latest_update {
            repository::entities::update_entity(
                txn,
                GolemBaseEntityUpdate {
                    key: update.entity_key,
                    data: update.data,
                    updated_at: tx.hash,
                    expires_at: tx.block_number.saturating_add(update.btl),
                },
            )
            .await?;

            repository::annotations::deactivate_annotations(txn, update.entity_key).await?;
        }

        for annotation in update.string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                StringAnnotation {
                    entity_key: update.entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                },
                latest_update,
            )
            .await?;
        }

        for annotation in update.numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                NumericAnnotation {
                    entity_key: update.entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    key: annotation.key,
                    value: annotation.value,
                },
                latest_update,
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip(self, tx), fields(tx_hash = ?tx.hash))]
    async fn handle_delete(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        delete: GolemBaseDelete,
        idx: u64,
    ) -> Result<()> {
        tracing::info!("Processing Delete operation for entity 0x{delete:x}");

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: delete,
                    sender: tx.from_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: idx,
                },
                operation: OperationData::delete(),
            },
        )
        .await?;

        repository::entities::delete_entity(
            txn,
            GolemBaseEntityDelete {
                key: delete,
                status: EntityStatus::Deleted,
                deleted_at_tx: tx.hash,
                deleted_at_block: tx.block_number,
            },
        )
        .await?;

        repository::annotations::deactivate_annotations(txn, delete).await?;
        Ok(())
    }

    #[instrument(skip(self, tx), fields(tx_hash = ?tx.hash))]
    async fn handle_extend(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        extend: Extend,
        idx: u64,
    ) -> Result<()> {
        tracing::info!(
            "Processing Extend operation for entity 0x{:x}",
            extend.entity_key
        );
        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: extend.entity_key,
                    sender: tx.from_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: idx,
                },
                operation: OperationData::extend(extend.number_of_blocks),
            },
        )
        .await?;

        // updating entity expiration is handled based on events

        Ok(())
    }

    async fn is_latest_update(
        &self,
        txn: &DatabaseTransaction,
        entity_key: EntityKey,
        tx_hash: TxHash,
        operation_index: u64,
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
            candidate_update.block_number,
            candidate_update.index,
            operation_index,
        );

        Ok(candidate_update > latest_stored_update)
    }

    #[instrument(skip(self, tx, log), fields(tx_hash = ?tx.hash))]
    async fn handle_extend_log(&self, txn: &DatabaseTransaction, tx: &Tx, log: Log) -> Result<()> {
        // extends are handled after updates, so if it's in the same tx, we need to process it
        let entity_key = if let Some(k) = log.second_topic {
            k
        } else {
            tracing::warn!("Extend Log event with no second topic?");
            return Ok(());
        };
        tracing::info!("Processing extend log for entity 0x{entity_key:x}");

        let latest_update = self
            .is_latest_update(txn, entity_key, tx.hash, u64::MAX)
            .await?;

        if latest_update {
            type EventArgs = (U256, U256);
            let (_, expires_at_block_number) = if let Ok(res) = EventArgs::abi_decode(&log.data) {
                res
            } else {
                tracing::warn!(data=?log.data, "Invalid GolemBaseStorageEntityBTLExtended event data encountered");
                return Ok(());
            };
            repository::entities::extend_entity(
                txn,
                GolemBaseEntityExtend {
                    key: entity_key,
                    extended_at: tx.hash,
                    expires_at: expires_at_block_number.try_into().unwrap_or(u64::MAX),
                },
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip(self, tx, log), fields(tx_hash = ?tx.hash))]
    async fn handle_expire_log(&self, txn: &DatabaseTransaction, tx: &Tx, log: Log) -> Result<()> {
        let entity_key = if let Some(k) = log.second_topic {
            k
        } else {
            tracing::warn!("Delete Log event with no second topic?");
            return Ok(());
        };
        tracing::info!("Processing delete log for entity 0x{entity_key:x}");

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key,
                    sender: tx.from_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: log.index, // FIXME this should be an operation index, not log index,
                                      // but we don't really have an operation in this case...
                },
                operation: OperationData::delete(),
            },
        )
        .await?;

        repository::entities::delete_entity(
            txn,
            GolemBaseEntityDelete {
                key: entity_key,
                status: EntityStatus::Expired,
                deleted_at_tx: tx.hash,
                deleted_at_block: tx.block_number,
            },
        )
        .await?;

        Ok(())
    }
}
