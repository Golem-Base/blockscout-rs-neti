use alloy_rlp::Decodable;
use anyhow::{anyhow, Context, Result};
use futures::{StreamExt, TryStreamExt};
use golem_base_sdk::entity::{
    Create, EncodableGolemBaseTransaction, Extend, GolemBaseDelete, Update,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use serde::Deserialize;
use serde_with::serde_as;
use std::{sync::Arc, time};
use tokio::time::sleep;
use tracing::instrument;

use crate::{
    golem_base::{decode_extend_log_data, entity_key},
    repository::entities::{
        GolemBaseEntityCreate, GolemBaseEntityDelete, GolemBaseEntityExtend, GolemBaseEntityUpdate,
    },
    types::{
        Entity, EntityKey, EntityStatus, FullNumericAnnotation, FullStringAnnotation, Log,
        NumericAnnotation, Operation, OperationData, OperationMetadata, StringAnnotation, Tx,
        TxHash,
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
// FIXME test what happens when DB connection fails
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
        repository::blockscout::stream_unprocessed_tx_hashes(&*self.db)
            .await?
            .map(Ok)
            .try_for_each_concurrent(self.settings.concurrency, |tx| async move {
                self.handle_tx(tx).await
            })
            .await?;

        repository::blockscout::stream_tx_hashes_for_cleanup(&*self.db)
            .await?
            .map(Ok)
            .try_for_each_concurrent(self.settings.concurrency, |tx| async move {
                self.handle_tx_cleanup(tx).await
            })
            .await
    }

    #[instrument(skip(self))]
    async fn handle_tx_cleanup(&self, tx_hash: TxHash) -> Result<()> {
        tracing::info!("Processing tx cleanup after reorg");
        let txn = self.db.begin().await?;

        let affected_entities: Vec<EntityKey> =
            repository::entities::find_by_tx_hash(&txn, tx_hash)
                .await
                .with_context(|| format!("Finding entities for tx hash {tx_hash}"))?
                .into_iter()
                .map(|e| e.key)
                .collect();

        repository::operations::delete_by_tx_hash(&txn, tx_hash)
            .await
            .with_context(|| format!("Deleting operations for tx hash {tx_hash}"))?;

        for entity in affected_entities {
            self.reindex_entity(&txn, entity).await?;
        }

        repository::transactions::finish_tx_cleanup(&txn, tx_hash).await?;

        txn.commit().await?;
        Ok(())
    }

    async fn reindex_entity_with_latest_operation<T: ConnectionTrait>(
        &self,
        txn: &T,
        latest_op: Operation,
        entity: EntityKey,
    ) -> Result<()> {
        let create_op = repository::operations::find_create_operation(txn, entity).await?;
        let delete_op = repository::operations::find_delete_operation(txn, entity).await?;
        let update_op = repository::operations::find_latest_update_operation(txn, entity).await?;

        let create_op = create_op.as_ref();
        let delete_op = delete_op.as_ref();
        let update_op = update_op.as_ref();

        let last_extend_expires_at = repository::logs::find_latest_extend_log(txn, entity)
            .await?
            .map(|v| decode_extend_log_data(&v.data))
            .transpose()
            .ok()
            .flatten();

        let data = match delete_op {
            Some(_) => None,
            None => update_op
                .map(|v| v.operation.data().expect("Update op always has data"))
                .or(create_op.map(|v| v.operation.data().expect("Create op always has data"))),
        };

        let status = match delete_op {
            Some(x) if x.metadata.recipient == well_known::L1_BLOCK_CONTRACT_ADDRESS => {
                EntityStatus::Expired
            }
            Some(_) => EntityStatus::Deleted,
            _ => EntityStatus::Active,
        };

        let expires_at_block_number = match latest_op.operation {
            OperationData::Create(_, _) => {
                let op = create_op.expect("It's latest tx so it exists");
                let btl = op.operation.btl().expect("Creates have BTL");
                let create_tx = repository::blockscout::get_tx(txn, op.metadata.tx_hash)
                    .await?
                    .expect("If we have op, then we have tx");

                // taking default for block number here might look bad, but if there's no block
                // number it means that we had a chain reorg and tx was dropped. we're not handling it right
                // now, or we would have dropped the create_tx, so it must still be in the queue - so it will
                // be processed right after we finish with what we're doing here and we'll reprocess the
                // expiration either way
                create_tx.block_number.unwrap_or_default() + btl
            }

            OperationData::Update(_, _) => {
                let op = update_op.expect("It's latest tx so it exists");
                let btl = op.operation.btl().expect("Updates have BTL");
                let update_tx = repository::blockscout::get_tx(txn, op.metadata.tx_hash)
                    .await?
                    .expect("If we have op, then we have tx");
                update_tx.block_number.unwrap_or_default() + btl
            }

            OperationData::Delete => {
                let op = delete_op.expect("It's latest tx so it exists");
                let delete_tx = repository::blockscout::get_tx(txn, op.metadata.tx_hash)
                    .await?
                    .expect("If we have op, then we have tx");
                delete_tx.block_number.unwrap_or_default()
            }

            OperationData::Extend(_) => last_extend_expires_at.expect("It's latest so it exists"),
        };

        let entity = Entity {
            key: entity,
            data: data.map(|v| v.to_owned()),
            owner: latest_op.metadata.sender,
            status,
            created_at_tx_hash: create_op.map(|v| v.metadata.tx_hash),
            last_updated_at_tx_hash: latest_op.metadata.tx_hash,
            expires_at_block_number,
        };

        repository::entities::replace_entity(txn, entity).await?;
        Ok(())
    }

    #[instrument(skip(self, txn))]
    async fn reindex_entity<T: ConnectionTrait>(&self, txn: &T, entity: EntityKey) -> Result<()> {
        match repository::operations::find_latest_operation(txn, entity).await? {
            Some(latest_op) => {
                self.reindex_entity_with_latest_operation(txn, latest_op, entity)
                    .await?
            }
            None => repository::entities::drop_entity(txn, entity).await?,
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn handle_tx(&self, tx_hash: TxHash) -> Result<()> {
        tracing::info!("Processing tx");

        let txn = self.db.begin().await?;

        let tx = repository::blockscout::get_tx(&txn, tx_hash)
            .await
            .with_context(|| format!("Getting tx {tx_hash}"))?;
        let tx = tx.ok_or(anyhow!("Somehow tx disappeared from the DB"))?;

        if tx.to_address_hash == well_known::GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS {
            let storagetx = match EncodableGolemBaseTransaction::decode(&mut &*tx.input) {
                Ok(storagetx) => storagetx,
                Err(e) => {
                    tracing::warn!(?e, "Storage tx with undecodable data");
                    return Ok(());
                }
            };

            // following operations are a good candidate for optimization when needed
            // possible improvements include parallelization and batching
            let mut idx = 0;
            for create in storagetx.creates {
                self.handle_create(&txn, &tx, create, idx)
                    .await
                    .with_context(|| format!("Handling create op tx_hash={tx_hash} idx={idx}"))?;
                idx += 1;
            }
            for delete in storagetx.deletes {
                self.handle_delete(&txn, &tx, delete, idx)
                    .await
                    .with_context(|| format!("Handling delete op tx_hash={tx_hash} idx={idx}"))?;
                idx += 1;
            }
            for update in storagetx.updates {
                self.handle_update(&txn, &tx, update, idx)
                    .await
                    .with_context(|| format!("Handling update op tx_hash={tx_hash} idx={idx}"))?;
                idx += 1;
            }
            for extend in storagetx.extensions {
                self.handle_extend(&txn, &tx, extend, idx)
                    .await
                    .with_context(|| format!("Handling extend op tx_hash={tx_hash} idx={idx}"))?;
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

    #[instrument(skip_all, fields(create, idx))]
    async fn handle_create(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        create: Create,
        idx: u64,
    ) -> Result<()> {
        let key = entity_key(tx.hash, create.data.clone(), idx);
        tracing::info!("Processing Create operation");

        let latest_update = self.is_latest_update(txn, key, tx.hash, idx).await?;

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: key,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash.expect("We only process txes with block hash"),
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
                sender: tx.from_address_hash,
                created_at: tx.hash,
                expires_at: tx
                    .block_number
                    .expect("We only process txes with block number")
                    .saturating_add(create.btl),
            },
        )
        .await?;

        for annotation in create.string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                FullStringAnnotation {
                    entity_key: key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    annotation: StringAnnotation {
                        key: annotation.key,
                        value: annotation.value,
                    },
                },
                latest_update,
            )
            .await?;
        }

        for annotation in create.numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                FullNumericAnnotation {
                    entity_key: key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    annotation: NumericAnnotation {
                        key: annotation.key,
                        value: annotation.value,
                    },
                },
                latest_update,
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip_all, fields(update, idx))]
    async fn handle_update(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        update: Update,
        idx: u64,
    ) -> Result<()> {
        tracing::info!("Processing Update operation");

        let latest_update = self
            .is_latest_update(txn, update.entity_key, tx.hash, idx)
            .await?;

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: update.entity_key,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx
                        .block_hash
                        .expect("We only process txses with block hash"),
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
                    sender: tx.from_address_hash,
                    updated_at: tx.hash,
                    expires_at: tx
                        .block_number
                        .expect("We only process txes with block number")
                        .saturating_add(update.btl),
                },
            )
            .await?;

            repository::annotations::deactivate_annotations(txn, update.entity_key).await?;
        }

        for annotation in update.string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                FullStringAnnotation {
                    entity_key: update.entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    annotation: StringAnnotation {
                        key: annotation.key,
                        value: annotation.value,
                    },
                },
                latest_update,
            )
            .await?;
        }

        for annotation in update.numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                FullNumericAnnotation {
                    entity_key: update.entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    annotation: NumericAnnotation {
                        key: annotation.key,
                        value: annotation.value,
                    },
                },
                latest_update,
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip_all, fields(delete, idx))]
    async fn handle_delete(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        delete: GolemBaseDelete,
        idx: u64,
    ) -> Result<()> {
        tracing::info!("Processing Delete operation");

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: delete,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx
                        .block_hash
                        .expect("We only process txses with block hash"),
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
                sender: tx.from_address_hash,
                deleted_at_tx: tx.hash,
                deleted_at_block: tx
                    .block_number
                    .expect("We only process txses with block number"),
            },
        )
        .await?;

        repository::annotations::deactivate_annotations(txn, delete).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(extend, idx))]
    async fn handle_extend(
        &self,
        txn: &DatabaseTransaction,
        tx: &Tx,
        extend: Extend,
        idx: u64,
    ) -> Result<()> {
        tracing::info!("Processing Extend operation");
        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key: extend.entity_key,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx
                        .block_hash
                        .expect("We only process txses with block hash"),
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
        let entity = repository::entities::get_entity(txn, entity_key).await?;

        if let Some(entity) = entity {
            if !matches!(entity.status, EntityStatus::Active) {
                return Ok(false);
            }
        }

        let latest_stored_update =
            repository::operations::get_latest_update(txn, entity_key).await?;
        let latest_stored_update = if let Some(update) = latest_stored_update {
            update
        } else {
            return Ok(true);
        };

        let candidate_update = repository::blockscout::get_tx(txn, tx_hash).await?;
        let candidate_update = if let Some(tx) = candidate_update {
            tx
        } else {
            tracing::warn!(tx=?tx_hash, "Transaction disappeared from the database");
            return Ok(true);
        };

        let candidate_update = (
            candidate_update
                .block_number
                .expect("We only process txses with block number"),
            candidate_update
                .index
                .expect("We only process txses with index"),
            operation_index,
        );

        Ok(candidate_update > latest_stored_update)
    }

    #[instrument(skip_all, fields(log))]
    async fn handle_extend_log(&self, txn: &DatabaseTransaction, tx: &Tx, log: Log) -> Result<()> {
        // extends are handled after updates, so if it's in the same tx, we need to process it
        let entity_key = if let Some(k) = log.second_topic {
            k
        } else {
            tracing::warn!("Extend Log event with no second topic?");
            return Ok(());
        };
        tracing::info!("Processing extend log for entity {entity_key}");

        let latest_update = self
            .is_latest_update(txn, entity_key, tx.hash, u64::MAX)
            .await?;

        if latest_update {
            let expires_at_block_number = if let Ok(v) = decode_extend_log_data(&log.data) {
                v
            } else {
                tracing::warn!(data=?log.data, "Invalid GolemBaseStorageEntityBTLExtended event data encountered");
                return Ok(());
            };
            repository::entities::extend_entity(
                txn,
                GolemBaseEntityExtend {
                    key: entity_key,
                    extended_at: tx.hash,
                    sender: tx.from_address_hash,
                    expires_at: expires_at_block_number,
                },
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip_all, fields(log))]
    async fn handle_expire_log(&self, txn: &DatabaseTransaction, tx: &Tx, log: Log) -> Result<()> {
        let entity_key = if let Some(k) = log.second_topic {
            k
        } else {
            tracing::warn!("Delete Log event with no second topic?");
            return Ok(());
        };
        tracing::info!("Processing delete log for entity {entity_key}");

        repository::operations::insert_operation(
            txn,
            Operation {
                metadata: OperationMetadata {
                    entity_key,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx
                        .block_hash
                        .expect("We only process txses with block hash"),
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
                sender: tx.from_address_hash,
                deleted_at_tx: tx.hash,
                deleted_at_block: tx
                    .block_number
                    .expect("We only process txses with block number"),
            },
        )
        .await?;

        Ok(())
    }
}
