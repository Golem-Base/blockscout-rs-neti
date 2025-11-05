//! Layer3 Indexer Task.
//!
//! This module implements individual indexing tasks for Layer3 chains. Each task is responsible
//! for connecting to a chain's RPC endpoint, fetching blocks and receipts, and
//! extracting relevant data using extractors.
use super::{
    extractors::extract_deposits,
    types::{Layer3Chains, Layer3IndexerTaskOutput, Layer3IndexerTaskOutputItem},
};

use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    providers::{Identity, Provider, ProviderBuilder},
};
use anyhow::{anyhow, Context, Result};
use op_alloy::network::Optimism;
use tokio::time::{sleep, Duration};

pub struct Layer3IndexerTask {
    config: Layer3Chains::Model,
}

/// A single indexing task for a Layer3 chain.
impl Layer3IndexerTask {
    /// Creates a new indexer task for the given chain configuration.
    pub fn new(config: Layer3Chains::Model) -> Self {
        Self { config }
    }

    /// Runs the indexing task after waiting for the specified delay.
    pub async fn run_with_delay(&self, delay: Duration) -> Result<Layer3IndexerTaskOutput> {
        tracing::debug!(
            "[{}] Sleeping for {} second(s) before indexing",
            self.config.chain_name,
            delay.as_secs(),
        );

        sleep(delay).await;
        self.run().await
    }

    /// Executes the main indexing logic for this chain.
    pub async fn run(&self) -> Result<Layer3IndexerTaskOutput> {
        tracing::debug!("[{}] Starting indexing", self.config.chain_name);

        let mut config = self.config.clone();

        // Connect to RPC
        let provider = ProviderBuilder::<Identity, Identity, Optimism>::default()
            .connect(&self.config.l3_rpc_url)
            .await?;

        // Fetch latest block number
        let latest_block_number = self.fetch_latest_block_number(&provider).await?;
        config.l3_latest_block = Some(latest_block_number as i64);

        // Calculate block range to index
        let block_range = self.calculate_block_range(latest_block_number);

        // Start indexing
        if let Some((from_block, to_block)) = block_range {
            tracing::debug!(
                "[{}] Indexing block range: {} to {}",
                self.config.chain_name,
                from_block,
                to_block
            );

            // Run indexing for a block range
            let items = self
                .index_block_range(&provider, from_block, to_block)
                .await?;

            // Update last indexed block
            config.l3_last_indexed_block = Some(to_block as i64);

            tracing::debug!(
                "[{}] Finished indexing. Indexed {} items",
                self.config.chain_name,
                items.len()
            );

            Ok((config, items))
        } else {
            // No need blocks to index
            tracing::debug!(
                "[{}] No new blocks to index (last_indexed: {}, latest: {})",
                self.config.chain_name,
                config.l3_last_indexed_block.unwrap_or(0),
                latest_block_number
            );

            Ok((config, vec![]))
        }
    }

    /// Fetches the latest block number from the chain's RPC endpoint.
    async fn fetch_latest_block_number<P>(&self, provider: &P) -> Result<u64>
    where
        P: Provider<Optimism>,
    {
        let latest_block_number = provider
            .get_block_number()
            .await
            .context("Failed to fetch latest block number")?;

        tracing::debug!(
            "[{}] Latest block number from RPC: {}",
            self.config.chain_name,
            latest_block_number
        );

        Ok(latest_block_number)
    }

    /// Calculates the block range to index based on the current state.
    fn calculate_block_range(&self, latest_block: u64) -> Option<(u64, u64)> {
        let last_indexed = self.config.l3_last_indexed_block.unwrap_or(0) as u64 + 1;

        if last_indexed >= latest_block {
            return None;
        }

        let batch_size = self.config.l3_rpc_batch_size.unwrap_or(2000) as u64;
        let from_block = last_indexed;
        let to_block = std::cmp::min(latest_block, last_indexed + batch_size);

        Some((from_block, to_block))
    }

    /// Indexes the specified block range by fetching blocks and receipts.
    async fn index_block_range<P>(
        &self,
        provider: &P,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Layer3IndexerTaskOutputItem>>
    where
        P: Provider<Optimism>,
    {
        let mut items = Vec::new();

        for block_no in from_block..to_block + 1 {
            // Fetch block with tx data
            let block = provider
                .get_block(BlockId::Number(BlockNumberOrTag::Number(block_no)))
                .full()
                .await?
                .ok_or_else(|| anyhow!("Error fetching block number {}", block_no))?;

            // Fetch block receipts
            let receipts = provider
                .get_block_receipts(BlockId::Number(BlockNumberOrTag::Number(block_no)))
                .await?
                .ok_or_else(|| anyhow!("Error fetching receipts for block number {}", block_no))?;

            // Run extractors
            items.append(&mut extract_deposits(&self.config, &block, &receipts)?);
        }

        tracing::debug!(
            "[{}] Fetched blocks {} - {}",
            self.config.chain_name,
            from_block,
            to_block
        );

        Ok(items)
    }
}
