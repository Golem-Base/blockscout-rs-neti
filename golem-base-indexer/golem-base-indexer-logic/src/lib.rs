use alloy_rlp::Decodable;
use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use golem_base_sdk::entity::{
    Create, EncodableGolemBaseTransaction, Extend, GolemBaseDelete, Update,
};
use lazy_static::lazy_static;
use prometheus::{opts, register_counter, register_gauge, Counter, Gauge};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use serde::Deserialize;
use serde_with::serde_as;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
    time::{self, Duration},
};
use tokio::time::sleep;
use tracing::{instrument, warn};

use crate::{
    golem_base::{block_timestamp, entity_key},
    repository::locks::Guard,
    types::{
        Block, ConsensusTx, EntityHistoryEntry, EntityKey, EntityStatus, FullNumericAnnotation,
        FullOperationIndex, FullStringAnnotation, ListOperationsFilter, LogIndex,
        NumericAnnotation, Operation, OperationData, OperationMetadata, OperationsFilter,
        PaginationParams, StringAnnotation, TxHash,
    },
};

mod annotations;
mod consensus_tx;
pub mod golem_base;
pub mod mat_view_scheduler;
pub mod model;
pub mod pagination;
pub mod repository;
pub mod types;
pub mod updater_leaderboards;
pub mod updater_timeseries;
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
        repository::locks::clear(&*self.db).await?;
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

    #[instrument(skip_all)]
    pub async fn tick(&self) -> Result<()> {
        repository::blockscout::stream_unprocessed_tx_hashes(&*self.db)
            .await?
            .for_each_concurrent(self.settings.concurrency, |tx| async move {
                // ignore errors, it's most likely just a deadlock anyway, we'll just retry.
                let _ = self
                    .handle_tx(tx)
                    .await
                    .inspect_err(|e| tracing::warn!(?e, ?tx, "Handling tx failed"));
            })
            .await;

        repository::blockscout::stream_unprocessed_logs(&*self.db)
            .await?
            .for_each_concurrent(self.settings.concurrency, |log| async move {
                let _ = self
                    .handle_log(log.clone())
                    .await
                    .inspect_err(|e| tracing::warn!(?e, ?log, "Handling log failed"));
            })
            .await;

        repository::blockscout::stream_tx_hashes_for_cleanup(&*self.db)
            .await?
            .for_each_concurrent(self.settings.concurrency, |tx| async move {
                // ignore errors, it's most likely just a deadlock anyway, we'll just retry.
                let _ = self
                    .handle_tx_cleanup(tx)
                    .await
                    .inspect_err(|e| tracing::warn!(?e, ?tx, "Handling tx cleanup failed"));
            })
            .await;

        Ok(())
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

        let mut guards = HashMap::<_, _>::new();
        for entity in affected_entities {
            self.reindex_entity(&txn, Some(&mut guards), entity).await?;
        }

        repository::transactions::finish_tx_processing(&txn, tx_hash).await?;
        repository::transactions::finish_tx_cleanup(&txn, tx_hash).await?;

        for guard in guards.into_values() {
            guard.unlock(&txn).await?;
        }
        txn.commit().await?;
        TX_REORG_COUNTER.inc();
        Ok(())
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
        let owner = ops
            .iter()
            .find(|v| !matches!(v.op.operation, OperationData::Delete))
            .map(|v| v.op.metadata.sender);

        repository::entities::delete_history(txn, entity).await?;
        let mut prev_entry: Option<EntityHistoryEntry> = None;
        let mut active_annotations_index = None;
        for op in ops {
            let status = match op.op.operation {
                OperationData::Create(_, _) => EntityStatus::Active,
                OperationData::Update(_, _) => EntityStatus::Active,
                OperationData::Extend(_) => EntityStatus::Active,
                OperationData::Delete => {
                    if op.op.metadata.recipient == well_known::L1_BLOCK_CONTRACT_ADDRESS {
                        EntityStatus::Expired
                    } else {
                        EntityStatus::Deleted
                    }
                }
            };

            let expires_at_block_number = match op.op.operation {
                OperationData::Create(_, btl) => Some(op.op.metadata.block_number + btl),
                OperationData::Update(_, btl) => Some(op.op.metadata.block_number + btl),
                OperationData::Extend(extend_btl) => prev_entry
                    .as_ref()
                    .and_then(|v| v.expires_at_block_number.map(|v| v + extend_btl)),
                OperationData::Delete => Some(op.op.metadata.block_number),
            };

            let data = match op.op.operation {
                OperationData::Extend(_) => prev_entry.as_ref().and_then(|v| v.data.to_owned()),
                _ => op.op.operation.data().cloned(),
            };

            let expires_at_timestamp = expires_at_block_number.and_then(|v| {
                block_timestamp(
                    v,
                    &Block {
                        number: op.op.metadata.block_number,
                        timestamp: op.block_timestamp,
                        hash: op.op.metadata.block_hash,
                    },
                )
            });

            active_annotations_index = match op.op.operation {
                OperationData::Delete => None,
                OperationData::Extend(_) => active_annotations_index,
                _ => Some((op.op.metadata.tx_hash, op.op.metadata.index)),
            };

            let entry = EntityHistoryEntry {
                entity_key: entity,
                block_number: op.op.metadata.block_number,
                block_hash: op.op.metadata.block_hash,
                transaction_hash: op.op.metadata.tx_hash,
                tx_index: op.op.metadata.tx_index,
                op_index: op.op.metadata.index,
                block_timestamp: op.block_timestamp,
                owner,
                sender: op.op.metadata.sender,
                data,
                prev_data: prev_entry
                    .as_ref()
                    .and_then(|prev_entry| prev_entry.data.clone()),
                operation: op.op.operation.clone(),
                status,
                prev_status: prev_entry.as_ref().map(|prev_entry| prev_entry.status),
                expires_at_block_number,
                prev_expires_at_block_number: prev_entry
                    .as_ref()
                    .and_then(|prev_entry| prev_entry.expires_at_block_number),
                expires_at_timestamp,
                prev_expires_at_timestamp: prev_entry
                    .and_then(|prev_entry| prev_entry.expires_at_timestamp),
                btl: op.op.operation.btl(),
            };
            repository::entities::insert_history_entry(txn, entry.clone()).await?;
            prev_entry = Some(entry);
        }
        repository::annotations::deactivate_annotations(txn, entity).await?;
        if let Some(active_annotations_index) = active_annotations_index {
            repository::annotations::activate_annotations(txn, entity, active_annotations_index)
                .await?;
        }

        Ok(())
    }

    #[instrument(skip_all, fields(entity))]
    pub async fn reindex_entity<T: ConnectionTrait>(
        &self,
        txn: &T,
        guards: Option<&mut HashMap<EntityKey, Guard>>,
        entity: EntityKey,
    ) -> Result<()> {
        if let Some(guards) = guards {
            if let Entry::Vacant(e) = guards.entry(entity) {
                e.insert(repository::locks::lock(txn, entity).await?);
            }
        }
        match repository::operations::find_latest_operation(txn, entity).await? {
            Some(_) => self.reindex_entity_with_ops(txn, entity).await?,
            None => repository::entities::drop_entity(txn, entity).await?,
        }
        repository::entities::refresh_entity_based_on_history(txn, entity).await?;
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
        let mut guards = HashMap::<_, _>::new();
        let storagetx = match EncodableGolemBaseTransaction::decode(&mut &*tx.input) {
            Ok(storagetx) => storagetx,
            Err(e) => {
                tracing::warn!(?e, "Storage tx with undecodable data");
                return Ok(());
            }
        };

        // following operations are a good candidate for optimization when needed
        // possible improvements include parallelization and batching
        for create in storagetx.creates {
            self.handle_create(&txn, &mut guards, &tx, create, op_idx)
                .await
                .with_context(|| format!("Handling create op tx_hash={tx_hash} op_idx={op_idx}"))?;
            op_idx += 1;
        }
        for delete in storagetx.deletes {
            self.handle_delete(&txn, &mut guards, &tx, delete, op_idx)
                .await
                .with_context(|| format!("Handling delete op tx_hash={tx_hash} op_idx={op_idx}"))?;
            op_idx += 1;
        }
        for update in storagetx.updates {
            self.handle_update(&txn, &mut guards, &tx, update, op_idx)
                .await
                .with_context(|| format!("Handling update op tx_hash={tx_hash} op_idx={op_idx}"))?;
            op_idx += 1;
        }
        for extend in storagetx.extensions {
            self.handle_extend(&txn, &mut guards, &tx, extend, op_idx)
                .await
                .with_context(|| format!("Handling extend op tx_hash={tx_hash} op_idx={op_idx}"))?;
            op_idx += 1;
        }

        repository::transactions::finish_tx_processing(&txn, tx_hash).await?;
        for guard in guards.into_values() {
            guard.unlock(&txn).await?;
        }
        txn.commit().await?;

        TX_COUNTER.inc();
        OP_COUNTER.inc_by(op_idx as f64);
        Ok(())
    }

    #[instrument(skip_all, fields(create, idx))]
    async fn handle_create(
        &self,
        txn: &DatabaseTransaction,
        guards: &mut HashMap<EntityKey, Guard>,
        tx: &ConsensusTx,
        create: Create,
        idx: u64,
    ) -> Result<()> {
        let key = entity_key(tx.hash, create.data.clone(), idx);
        if let Entry::Vacant(e) = guards.entry(key) {
            e.insert(repository::locks::lock(txn, key).await?);
        }
        tracing::info!("Processing Create operation");

        let op = Operation {
            metadata: OperationMetadata {
                entity_key: key,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                tx_index: tx.index,
                block_number: tx.block_number,
            },
            operation: OperationData::create(create.data.clone(), create.btl),
        };
        repository::operations::insert_operation(txn, op.clone()).await?;

        if repository::entities::get_oldest_entity_history_entry(
            txn,
            key,
            FullOperationIndex {
                block_number: tx.block_number,
                tx_index: tx.index,
                op_index: idx,
            },
        )
        .await?
        .is_some()
        {
            self.reindex_entity_with_ops(txn, key).await?;
        } else {
            self.insert_history_entry(txn, key, tx, op).await?;
        }

        repository::entities::refresh_entity_based_on_history(txn, key).await?;

        self.store_annotations(
            txn,
            key,
            tx,
            idx,
            create
                .string_annotations
                .into_iter()
                .map(Into::into)
                .collect(),
            create
                .numeric_annotations
                .into_iter()
                .map(Into::into)
                .collect(),
        )
        .await?;

        Ok(())
    }

    async fn insert_history_entry<T: ConnectionTrait>(
        &self,
        txn: &T,
        entity_key: EntityKey,
        tx: &ConsensusTx,
        op: Operation,
    ) -> Result<()> {
        let idx = FullOperationIndex {
            block_number: tx.block_number,
            tx_index: tx.index,
            op_index: op.metadata.index,
        };
        let prev_entry = repository::entities::get_latest_entity_history_entry(
            txn,
            entity_key,
            Some(idx.clone()),
        )
        .await?;
        let status = match op.operation {
            OperationData::Delete
                if tx.to_address_hash == well_known::L1_BLOCK_CONTRACT_ADDRESS =>
            {
                EntityStatus::Expired
            }
            OperationData::Delete => EntityStatus::Deleted,
            _ => EntityStatus::Active,
        };
        let owner = match op.operation {
            OperationData::Delete
                if tx.to_address_hash == well_known::L1_BLOCK_CONTRACT_ADDRESS =>
            {
                prev_entry.as_ref().and_then(|v| v.owner)
            }
            _ => Some(tx.from_address_hash),
        };
        let data = match op.operation {
            OperationData::Extend(_) => prev_entry.as_ref().and_then(|v| v.data.clone()),
            _ => op.operation.data().map(ToOwned::to_owned),
        };

        let expires_at_block_number = match op.operation {
            OperationData::Create(_, btl) => Some(tx.block_number + btl),
            OperationData::Update(_, btl) => Some(tx.block_number + btl),
            OperationData::Extend(extend_btl) => prev_entry
                .as_ref()
                .and_then(|v| v.expires_at_block_number.map(|v| v + extend_btl)),
            OperationData::Delete => Some(tx.block_number),
        };

        let expires_at_timestamp = expires_at_block_number.and_then(|v| {
            block_timestamp(
                v,
                &Block {
                    number: tx.block_number,
                    timestamp: tx.block_timestamp,
                    hash: tx.block_hash,
                },
            )
        });
        let entry = EntityHistoryEntry {
            entity_key,
            block_number: tx.block_number,
            block_hash: tx.block_hash,
            transaction_hash: tx.hash,
            tx_index: tx.index,
            op_index: op.metadata.index,
            block_timestamp: tx.block_timestamp,
            owner,
            sender: tx.from_address_hash,
            data,
            prev_data: prev_entry
                .as_ref()
                .and_then(|prev_entry| prev_entry.data.clone()),
            operation: op.operation.clone(),
            status,
            prev_status: prev_entry.as_ref().map(|prev_entry| prev_entry.status),
            expires_at_block_number,
            prev_expires_at_block_number: prev_entry
                .as_ref()
                .and_then(|prev_entry| prev_entry.expires_at_block_number),
            expires_at_timestamp,
            prev_expires_at_timestamp: prev_entry
                .and_then(|prev_entry| prev_entry.expires_at_timestamp),
            btl: op.operation.btl(),
        };
        repository::entities::insert_history_entry(txn, entry.clone()).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(update, idx))]
    async fn handle_update(
        &self,
        txn: &DatabaseTransaction,
        guards: &mut HashMap<EntityKey, Guard>,
        tx: &ConsensusTx,
        update: Update,
        idx: u64,
    ) -> Result<()> {
        if let Entry::Vacant(e) = guards.entry(update.entity_key) {
            e.insert(repository::locks::lock(txn, update.entity_key).await?);
        }
        tracing::info!("Processing Update operation");

        let op = Operation {
            metadata: OperationMetadata {
                entity_key: update.entity_key,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                block_number: tx.block_number,
                tx_index: tx.index,
            },
            operation: OperationData::update(update.data.clone(), update.btl),
        };
        repository::operations::insert_operation(txn, op.clone()).await?;

        if repository::entities::get_oldest_entity_history_entry(
            txn,
            update.entity_key,
            FullOperationIndex {
                block_number: tx.block_number,
                tx_index: tx.index,
                op_index: idx,
            },
        )
        .await?
        .is_some()
        {
            self.reindex_entity_with_ops(txn, update.entity_key).await?;
        } else {
            self.insert_history_entry(txn, update.entity_key, tx, op)
                .await?;
        }

        repository::entities::refresh_entity_based_on_history(txn, update.entity_key).await?;

        self.store_annotations(
            txn,
            update.entity_key,
            tx,
            idx,
            update
                .string_annotations
                .into_iter()
                .map(Into::into)
                .collect(),
            update
                .numeric_annotations
                .into_iter()
                .map(Into::into)
                .collect(),
        )
        .await?;

        Ok(())
    }

    #[instrument(skip_all, fields(delete, idx))]
    async fn handle_delete(
        &self,
        txn: &DatabaseTransaction,
        guards: &mut HashMap<EntityKey, Guard>,
        tx: &ConsensusTx,
        delete: GolemBaseDelete,
        idx: u64,
    ) -> Result<()> {
        if let Entry::Vacant(e) = guards.entry(delete) {
            e.insert(repository::locks::lock(txn, delete).await?);
        }
        tracing::info!("Processing Delete operation");
        let op = Operation {
            metadata: OperationMetadata {
                entity_key: delete,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                block_number: tx.block_number,
                tx_index: tx.index,
            },
            operation: OperationData::delete(),
        };
        repository::operations::insert_operation(txn, op.clone()).await?;

        self.insert_history_entry(txn, delete, tx, op).await?;
        repository::entities::refresh_entity_based_on_history(txn, delete).await?;

        repository::annotations::deactivate_annotations(txn, delete).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(extend, idx))]
    async fn handle_extend(
        &self,
        txn: &DatabaseTransaction,
        guards: &mut HashMap<EntityKey, Guard>,
        tx: &ConsensusTx,
        extend: Extend,
        idx: u64,
    ) -> Result<()> {
        if let Entry::Vacant(e) = guards.entry(extend.entity_key) {
            e.insert(repository::locks::lock(txn, extend.entity_key).await?);
        }
        tracing::info!("Processing Extend operation");
        let op = Operation {
            metadata: OperationMetadata {
                entity_key: extend.entity_key,
                sender: tx.from_address_hash,
                recipient: tx.to_address_hash,
                tx_hash: tx.hash,
                block_hash: tx.block_hash,
                index: idx,
                block_number: tx.block_number,
                tx_index: tx.index,
            },
            operation: OperationData::extend(extend.number_of_blocks),
        };
        repository::operations::insert_operation(txn, op.clone()).await?;

        if repository::entities::get_oldest_entity_history_entry(
            txn,
            extend.entity_key,
            FullOperationIndex {
                block_number: tx.block_number,
                tx_index: tx.index,
                op_index: idx,
            },
        )
        .await?
        .is_some()
        {
            self.reindex_entity_with_ops(txn, extend.entity_key).await?;
        } else {
            self.insert_history_entry(txn, extend.entity_key, tx, op)
                .await?;
        }

        repository::entities::refresh_entity_based_on_history(txn, extend.entity_key).await?;

        Ok(())
    }

    async fn is_latest_update(
        &self,
        txn: &DatabaseTransaction,
        entity_key: EntityKey,
        index: FullOperationIndex,
    ) -> Result<bool> {
        let entity = repository::entities::get_entity(txn, entity_key).await?;

        if let Some(entity) = entity {
            if !matches!(entity.status, EntityStatus::Active) {
                return Ok(false);
            }
        }

        match repository::operations::get_latest_update(txn, entity_key).await? {
            Some(latest_stored_index) => Ok(index >= latest_stored_index),
            None => Ok(true),
        }
    }

    #[instrument(skip_all, fields(log))]
    async fn handle_log(&self, log: LogIndex) -> Result<()> {
        tracing::info!("Processing log");
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
        let guard = repository::locks::lock(&txn, entity_key).await?;
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
            },
            operation: OperationData::delete(),
        };
        repository::operations::insert_operation(&txn, op.clone()).await?;

        self.insert_history_entry(&txn, entity_key, &tx, op).await?;
        repository::entities::refresh_entity_based_on_history(&txn, entity_key).await?;

        repository::annotations::deactivate_annotations(&txn, entity_key).await?;
        guard.unlock(&txn).await?;
        txn.commit().await?;
        OP_COUNTER.inc();

        Ok(())
    }

    #[instrument(skip_all, fields(entity_key))]
    async fn store_annotations(
        &self,
        txn: &DatabaseTransaction,
        entity_key: EntityKey,
        tx: &ConsensusTx,
        op_index: u64,
        string_annotations: Vec<StringAnnotation>,
        numeric_annotations: Vec<NumericAnnotation>,
    ) -> Result<()> {
        let latest_update = self
            .is_latest_update(
                txn,
                entity_key,
                FullOperationIndex {
                    block_number: tx.block_number,
                    tx_index: tx.index,
                    op_index,
                },
            )
            .await?;

        if latest_update {
            repository::annotations::deactivate_annotations(txn, entity_key).await?;
        }

        for annotation in string_annotations {
            repository::annotations::insert_string_annotation(
                txn,
                FullStringAnnotation {
                    entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: op_index,
                    annotation: StringAnnotation {
                        key: annotation.key,
                        value: annotation.value,
                    },
                },
                latest_update,
            )
            .await?;
        }

        for annotation in numeric_annotations {
            repository::annotations::insert_numeric_annotation(
                txn,
                FullNumericAnnotation {
                    entity_key,
                    operation_tx_hash: tx.hash,
                    operation_index: op_index,
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
}
