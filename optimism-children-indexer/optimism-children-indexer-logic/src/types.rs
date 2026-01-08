use alloy_primitives::{Bytes, B256};
use chrono::{DateTime, Utc};

pub use alloy_primitives::{
    Address, BlockHash, BlockNumber, ChainId, TxHash, U256 as CurrencyAmount, U256,
};
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
    pub next_page: Option<PaginationParams>,
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
    pub from: Address,
    pub to: Address,
    pub transaction_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
    pub block_number: BlockNumber,
    pub block_timestamp: Timestamp,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullDeposit<T> {
    pub event: FullEvent<TransactionDepositedEvent<T>>,
    pub execution_tx: Option<ExecutionTransaction>,
    pub chain_id: Option<ChainId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionTransaction {
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    pub block_timestamp: Timestamp,
    pub hash: TxHash,
    pub from: Address,
    pub to: Address,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawalProvenEvent {
    pub withdrawal_hash: B256,
    pub from: Address,
    pub to: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawalFinalizedEvent {
    pub withdrawal_hash: B256,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullWithdrawal {
    pub chain_id: ChainId,
    pub l3_block_number: BlockNumber,
    pub l3_block_hash: BlockHash,
    pub l3_block_timestamp: Timestamp,
    pub l3_tx_hash: TxHash,
    pub nonce: U256,
    pub sender: Address,
    pub target: Address,
    pub value: U256,
    pub gas_limit: U256,
    pub data: Bytes,
    pub withdrawal_hash: B256,
    pub proving_tx: Option<FullEvent<WithdrawalProvenEvent>>,
    pub finalizing_tx: Option<FullEvent<WithdrawalFinalizedEvent>>,
}
