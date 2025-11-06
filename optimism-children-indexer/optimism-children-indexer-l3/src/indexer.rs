//! Layer3 Indexer.
//!
//! This module manages concurrent indexing tasks for multiple Layer3 chains.
//! It automatically spawns, monitors, and restarts indexing tasks based on the current
//! state of chains in the database.
use super::{
    Layer3IndexerTask,
    types::{Layer3IndexerTaskOutput, Layer3IndexerTaskOutputItem, optimism_children_l3_deposits},
};

use anyhow::Result;
use chrono::Utc;
use optimism_children_indexer_entity::optimism_children_l3_chains::{self, Model as Layer3Chain};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    task::{AbortHandle, JoinSet},
    time::{Duration, interval},
};

///  TODO: Consider making this configurable via Settings
/// Time between database chains refreshes
const REFRESH_CHAINS_INTERVAL: Duration = Duration::from_secs(15);
/// Restart delay for fully synced chains
const RESTART_DELAY_SYNCED: Duration = Duration::from_mins(5);
/// Restart delay for chains falling behind
const RESTART_DELAY_BEHIND: Duration = Duration::from_secs(5);
/// Restart delay for failing chains
const RESTART_DELAY_FAILING: Duration = Duration::from_secs(90);

/// Main task for Layer3 Indexer.
pub struct Layer3Indexer {
    db: Arc<DatabaseConnection>,
    chains: HashMap<i64, Layer3Chain>,
    tasks: JoinSet<(i64, Result<Layer3IndexerTaskOutput>)>,
    abort_handles: HashMap<i64, AbortHandle>,
}

