#![allow(clippy::derive_partial_eq_without_eq)]

use crate::blockscout::optimism_children_indexer::v1;
use anyhow::Result;
use optimism_children_indexer_logic::types::{
    DepositV0, EventMetadata, ExecutionTransaction, FullDeposit, PaginationMetadata,
    PaginationParams,
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
