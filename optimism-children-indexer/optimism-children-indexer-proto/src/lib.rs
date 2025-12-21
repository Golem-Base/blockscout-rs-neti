#![allow(clippy::derive_partial_eq_without_eq)]

use crate::blockscout::optimism_children_indexer::v1;
use anyhow::Result;
use optimism_children_indexer_logic::types::{
    DepositV0, EventMetadata, ExecutionTransaction, FullDeposit, FullEvent, FullWithdrawal,
    PaginationMetadata, PaginationParams, WithdrawalFinalizedEvent, WithdrawalProvenEvent,
};

pub mod blockscout {
    pub mod optimism_children_indexer {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.optimism_children_indexer.v1.rs"
            ));
        }
    }
}

impl From<PaginationMetadata> for v1::Pagination {
    fn from(value: PaginationMetadata) -> Self {
        Self {
            page: value.pagination.page,
            page_size: value.pagination.page_size,
            total_pages: value.total_pages,
            total_items: value.total_items,
        }
    }
}

impl From<PaginationParams> for v1::PaginationNextPage {
    fn from(value: PaginationParams) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
        }
    }
}

impl TryFrom<v1::PaginationRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::PaginationRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl From<EventMetadata> for v1::TxInfo {
    fn from(v: EventMetadata) -> Self {
        Self {
            from: v.from.to_checksum(None),
            to: v.to.to_checksum(None),
            transaction_hash: v.transaction_hash.to_string(),
            block_hash: v.block_hash.to_string(),
            block_number: v.block_number,
            success: true,
        }
    }
}

impl From<ExecutionTransaction> for v1::TxInfo {
    fn from(v: ExecutionTransaction) -> Self {
        Self {
            from: v.from.to_checksum(None),
            to: v.to.to_checksum(None),
            transaction_hash: v.hash.to_string(),
            block_hash: v.block_hash.to_string(),
            block_number: v.block_number,
            success: v.success,
        }
    }
}

impl From<FullDeposit<DepositV0>> for v1::Deposit {
    fn from(d: FullDeposit<DepositV0>) -> Self {
        Self {
            init_tx: Some(d.event.metadata.into()),
            execution_tx: d.execution_tx.map(Into::into),
            from: d.event.event.from.to_checksum(None),
            to: d.event.event.to.to_checksum(None),
            mint: d.event.event.deposit.mint.to_string(),
            value: d.event.event.deposit.value.to_string(),
            gas_limit: d.event.event.deposit.gas_limit.to_string(),
            is_creation: d.event.event.deposit.is_creation,
            destination_chain_id: d.chain_id.map(|v| v.to_string()),
        }
    }
}

impl From<WithdrawalProvenEvent> for v1::WithdrawalProvenEvent {
    fn from(v: WithdrawalProvenEvent) -> Self {
        Self {
            withdrawal_hash: v.withdrawal_hash.to_string(),
            from: v.from.to_string(),
            to: v.to.to_string(),
        }
    }
}

impl From<FullEvent<WithdrawalProvenEvent>> for v1::WithdrawalProving {
    fn from(v: FullEvent<WithdrawalProvenEvent>) -> Self {
        Self {
            metadata: Some(v.metadata.into()),
            event: Some(v.event.into()),
        }
    }
}

impl From<WithdrawalFinalizedEvent> for v1::WithdrawalFinalizedEvent {
    fn from(v: WithdrawalFinalizedEvent) -> Self {
        Self {
            withdrawal_hash: v.withdrawal_hash.to_string(),
            success: v.success,
        }
    }
}

impl From<FullEvent<WithdrawalFinalizedEvent>> for v1::WithdrawalFinalizing {
    fn from(v: FullEvent<WithdrawalFinalizedEvent>) -> Self {
        Self {
            metadata: Some(v.metadata.into()),
            event: Some(v.event.into()),
        }
    }
}

impl TryFrom<FullWithdrawal> for v1::Withdrawal {
    type Error = anyhow::Error;

    fn try_from(v: FullWithdrawal) -> Result<Self> {
        Ok(Self {
            chain_id: v.chain_id.to_string(),
            l3_block_number: v.l3_block_number,
            l3_block_hash: v.l3_block_hash.to_string(),
            l3_tx_hash: v.l3_tx_hash.to_string(),
            nonce: v.nonce.to_string(),
            sender: v.sender.to_string(),
            target: v.target.to_string(),
            value: v.value.to_string(),
            gas_limit: v.gas_limit.to_string(),
            data: v.data.to_string(),
            withdrawal_hash: v.withdrawal_hash.to_string(),
            proving_tx: v.proving_tx.map(Into::into),
            finalizing_tx: v.finalizing_tx.map(Into::into),
        })
    }
}
