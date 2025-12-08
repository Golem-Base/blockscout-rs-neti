use alloy_sol_types::SolEvent;
use anyhow::{anyhow, Context, Result};
use arkiv_storage_tx::{ArkivABI, ChangeOwner, Create, Delete, Extend, StorageTransaction, Update};
use futures::StreamExt;
use lazy_static::lazy_static;
use prometheus::{opts, register_counter, register_gauge, Counter, Gauge};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use serde::Deserialize;
use serde_with::serde_as;
use std::{
    collections::HashSet,
    sync::Arc,
    time::{self, Duration},
};
use tokio::time::sleep;
use tracing::{instrument, warn};

use crate::{
    arkiv::{block_timestamp, block_timestamp_sec, entity_key},
    types::{
        Block, ConsensusTx, CurrencyAmount, EntityHistoryEntry, EntityKey, EntityStatus,
        FullNumericAttribute, FullOperationIndex, FullStringAttribute, ListOperationsFilter,
        LogEventIndex, LogIndex, Operation, OperationData, OperationMetadata, OperationType,
        OperationsFilter, PaginationParams, Timestamp, TxHash,
    },
};

#[cfg(feature = "test-utils")]
pub mod test_utils;

pub mod arkiv;
mod attributes;
mod consensus_tx;
pub mod mat_view_scheduler;
pub mod model;
mod operations;
pub mod pagination;
pub mod repository;
pub mod services;
pub mod types;
pub mod well_known;

