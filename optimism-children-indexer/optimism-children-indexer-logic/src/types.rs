use alloy_primitives::{Bytes, B256};
use chrono::{DateTime, Utc};

pub use alloy_primitives::{Address, BlockHash, BlockNumber, TxHash, U256 as CurrencyAmount};
use anyhow::{Context, Result};

pub type Timestamp = DateTime<Utc>;

#[derive(Clone, Debug)]
pub struct Log {
    pub data: Bytes,
    pub index: u64,
    pub first_topic: Option<B256>,
    pub second_topic: Option<B256>,
    pub third_topic: Option<B256>,
    pub fourth_topic: Option<B256>,
    pub tx_hash: TxHash,
}

#[derive(Debug, Clone)]
pub struct LogIndex {
    pub transaction_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub hash: TxHash,
    pub from_address_hash: Address,
    pub to_address_hash: Address,
    pub block_number: Option<BlockNumber>,
    pub block_hash: Option<BlockHash>,
    pub block_timestamp: Option<Timestamp>,
    pub input: Bytes,
    pub index: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ConsensusTx {
    pub hash: TxHash,
    pub from_address_hash: Address,
    pub to_address_hash: Address,
    pub block_number: BlockNumber,
    pub block_hash: BlockHash,
    pub block_timestamp: Timestamp,
    pub input: Bytes,
    pub index: u64,
}

#[derive(Debug, Clone)]
pub enum BlockNumberOrHashFilter {
    Number(BlockNumber),
    Hash(BlockHash),
}

impl core::str::FromStr for BlockNumberOrHashFilter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        s.parse::<BlockHash>()
            .context("Parsing as block hash")
            .map(Self::Hash)
            .or_else(|_| {
                s.parse::<BlockNumber>()
                    .map(Self::Number)
                    .context("Parsing as block number")
            })
    }
}

#[derive(Debug, Clone)]
pub struct PaginationMetadata {
    pub pagination: PaginationParams,
    pub total_pages: u64,
    pub total_items: u64,
}

#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub hash: BlockHash,
    pub number: BlockNumber,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub hash: TxHash,
    pub from_address_hash: Address,
    pub to_address_hash: Option<Address>,
    pub status: Option<u8>,
    pub block_number: Option<BlockNumber>,
    pub block_hash: Option<BlockHash>,
    pub block_consensus: Option<bool>,
    pub block_timestamp: Option<Timestamp>,
    pub index: Option<u64>,
    pub cumulative_gas_used: Option<u64>,
    pub gas_price: Option<u64>,
    pub error: Option<String>,
    pub input: Bytes,
    pub value: CurrencyAmount,
    pub created_contract_address_hash: Option<Address>,
    pub r#type: Option<i32>,
    pub l1_transaction_origin: Option<Address>,
    pub l1_block_number: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullEvent<T> {
    pub metadata: EventMetadata,
    pub event: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMetadata {
    pub transaction_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
    pub block_number: BlockNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDepositedEvent<T> {
    pub from: Address,
    pub to: Address,
    pub source_hash: B256,
    pub deposit: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositV0 {
    pub mint: CurrencyAmount,
    pub value: CurrencyAmount,
    pub gas_limit: u64,
    pub is_creation: bool,
    pub calldata: Bytes,
}
