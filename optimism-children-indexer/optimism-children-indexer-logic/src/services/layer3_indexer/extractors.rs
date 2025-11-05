//! Data extractors for Layer3 chains.
use super::types::{Layer3Chains, Layer3Deposit, Layer3IndexerTaskOutputItem};

use alloy::{
    network::{ReceiptResponse, TransactionResponse},
    primitives::{address, Address},
    providers::Network,
};
use anyhow::{anyhow, Result};
use op_alloy::network::Optimism;

const ARKIV_HOUSEKEEPING_ADDRESS: Address = address!("deaddeaddeaddeaddeaddeaddeaddeaddead0001");

/// Extracts Optimism deposit transactions from a block.
pub fn extract_deposits(
    config: &Layer3Chains::Model,
    block: &<Optimism as Network>::BlockResponse,
    receipts: &Vec<<Optimism as Network>::ReceiptResponse>,
) -> Result<Vec<Layer3IndexerTaskOutputItem>> {
    let mut items = Vec::new();

    if let Some(txs) = block.transactions.as_transactions() {
        for (tx, receipt) in txs.iter().zip(receipts.iter()) {
            if let Some(deposit_tx) = tx.inner.inner.as_deposit() {
                // Ignore Arkiv housekeeping transactions
                if deposit_tx.from == ARKIV_HOUSEKEEPING_ADDRESS {
                    continue;
                }

                // Collect deposit transaction information
                let deposit = Layer3Deposit {
                    chain_id: config.chain_id,
                    block_hash: tx
                        .block_hash
                        .ok_or_else(|| anyhow!("Missing block_hash for tx_hash: {}", tx.tx_hash()))?
                        .as_slice()
                        .to_vec(),
                    tx_hash: tx.tx_hash().as_slice().to_vec(),
                    source_hash: deposit_tx.source_hash.as_slice().to_vec(),
                    status: receipt.status(),
                };

                items.push(Layer3IndexerTaskOutputItem::Deposit(deposit));
            }
        }
    }

    Ok(items)
}
