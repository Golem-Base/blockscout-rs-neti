use alloy_primitives::{Address, B256, U256};
use alloy_sol_types::SolValue;
use anyhow::{anyhow, ensure, Result};
use futures::StreamExt;
use lazy_static::lazy_static;
use prometheus::{opts, register_gauge, Gauge};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::Deserialize;
use serde_with::serde_as;
use std::{
    sync::Arc,
    time::{self, Duration},
};
use tokio::time::sleep;
use tracing::{instrument, warn};

use crate::{
    deposit::source_hash,
    types::{
        ConsensusTx, LogIndex, TransactionDepositedEvent, WithdrawalFinalizedEvent,
        WithdrawalProvenEvent,
    },
    well_known::{TRANSACTION_DEPOSITED_SIG, WITHDRAWAL_FINALIZED_SIG, WITHDRAWAL_PROVEN_SIG},
};

mod consensus_tx;
mod deposit;
pub mod pagination;
pub mod repository;
pub mod types;
pub mod well_known;

lazy_static! {
    static ref PENDING_LOGS_GAUGE: Gauge =
        register_gauge!(opts!("pending_logs", "Number of logs to be processed.",)).unwrap();
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

impl Indexer {
    pub fn new(db: Arc<DatabaseConnection>, settings: IndexerSettings) -> Self {
        Self { db, settings }
    }

    #[instrument(skip_all)]
    pub async fn run(self) -> Result<()> {
        loop {
            self.tick().await.inspect_err(|e| {
                tracing::error!(?e, "Failed to index logs, exiting (will be restarted)...")
            })?;
            sleep(self.settings.polling_interval).await;
        }
    }

    pub async fn update_gauges(&self) -> ! {
        loop {
            match repository::blockscout::count_unprocessed_logs(&*self.db).await {
                Ok(v) => PENDING_LOGS_GAUGE.set(v as f64),
                Err(e) => warn!(?e, "Failed to update metrics"),
            }
            sleep(Duration::from_secs(5)).await;
        }
    }

    #[instrument(skip_all)]
    pub async fn tick(&self) -> Result<()> {
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

        let signature_hash = log
            .first_topic
            .ok_or(anyhow!("Log with missing first topic?"))?;
        match signature_hash {
            TRANSACTION_DEPOSITED_SIG => {
                let from = if let Some(second_topic) = log.second_topic {
                    Address::abi_decode_validate(second_topic.as_slice())?
                } else {
                    tracing::warn!("TransactionDeposited event with no second topic?");
                    return Ok(());
                };

                let to = if let Some(third_topic) = log.third_topic {
                    Address::abi_decode_validate(third_topic.as_slice())?
                } else {
                    tracing::warn!("TransactionDeposited event with no third topic?");
                    return Ok(());
                };

                let version: U256 = if let Some(fourth_topic) = log.fourth_topic {
                    fourth_topic.into()
                } else {
                    tracing::warn!("TransactionDeposited event with no fourth topic?");
                    return Ok(());
                };

                ensure!(version == U256::ZERO, "Unsupported deposit version");

                let event = TransactionDepositedEvent {
                    from,
                    to,
                    source_hash: source_hash(tx.block_hash, log.index.try_into()?),
                    deposit: log.data.clone().try_into()?,
                };

                repository::deposits::store_transaction_deposited(
                    &txn,
                    tx.clone(),
                    log.clone(),
                    event,
                )
                .await?;
            }
            WITHDRAWAL_PROVEN_SIG => {
                let withdrawal_hash: B256 = if let Some(second_topic) = log.second_topic {
                    second_topic
                } else {
                    tracing::warn!("WithdrawalProven event with no second topic?");
                    return Ok(());
                };

                let from = if let Some(third_topic) = log.third_topic {
                    Address::abi_decode_validate(third_topic.as_slice())?
                } else {
                    tracing::warn!("WithdrawalProven event with no third topic?");
                    return Ok(());
                };

                let to = if let Some(fourth_topic) = log.fourth_topic {
                    Address::abi_decode_validate(fourth_topic.as_slice())?
                } else {
                    tracing::warn!("WithdrawalProven event with no fourth topic?");
                    return Ok(());
                };

                let event = WithdrawalProvenEvent {
                    withdrawal_hash,
                    from,
                    to,
                };

                repository::withdrawals::store_withdrawal_proven(
                    &txn,
                    tx.clone(),
                    log.clone(),
                    event,
                )
                .await?;
            }
            WITHDRAWAL_FINALIZED_SIG => {
                let withdrawal_hash: B256 = if let Some(second_topic) = log.second_topic {
                    second_topic
                } else {
                    tracing::warn!("WithdrawalFinalized event with no second topic?");
                    return Ok(());
                };

                let success = log.data[31] != 0;

                let event = WithdrawalFinalizedEvent {
                    withdrawal_hash,
                    success,
                };

                repository::withdrawals::store_withdrawal_finalized(
                    &txn,
                    tx.clone(),
                    log.clone(),
                    event,
                )
                .await?;
            }
            _ => {
                tracing::warn!(
                    "Tried processing event with unrecognized signature hash: {signature_hash}"
                );
                return Ok(());
            }
        }

        repository::logs::finish_log_processing(&txn, log.tx_hash, tx.block_hash, log.index)
            .await?;
        txn.commit().await?;

        Ok(())
    }
}
