#![allow(clippy::derive_partial_eq_without_eq)]

use crate::blockscout::optimism_children_indexer::v1;
use anyhow::Result;
use optimism_children_indexer_logic::types::{
    DepositV0, FullEvent, PaginationMetadata, PaginationParams, TransactionDepositedEvent,
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

impl TryFrom<v1::PaginationRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::PaginationRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl From<FullEvent<TransactionDepositedEvent<DepositV0>>> for v1::Deposit {
    fn from(ev: FullEvent<TransactionDepositedEvent<DepositV0>>) -> Self {
        Self {
            init_tx: Some(v1::TxInfo {
                from: ev.metadata.from.to_checksum(None),
                to: ev.metadata.to.to_checksum(None),
                transaction_hash: ev.metadata.transaction_hash.to_string(),
                block_hash: ev.metadata.block_hash.to_string(),
                block_number: ev.metadata.block_number,
            }),
            from: ev.event.from.to_checksum(None),
            to: ev.event.to.to_checksum(None),
            mint: ev.event.deposit.mint.to_string(),
            value: ev.event.deposit.value.to_string(),
            gas_limit: ev.event.deposit.gas_limit.to_string(),
            is_creation: ev.event.deposit.is_creation,
        }
    }
}