impl Layer3Indexer {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            chains: HashMap::new(),
            tasks: JoinSet::new(),
            abort_handles: HashMap::new(),
        }
    }

    /// Fetch enabled chains from database
    async fn refresh_chains(&mut self) -> Result<()> {
        self.chains = optimism_children_l3_chains::Entity::find()
            .all(&*self.db)
            .await?
            .into_iter()
            .filter(|config| config.enabled)
            .map(|config| (config.chain_id, config))
            .collect();

        Ok(())
    }

    /// Synchronize running tasks with current chain list
    /// Spawns tasks for new chains, cancels tasks for removed/disabled chains
    fn sync_tasks(&mut self) {
        // Collect chains to spawn
        let to_spawn: Vec<_> = self
            .chains
            .iter()
            .filter(|&(&chain_id, _)| !self.abort_handles.contains_key(&chain_id))
            .map(|(_, chain)| chain.clone())
            .collect();

        // Spawn new tasks
        for config in to_spawn {
            tracing::debug!(
                chain_id = config.chain_id,
                "[{}] Spawning indexer task.",
                config.chain_name
            );

            self.spawn_task(config, Duration::ZERO);
        }

        // Cancel tasks for chains that were removed or disabled
        self.abort_handles.retain(|&chain_id, handle| {
            if !self.chains.contains_key(&chain_id) {
                tracing::debug!(
                    chain_id = chain_id,
                    "Cancelling task for removed/disabled chain."
                );
                handle.abort();
                false
            } else {
                true
            }
        });
    }

    /// Spawn a new indexer task with delay
    fn spawn_task(&mut self, config: Layer3Chain, delay: Duration) {
        let chain_id = config.chain_id;

        let handle = self.tasks.spawn(async move {
            let chain_id = config.chain_id;
            let task = Layer3IndexerTask::new(config);
            let result = task.run_with_delay(delay).await;
            (chain_id, result)
        });

        self.abort_handles.insert(chain_id, handle);
    }

    /// Attempt to respawn a task after completion or failure
    /// Only respawns if the chain still exists and is enabled
    fn try_respawn(&mut self, chain_id: i64, succeeded: bool) {
        if let Some(chain) = self.chains.get(&chain_id).cloned() {
            let delay = self.calculate_restart_delay(&chain, succeeded);

            tracing::debug!(
                chain_id = chain_id,
                delay_secs = delay.as_secs(),
                succeeded = succeeded,
                "[{}] Scheduling task restart.",
                chain.chain_name
            );

            self.spawn_task(chain, delay);
        }
    }

    /// Calculate appropriate restart delay
    fn calculate_restart_delay(&self, chain: &Layer3Chain, succeeded: bool) -> Duration {
        if !succeeded {
            return RESTART_DELAY_FAILING;
        }

        match (chain.l3_last_indexed_block, chain.l3_latest_block) {
            (last, Some(latest)) if last >= latest => {
                // Fully synced - check less frequently
                RESTART_DELAY_SYNCED
            }
            (_, Some(_)) => {
                // Behind - catch up
                RESTART_DELAY_BEHIND
            }
            _ => {
                // No block info - use default
                RESTART_DELAY_BEHIND
            }
        }
    }

    /// Handle task completion
    async fn handle_task_completion(
        &mut self,
        chain_id: i64,
        result: Result<Layer3IndexerTaskOutput>,
    ) {
        // Always remove from active tasks first
        self.abort_handles.remove(&chain_id);

        let chain_name = self
            .chains
            .get(&chain_id)
            .map(|config| config.chain_name.as_str())
            .unwrap_or("unknown");

        match result {
            Ok(output) => {
                // Handle task result
                let (config, items) = output;

                // Store indexed items
                if let Err(err) = self.store_indexed_items(items).await {
                    tracing::error!(err = ?err, "Failed to store indexed items in database.");
                } else {
                    // Update chain config
                    self.update_chain_state(config.clone())
                        .await
                        .expect("Failed to update chain state.");
                    self.chains.insert(chain_id, config);
                }

                // Schedule respawn
                self.try_respawn(chain_id, true);
            }
            Err(e) => {
                tracing::error!(
                    chain_id = chain_id,
                    error = %e,
                    "[{}] Task failed.", chain_name
                );
                self.try_respawn(chain_id, false);
            }
        }
    }

    /// Update chain state
    async fn update_chain_state(&mut self, config: Layer3Chain) -> Result<()> {
        let mut model: optimism_children_l3_chains::ActiveModel = config.clone().into();

        // Update block numbers
        model.l3_last_indexed_block = Set(config.l3_last_indexed_block);
        model.l3_latest_block = Set(config.l3_latest_block);
        model.l3_latest_block_updated_at = Set(Some(Utc::now().naive_utc()));
        model.updated_at = Set(Utc::now().naive_utc());
        let updated = model.update(&*self.db).await?;

        tracing::debug!(
            chain_id = config.chain_id,
            last_indexed = ?updated.l3_last_indexed_block,
            latest = ?updated.l3_latest_block,
            "[{}] Updated chain state.",
            updated.chain_name
        );

        Ok(())
    }

    /// Stores indexed items in database
    async fn store_indexed_items(&self, items: Vec<Layer3IndexerTaskOutputItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        // Separate items by type
        let mut deposits: Vec<optimism_children_l3_deposits::ActiveModel> = Vec::new();

        for item in items {
            match item {
                Layer3IndexerTaskOutputItem::Deposit(deposit) => {
                    deposits.push(deposit.into());
                }
            }
        }

        // Store deposits
        optimism_children_l3_deposits::Entity::insert_many(deposits)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut check_interval = interval(REFRESH_CHAINS_INTERVAL);

        // Initial load
        self.refresh_chains().await?;
        self.sync_tasks();

        loop {
            tokio::select! {
                // Handle tasks
                Some(result) = self.tasks.join_next() => {
                    match result {
                        Ok((chain_id, task_result)) => {
                            self.handle_task_completion(chain_id, task_result).await;
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Task panicked");
                        }
                    }
                }

                // Periodic chain refresh and sync
                _ = check_interval.tick() => {
                        if let Err(err) = self.refresh_chains().await {
                            tracing::error!(error = %err, "Failed to refresh chains from database");
                            continue;
                        }

                        self.sync_tasks();
                    }
            }
        }
    }
}
