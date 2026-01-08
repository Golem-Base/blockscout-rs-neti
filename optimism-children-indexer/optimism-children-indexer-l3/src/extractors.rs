//! Data extractors for Layer3 chains.
use super::{
    abi::L2ToL1MessagePasser,
    types::{
        Layer3Chains, Layer3Deposit, Layer3IndexerTaskOutputItem, Layer3Withdrawal, Timestamp,
    },
};
use optimism_children_indexer_logic::well_known::{
    ARKIV_HOUSEKEEPING_ADDRESS, OPTIMISM_L3_TO_L2_MESSAGE_PASSER_ADDRESS,
};

use alloy::{
    consensus::BlockHeader,
    network::{ReceiptResponse, TransactionResponse},
    providers::Network,
    sol_types::SolEvent,
};
use anyhow::{Result, anyhow};
use op_alloy::network::Optimism;

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
                    from: deposit_tx.from.into_array().to_vec(),
                    to: deposit_tx
                        .to
                        .into_to()
                        .ok_or(anyhow!("Failed to get 'to' for tx_hash: {}", tx.tx_hash()))?
                        .into_array()
                        .to_vec(),
                    block_number: block.number() as i64,
                    block_hash: block.hash().to_vec(),
                    block_timestamp: Timestamp::from_timestamp_secs(
                        block.header.timestamp().try_into()?,
                    )
                    .ok_or(anyhow!("Failed to convert block timestamp"))?,
                    tx_hash: tx.tx_hash().as_slice().to_vec(),
                    source_hash: deposit_tx.source_hash.as_slice().to_vec(),
                    success: receipt.status(),
                };

                items.push(Layer3IndexerTaskOutputItem::Deposit(deposit));
            }
        }
    }

    Ok(items)
}

/// Extract Optimism withdrawal events from a block.
pub fn extract_withdrawals(
    config: &Layer3Chains::Model,
    block: &<Optimism as Network>::BlockResponse,
    receipts: &Vec<<Optimism as Network>::ReceiptResponse>,
) -> Result<Vec<Layer3IndexerTaskOutputItem>> {
    let block_timestamp = Timestamp::from_timestamp_secs(block.header.timestamp().try_into()?)
        .ok_or(anyhow!("Failed to convert block timestamp"))?;

    let items: Vec<Layer3IndexerTaskOutputItem> = receipts
        .iter()
        .filter(|receipt| {
            // Use bloom filter to look for `MessagePassed` event.
            receipt.inner.inner.logs_bloom().contains_raw_log(
                OPTIMISM_L3_TO_L2_MESSAGE_PASSER_ADDRESS,
                &[L2ToL1MessagePasser::MessagePassed::SIGNATURE_HASH],
            )
        })
        .filter_map(|receipt| {
            // Find and decode first `MessagePassed` event.
            if let Some(message_passed) = receipt
                .inner
                .decoded_log::<L2ToL1MessagePasser::MessagePassed>()
            {
                let withdrawal = Layer3Withdrawal {
                    chain_id: config.chain_id,
                    block_number: block.number() as i64,
                    block_hash: block.hash().to_vec(),
                    block_timestamp,
                    tx_hash: receipt.transaction_hash().to_vec(),
                    nonce: message_passed.nonce,
                    sender: message_passed.sender.to_vec(),
                    target: message_passed.target.to_vec(),
                    value: message_passed.value,
                    gas_limit: message_passed.gasLimit,
                    data: message_passed.data.data.to_vec(),
                    withdrawal_hash: message_passed.withdrawalHash.to_vec(),
                };

                Some(Layer3IndexerTaskOutputItem::Withdrawal(withdrawal))
            } else {
                // `MessagePassed` event not found. Most likely bloom false positive.
                tracing::warn!(
                    block_number = block.number(),
                    block_hash = block.hash().to_string(),
                    tx_hash = receipt.inner.transaction_hash.to_string(),
                    "[{}] No MessagePassed event found in the receipt logs. Bloom filter false positive?",
                    config.chain_name,
                );

                None
            }
        })
        .collect();

    Ok(items)
}