lazy_static! {
    static ref TX_COUNTER: Counter = register_counter!(opts!(
        "processed_transaction_count",
        "Number of transactions processed.",
    ))
    .unwrap();
    static ref OP_COUNTER: Counter = register_counter!(opts!(
        "processed_operation_count",
        "Number of operations processed.",
    ))
    .unwrap();
    static ref TX_REORG_COUNTER: Counter = register_counter!(opts!(
        "processed_transaction_reorg_count",
        "Number of transaction reorgs processed.",
    ))
    .unwrap();
    static ref PENDING_TX_GAUGE: Gauge = register_gauge!(opts!(
        "pending_transactions",
        "Number of transactions to be processed.",
    ))
    .unwrap();
    static ref PENDING_TX_REORG_GAUGE: Gauge = register_gauge!(opts!(
        "pending_transaction_reorgs",
        "Number of transaction reorgs to be processed.",
    ))
    .unwrap();
}

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

    pub async fn update_gauges(&self) -> ! {
        loop {
            match repository::blockscout::count_unprocessed_txs(&*self.db).await {
                Ok(v) => PENDING_TX_GAUGE.set(v as f64),
                Err(e) => warn!(?e, "Failed to update metrics"),
            }

            match repository::blockscout::count_txs_for_cleanup(&*self.db).await {
                Ok(v) => PENDING_TX_REORG_GAUGE.set(v as f64),
                Err(e) => warn!(?e, "Failed to update metrics"),
            }
            sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn process_batch_of_transactions(&self) -> Result<()> {
        repository::blockscout::stream_unprocessed_tx_hashes(&*self.db)
            .await?
            .map(|tx| async move {
                self.handle_tx(tx)
                    .await
                    .inspect_err(|e| tracing::warn!(?e, ?tx, "Handling tx failed"))
                    .unwrap_or_default() // ignore error, it will be retried
            })
            .buffer_unordered(self.settings.concurrency)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn process_reindexes(&self) -> Result<()> {
        repository::entities::stream_entities_to_reindex(&*self.db)
            .await?
            .map(|key| {
                async move {
                    self.reindex_entity(key)
                        .await
                        .inspect_err(|e| tracing::warn!(?e, ?key, "Handling tx reindex failed"))
                        .unwrap_or_default() // ignore error, it will be retried
                }
            })
            .buffer_unordered(self.settings.concurrency)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    pub async fn process_delete_logs(&self) -> Result<()> {
        repository::blockscout::stream_unprocessed_logs(&*self.db)
            .await?
            .for_each_concurrent(self.settings.concurrency, |log| async move {
                let _ = self
                    .handle_log(log.clone())
                    .await
                    .inspect_err(|e| tracing::warn!(?e, ?log, "Handling log failed"));
            })
            .await;
        Ok(())
    }

    pub async fn process_logs_events(&self) -> Result<()> {
        let txn = self.db.begin().await?;
        let affected_entities: Vec<EntityKey> =
            repository::blockscout::stream_unprocessed_logs_events(&*self.db)
                .await?
                .map(|log| {
                    let txn = &txn;
                    async move {
                        self.handle_log_event(txn, log)
                            .await
                            .inspect_err(|e| tracing::warn!(?e, "Handling log failed"))
                            .unwrap_or_default()
                    }
                })
                .buffer_unordered(self.settings.concurrency)
                .collect()
                .await;

        if !affected_entities.is_empty() {
            repository::entities::batch_queue_reindex(&*self.db, affected_entities).await?;
        }

        txn.commit().await?;

        Ok(())
    }

    pub async fn process_tx_cleanups(&self) -> Result<()> {
        let txn = self.db.begin().await?;
        let affected_entities = repository::blockscout::stream_tx_hashes_for_cleanup(&*self.db)
            .await?
            .map(|tx| {
                let txn = &txn;
                async move {
                    self.handle_tx_cleanup(txn, tx)
                        .await
                        .inspect_err(|e| tracing::warn!(?e, ?tx, "Handling tx cleanup failed"))
                        .unwrap_or_default() // ignore error, it will be retried
                }
            })
            .buffer_unordered(self.settings.concurrency)
            .collect::<Vec<_>>()
            .await;

        if !affected_entities.is_empty() {
            repository::entities::batch_queue_reindex(
                &*self.db,
                affected_entities.into_iter().flatten().collect(),
            )
            .await?;
        }

        txn.commit().await?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn tick(&self) -> Result<()> {
        self.process_batch_of_transactions().await?;
        self.process_delete_logs().await?;
        self.process_tx_cleanups().await?;
        self.process_logs_events().await?;
        self.process_reindexes().await?;

        Ok(())
    }

    #[instrument(skip(self, txn))]
    async fn handle_tx_cleanup(
        &self,
        txn: &DatabaseTransaction,
        tx_hash: TxHash,
    ) -> Result<HashSet<EntityKey>> {
        tracing::info!("Processing tx cleanup after reorg");

        let affected_entities: Vec<EntityKey> = repository::entities::find_by_tx_hash(txn, tx_hash)
            .await
            .with_context(|| format!("Finding entities for tx hash {tx_hash}"))?
            .into_iter()
            .map(|e| e.key)
            .collect();

        repository::operations::delete_by_tx_hash(txn, tx_hash)
            .await
            .with_context(|| format!("Deleting operations for tx hash {tx_hash}"))?;

        repository::transactions::finish_tx_processing(txn, tx_hash).await?;
        repository::transactions::finish_tx_cleanup(txn, tx_hash).await?;

        TX_REORG_COUNTER.inc();

        Ok(affected_entities.into_iter().collect())
    }

    async fn reindex_entity_with_ops<T: ConnectionTrait>(
        &self,
        txn: &T,
        entity: EntityKey,
    ) -> Result<()> {
        let (ops, _) = repository::operations::list_operations(
            txn,
            ListOperationsFilter {
                pagination: PaginationParams {
                    page: 0,
                    page_size: i64::MAX as u64,
                },
                operation_type: None,
                operations_filter: OperationsFilter {
                    entity_key: Some(entity),
                    ..Default::default()
                },
            },
        )
        .await?;

        repository::entities::delete_history(txn, entity).await?;
        let mut prev_entry: Option<EntityHistoryEntry> = None;
        let mut active_attributes_index = None;
        let mut entries = Vec::new();
        for op in ops {
            active_attributes_index = match op.op.operation {
                OperationData::Delete => None,
                OperationData::Extend(_) => active_attributes_index,
                _ => Some((op.op.metadata.tx_hash, op.op.metadata.index)),
            };

            let entry = self.build_history_entry(op.op, op.block_timestamp, prev_entry.as_ref());
            entries.push(entry.clone());
            prev_entry = Some(entry);
        }
        repository::entities::batch_insert_history_entry(txn, entries).await?;
        repository::attributes::deactivate_attributes(txn, entity).await?;
        if let Some(active_attributes_index) = active_attributes_index {
            repository::attributes::activate_attributes(txn, entity, active_attributes_index)
                .await?;
        }

        Ok(())
    }

    #[instrument(skip_all, fields(entity))]
    pub async fn reindex_entity(&self, entity: EntityKey) -> Result<()> {
        tracing::info!(?entity, "Reprocessing entity");
        let txn = self.db.begin().await?;
        match repository::operations::find_latest_operation(&txn, entity).await? {
            Some(_) => self.reindex_entity_with_ops(&txn, entity).await?,
            None => repository::entities::drop_entity(&txn, entity).await?,
        }
        repository::entities::refresh_entity_based_on_history(&txn, entity).await?;
        repository::entities::finish_reindex(&txn, entity).await?;
        txn.commit().await?;
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
        let tx: ConsensusTx = tx.try_into()?;

        let mut op_idx = 0;
        let storagetx: StorageTransaction = match (&tx.input).try_into() {
            Ok(storagetx) => storagetx,
            Err(e) => {
                tracing::warn!(?e, "Storage tx with undecodable data");
                return Ok(());
            }
        };

        // following operations are a good candidate for optimization when needed
        // possible improvements include parallelization and batching
        let mut ops = Vec::new();
        let mut string_attributes = Vec::new();
        let mut numeric_attributes = Vec::new();
        for create in storagetx.creates {
            let (op, op_string_attributes, op_numeric_attributes) = self
                .handle_create(&tx, create, op_idx)
                .await
                .with_context(|| format!("Handling create op tx_hash={tx_hash} op_idx={op_idx}"))?;
            ops.push(op);
            string_attributes.extend(op_string_attributes);
            numeric_attributes.extend(op_numeric_attributes);
            op_idx += 1;
        }
        for delete in storagetx.deletes {
            let op = self
                .handle_delete(&tx, delete, op_idx)
                .await
                .with_context(|| format!("Handling delete op tx_hash={tx_hash} op_idx={op_idx}"))?;
            ops.push(op);
            op_idx += 1;
        }
        for update in storagetx.updates {
            let (op, op_string_attributes, op_numeric_attributes) = self
                .handle_update(&tx, update, op_idx)
                .await
                .with_context(|| format!("Handling update op tx_hash={tx_hash} op_idx={op_idx}"))?;
            ops.push(op);
            string_attributes.extend(op_string_attributes);
            numeric_attributes.extend(op_numeric_attributes);
            op_idx += 1;
        }
        for extend in storagetx.extensions {
            let op = self
                .handle_extend(&tx, extend, op_idx)
                .await
                .with_context(|| format!("Handling extend op tx_hash={tx_hash} op_idx={op_idx}"))?;
            ops.push(op);
            op_idx += 1;
        }
        for change_owner in storagetx.change_owners {
            let op = self
                .handle_change_owner(&tx, change_owner, op_idx)
                .await
                .with_context(|| {
                    format!("Handling change_owner op tx_hash={tx_hash} op_idx={op_idx}")
                })?;
            ops.push(op);
            op_idx += 1;
        }

        if !ops.is_empty() {
            repository::entities::batch_queue_reindex(
                &txn,
                ops.iter().map(|v| v.metadata.entity_key).collect(),
            )
            .await?;
            repository::operations::batch_insert_operation(&txn, ops).await?;
        }
        if !string_attributes.is_empty() {
            repository::attributes::batch_insert_string_attribute(&txn, string_attributes).await?;
        }
        if !numeric_attributes.is_empty() {
            repository::attributes::batch_insert_numeric_attribute(&txn, numeric_attributes)
                .await?;
        }

        repository::transactions::finish_tx_processing(&txn, tx_hash).await?;
        txn.commit().await?;

        TX_COUNTER.inc();
        OP_COUNTER.inc_by(op_idx as f64);
        Ok(())
    }

    #[instrument(skip_all, fields(create, idx))]
    async fn handle_create(
        &self,
        tx: &ConsensusTx,
        create: Create,
        idx: u64,
    ) -> Result<(
        Operation,
        Vec<FullStringAttribute>,
        Vec<FullNumericAttribute>,
    )> {
        let key = entity_key(tx.hash, create.payload.clone(), idx);

        Ok((
            Operation {
                metadata: OperationMetadata {
                    entity_key: key,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: idx,
                    tx_index: tx.index,
                    block_number: tx.block_number,
                    cost: None,
                },
                operation: OperationData::create(
                    create.payload.clone(),
                    create.btl,
                    &create.content_type,
                ),
            },
            create
                .string_attributes
                .into_iter()
                .map(|v| FullStringAttribute {
                    entity_key: key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    attribute: v.into(),
                })
                .collect(),
            create
                .numeric_attributes
                .into_iter()
                .map(|v| FullNumericAttribute {
                    entity_key: key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    attribute: v.into(),
                })
                .collect(),
        ))
    }

    fn build_history_entry(
        &self,
        op: Operation,
        block_ts: Timestamp,
        prev_entry: Option<&EntityHistoryEntry>,
    ) -> EntityHistoryEntry {
        let reference_block = Block {
            hash: op.metadata.block_hash,
            number: op.metadata.block_number,
            timestamp: block_ts,
        };
        let status = match op.operation {
            OperationData::Delete
                if op.metadata.recipient == well_known::L1_BLOCK_CONTRACT_ADDRESS =>
            {
                EntityStatus::Expired
            }
            OperationData::Delete => EntityStatus::Deleted,
            _ => EntityStatus::Active,
        };
        let owner = match op.operation {
            OperationData::Delete
                if op.metadata.recipient == well_known::L1_BLOCK_CONTRACT_ADDRESS =>
            {
                prev_entry.and_then(|v| v.owner)
            }
            OperationData::ChangeOwner(new_owner) => Some(new_owner),
            _ => Some(op.metadata.sender),
        };
        let data = match op.operation {
            OperationData::Extend(_) => prev_entry.and_then(|v| v.data.clone()),
            OperationData::ChangeOwner(_) => prev_entry.and_then(|v| v.data.clone()),
            _ => op.operation.data().map(ToOwned::to_owned),
        };

        let expires_at_block_number = match op.operation {
            OperationData::Create(_, btl, _) => Some(op.metadata.block_number + btl),
            OperationData::Update(_, btl, _) => Some(op.metadata.block_number + btl),
            OperationData::Extend(extend_btl) => {
                prev_entry.and_then(|v| v.expires_at_block_number.map(|v| v + extend_btl))
            }
            OperationData::Delete => Some(op.metadata.block_number),
            OperationData::ChangeOwner(_) => prev_entry.and_then(|v| v.expires_at_block_number),
        };

        let expires_at_timestamp =
            expires_at_block_number.and_then(|v| block_timestamp(v, &reference_block));
        let expires_at_timestamp_sec =
            expires_at_block_number.and_then(|v| block_timestamp_sec(v, &reference_block));
        let content_type = match op.operation {
            OperationData::Extend(_) => prev_entry.and_then(|v| v.content_type.clone()),
            OperationData::ChangeOwner(_) => prev_entry.and_then(|v| v.content_type.clone()),
            _ => op.operation.content_type(),
        };

        let total_cost = Some(
            prev_entry
                .and_then(|v| v.total_cost)
                .unwrap_or(CurrencyAmount::ZERO)
                .saturating_add(op.metadata.cost.unwrap_or(CurrencyAmount::ZERO)),
        );

        EntityHistoryEntry {
            entity_key: op.metadata.entity_key,
            block_number: op.metadata.block_number,
            block_hash: op.metadata.block_hash,
            transaction_hash: op.metadata.tx_hash,
            tx_index: op.metadata.tx_index,
            op_index: op.metadata.index,
            block_timestamp: reference_block.timestamp,
            owner,
            prev_owner: prev_entry.and_then(|prev_entry| prev_entry.owner),
            sender: op.metadata.sender,
            data,
            prev_data: prev_entry.and_then(|prev_entry| prev_entry.data.clone()),
            operation: op.operation.clone().into(),
            status,
            prev_status: prev_entry.map(|prev_entry| prev_entry.status),
            expires_at_block_number,
            prev_expires_at_block_number: prev_entry
                .and_then(|prev_entry| prev_entry.expires_at_block_number),
            expires_at_timestamp,
            expires_at_timestamp_sec,
            prev_expires_at_timestamp: prev_entry
                .and_then(|prev_entry| prev_entry.expires_at_timestamp),
            prev_expires_at_timestamp_sec: prev_entry
                .and_then(|prev_entry| prev_entry.expires_at_timestamp_sec),
            btl: op.operation.btl(),
            content_type,
            prev_content_type: prev_entry.and_then(|prev_entry| prev_entry.content_type.clone()),
            cost: op.metadata.cost,
            total_cost,
        }
    }

    #[instrument(skip_all, fields(update, idx))]
    async fn handle_update(
        &self,
        tx: &ConsensusTx,
        update: Update,
        idx: u64,
    ) -> Result<(
        Operation,
        Vec<FullStringAttribute>,
        Vec<FullNumericAttribute>,
    )> {
        Ok((
            Operation {
                metadata: OperationMetadata {
                    entity_key: update.entity_key,
                    sender: tx.from_address_hash,
                    recipient: tx.to_address_hash,
                    tx_hash: tx.hash,
                    block_hash: tx.block_hash,
                    index: idx,
                    block_number: tx.block_number,
                    tx_index: tx.index,
                    cost: None,
                },
                operation: OperationData::update(
                    update.payload.clone(),
                    update.btl,
                    &update.content_type,
                ),
            },
            update
                .string_attributes
                .into_iter()
                .map(|v| FullStringAttribute {
                    entity_key: update.entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    attribute: v.into(),
                })
                .collect(),
            update
                .numeric_attributes
                .into_iter()
                .map(|v| FullNumericAttribute {
                    entity_key: update.entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: idx,
                    attribute: v.into(),
                })
                .collect(),
        ))
    }

    #[instrument(skip_all, fields(delete, idx))]
    async fn handle_delete(&self, tx: &ConsensusTx, delete: Delete, idx: u64) -> Result<Operation> {
        Ok(Operation {
            metadata: OperationMetadata {
                entity_key: delete,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                block_number: tx.block_number,
                tx_index: tx.index,
                cost: None,
            },
            operation: OperationData::delete(),
        })
    }

    #[instrument(skip_all, fields(extend, idx))]
    async fn handle_extend(&self, tx: &ConsensusTx, extend: Extend, idx: u64) -> Result<Operation> {
        Ok(Operation {
            metadata: OperationMetadata {
                entity_key: extend.entity_key,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                block_number: tx.block_number,
                tx_index: tx.index,
                cost: None,
            },
            operation: OperationData::extend(extend.number_of_blocks),
        })
    }

    #[instrument(skip_all, fields(extend, idx))]
    async fn handle_change_owner(
        &self,
        tx: &ConsensusTx,
        change_owner: ChangeOwner,
        idx: u64,
    ) -> Result<Operation> {
        Ok(Operation {
            metadata: OperationMetadata {
                entity_key: change_owner.entity_key,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                block_number: tx.block_number,
                tx_index: tx.index,
                cost: None,
            },
            operation: OperationData::ChangeOwner(change_owner.new_owner),
        })
    }

    #[instrument(skip_all, fields(log))]
    async fn handle_log(&self, log: LogIndex) -> Result<()> {
        let txn = self.db.begin().await?;
        let tx = repository::blockscout::get_tx(&txn, log.transaction_hash)
            .await?
            .ok_or(anyhow!("Log with no tx!"))?;
        let tx: ConsensusTx = tx.try_into()?;
        let log = repository::logs::get_log(&txn, log)
            .await?
            .ok_or(anyhow!("Log disappeared from the DB?!"))?;
        let entity_key = if let Some(k) = log.second_topic {
            k
        } else {
            tracing::warn!("Delete Log event with no second topic?");
            return Ok(());
        };
        tracing::info!("Processing delete log for entity {entity_key}");

        let op = Operation {
            metadata: OperationMetadata {
                entity_key,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                block_number: tx.block_number,
                tx_index: tx.index,
                index: log.index,
                cost: None,
            },
            operation: OperationData::delete(),
        };
        repository::operations::insert_operation(&txn, op.clone()).await?;

        let idx = FullOperationIndex {
            block_number: tx.block_number,
            tx_index: tx.index,
            op_index: op.metadata.index,
        };
        let prev_entry = repository::entities::get_latest_entity_history_entry(
            &txn,
            entity_key,
            Some(idx.clone()),
        )
        .await?;
        let entry = self.build_history_entry(op, tx.block_timestamp, prev_entry.as_ref());
        repository::entities::batch_insert_history_entry(&txn, vec![entry]).await?;
        repository::entities::refresh_entity_based_on_history(&txn, entity_key).await?;

        repository::attributes::deactivate_attributes(&txn, entity_key).await?;
        repository::logs::finish_log_processing(&txn, tx.hash, tx.block_hash, log.index).await?;
        txn.commit().await?;
        OP_COUNTER.inc();

        Ok(())
    }

    #[instrument(skip_all, fields(log))]
    async fn handle_log_event(
        &self,
        txn: &DatabaseTransaction,
        log: LogEventIndex,
    ) -> Result<EntityKey> {
        tracing::info!(
            "Processing event log for tx_hash={}, op_index={}",
            log.transaction_hash,
            log.op_index
        );

        // Extract operation type and cost
        let (_op_type, cost) = match log.signature_hash {
            ArkivABI::ArkivEntityCreated::SIGNATURE_HASH => {
                let (_expiration_block, cost) =
                    ArkivABI::ArkivEntityCreated::abi_decode_data_validate(&log.data).map_err(
                        |e| anyhow!("Error decoding non-indexed parameters for event log: {e}"),
                    )?;
                (OperationType::Create, cost)
            }
            ArkivABI::ArkivEntityUpdated::SIGNATURE_HASH => {
                let (_old_expiration_block, _new_expiration_block, cost) =
                    ArkivABI::ArkivEntityUpdated::abi_decode_data_validate(&log.data).map_err(
                        |e| anyhow!("Error decoding non-indexed parameters for event log: {e}"),
                    )?;
                (OperationType::Update, cost)
            }
            ArkivABI::ArkivEntityBTLExtended::SIGNATURE_HASH => {
                let (_old_expiration_block, _new_expiration_block, cost) =
                    ArkivABI::ArkivEntityBTLExtended::abi_decode_data_validate(&log.data)
                        .map_err(|e| anyhow!("Error decoding non-indexed parameters. log={e}"))?;
                (OperationType::Extend, cost)
            }
            _ => {
                return Err(anyhow!(
                    "Unnrecognized event. signature_hash={}",
                    log.signature_hash
                ));
            }
        };

        // Get stored operation
        let mut op = repository::operations::get_operation(txn, log.transaction_hash, log.op_index)
            .await
            .map_err(|e| anyhow!("Error fetching operation for an event: {e}"))?
            .ok_or(anyhow!("No matching operation found for an event."))?;

        // Warn when overwriting operation cost
        if let Some(current_cost) = op.metadata.cost {
            tracing::warn!(?log.transaction_hash, log.op_index, "Replacing current operation cost ({}) with a new value ({})", current_cost.to_string(), cost.to_string());
        }

        // Set cost and update operation
        op.metadata.cost = Some(cost);
        let entity_key = op.metadata.entity_key;
        repository::operations::update_operation(txn, op).await?;

        // Remove log from the pending queue
        repository::logs::finish_log_event_processing(
            txn,
            log.transaction_hash,
            log.block_hash,
            log.index,
        )
        .await?;

        Ok(entity_key)
    }
}
