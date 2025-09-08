use alloy_primitives::B256;
use chrono::{DateTime, Utc};

pub use alloy_primitives::{Address, BlockHash, BlockNumber, TxHash, U256 as CurrencyAmount};
pub use alloy_rlp::Bytes;
use anyhow::{Context, Result};

pub type Timestamp = DateTime<Utc>;

pub type EntityKey = B256;

#[derive(Clone, Debug)]
pub struct FullAnnotation<T: std::fmt::Debug> {
    pub entity_key: EntityKey,
    pub operation_tx_hash: TxHash,
    pub operation_index: u64,
    pub annotation: Annotation<T>,
}

#[derive(Clone, Debug)]
pub struct AnnotationWithRelations<T: std::fmt::Debug> {
    pub annotation: Annotation<T>,
    pub related_entities: u64,
}

#[derive(Clone, Debug)]
pub struct Annotation<T: std::fmt::Debug> {
    pub key: String,
    pub value: T,
}

impl<T: std::fmt::Debug> From<FullAnnotation<T>> for Annotation<T> {
    fn from(v: FullAnnotation<T>) -> Self {
        v.annotation
    }
}

pub type StringAnnotation = Annotation<String>;
pub type NumericAnnotation = Annotation<u64>;

pub type FullStringAnnotation = FullAnnotation<String>;
pub type FullNumericAnnotation = FullAnnotation<u64>;

pub type StringAnnotationWithRelations = AnnotationWithRelations<String>;
pub type NumericAnnotationWithRelations = AnnotationWithRelations<u64>;

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

#[derive(Clone, Debug)]
pub struct Operation {
    pub metadata: OperationMetadata,
    pub operation: OperationData,
}

#[derive(Clone, Debug)]
pub struct OperationMetadata {
    pub entity_key: EntityKey,
    pub sender: Address,
    pub recipient: Address,
    pub tx_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
}

#[derive(Clone, Debug)]
pub struct OperationView {
    pub op: Operation,
    pub block_number: BlockNumber,
}

#[derive(Clone, Debug)]
pub enum OperationData {
    Create(Bytes, BlockNumber),
    Update(Bytes, BlockNumber),
    Delete,
    Extend(BlockNumber),
}

impl OperationData {
    pub fn create(data: Bytes, btl: BlockNumber) -> Self {
        Self::Create(data, btl)
    }
    pub fn update(data: Bytes, btl: BlockNumber) -> Self {
        Self::Update(data, btl)
    }
    pub fn delete() -> Self {
        Self::Delete
    }
    pub fn extend(btl: BlockNumber) -> Self {
        Self::Extend(btl)
    }
    pub fn data(&self) -> Option<&Bytes> {
        match self {
            Self::Create(data, _) => Some(data),
            Self::Update(data, _) => Some(data),
            Self::Delete => None,
            Self::Extend(_) => None,
        }
    }
    pub fn btl(&self) -> Option<u64> {
        match self {
            Self::Create(_, btl) => Some(*btl),
            Self::Update(_, btl) => Some(*btl),
            Self::Delete => None,
            Self::Extend(btl) => Some(*btl),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub hash: TxHash,
    pub from_address_hash: Address,
    pub to_address_hash: Address,
    pub block_number: Option<BlockNumber>,
    pub block_hash: Option<BlockHash>,
    pub input: Bytes,
    pub index: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityStatus {
    Active,
    Deleted,
    Expired,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub key: EntityKey,
    pub data: Option<Bytes>,
    pub owner: Option<Address>,
    pub status: EntityStatus,
    pub created_at_tx_hash: Option<TxHash>,
    pub last_updated_at_tx_hash: TxHash,
    pub expires_at_block_number: BlockNumber,
}

#[derive(Debug, Clone)]
pub struct EntityWithExpTimestamp {
    pub key: EntityKey,
    pub data: Option<Bytes>,
    pub owner: Option<Address>,
    pub status: EntityStatus,
    pub created_at_tx_hash: Option<TxHash>,
    pub last_updated_at_tx_hash: TxHash,
    pub expires_at_block_number: BlockNumber,
    pub expires_at_timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct FullEntity {
    pub key: EntityKey,
    pub data: Option<Bytes>,
    pub status: EntityStatus,

    pub created_at_tx_hash: Option<TxHash>,
    pub created_at_operation_index: Option<u64>,
    pub created_at_block_number: Option<BlockNumber>,
    pub created_at_timestamp: Option<Timestamp>,

    pub updated_at_tx_hash: TxHash,
    pub updated_at_operation_index: u64,
    pub updated_at_block_number: BlockNumber,
    pub updated_at_timestamp: Timestamp,

    pub expires_at_block_number: BlockNumber,
    pub expires_at_timestamp: Timestamp,

    pub owner: Option<Address>,
    pub gas_used: CurrencyAmount,
    pub fees_paid: CurrencyAmount,
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
pub struct ListOperationsFilter {
    pub pagination: PaginationParams,
    pub operation_type: Option<OperationData>,
    pub operations_filter: OperationsFilter,
}

#[derive(Debug, Clone, Default)]
pub struct OperationsFilter {
    pub entity_key: Option<EntityKey>,
    pub sender: Option<Address>,
    pub block_number_or_hash: Option<BlockNumberOrHashFilter>,
    pub transaction_hash: Option<TxHash>,
}

#[derive(Debug, Clone, Default)]
pub struct OperationsCount {
    pub create_count: u64,
    pub update_count: u64,
    pub delete_count: u64,
    pub extend_count: u64,
}

#[derive(Debug, Clone)]
pub struct PaginationMetadata {
    pub pagination: PaginationParams,
    pub total_pages: u64,
    pub total_items: u64,
}

#[derive(Debug, Clone)]
pub struct EntityHistoryFilter {
    pub entity_key: EntityKey,
    pub pagination: PaginationParams,
}

#[derive(Debug, Clone)]
pub struct EntitiesFilter {
    pub status: Option<EntityStatus>,
    pub string_annotation: Option<StringAnnotation>,
    pub numeric_annotation: Option<NumericAnnotation>,
}

#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone)]
pub struct ListEntitiesFilter {
    pub pagination: PaginationParams,
    pub entities_filter: EntitiesFilter,
}

#[derive(Debug, Clone)]
pub struct OperationFilter {
    pub tx_hash: TxHash,
    pub op_index: u64,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub hash: BlockHash,
    pub number: BlockNumber,
    pub timestamp: Timestamp,
}

pub struct AddressFilter {
    pub address: Address,
}

#[derive(Debug, Clone)]
pub struct AddressEntitiesCount {
    pub total_entities: u64,
    pub size_of_active_entities: u64,
    pub active_entities: u64,
}

#[derive(Debug, Clone)]
pub struct AddressTxsCount {
    pub total_transactions: u64,
    pub failed_transactions: u64,
}

#[derive(Debug, Clone)]
pub struct BiggestSpenders {
    pub rank: u64,
    pub address: Address,
    pub total_fees: CurrencyAmount,
}

#[derive(Debug, Clone)]
pub struct EntityHistoryEntry {
    pub entity_key: EntityKey,
    pub block_number: BlockNumber,
    pub block_hash: BlockHash,
    pub transaction_hash: TxHash,
    pub tx_index: u64,
    pub op_index: u64,
    pub block_timestamp: Timestamp,
    pub sender: Address,
    pub data: Option<Bytes>,
    pub prev_data: Option<Bytes>,
    pub operation: OperationData,
    pub status: EntityStatus,
    pub prev_status: Option<EntityStatus>,
    pub expires_at_block_number: BlockNumber,
    pub prev_expires_at_block_number: Option<BlockNumber>,
    pub expires_at_timestamp: Timestamp,
    pub prev_expires_at_timestamp: Option<Timestamp>,
    pub btl: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct BlockEntitiesCount {
    pub create_count: u64,
    pub update_count: u64,
    pub expire_count: u64,
    pub delete_count: u64,
    pub extend_count: u64,
}

#[derive(Debug, Clone)]
pub struct BlockStorageUsage {
    pub block_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct AddressByEntitiesOwned {
    pub address: Address,
    pub entities_count: i64,
}

#[derive(Debug, Clone)]
pub struct EntityDataSize {
    pub entity_key: EntityKey,
    pub data_size: u64,
}

#[derive(Debug, Clone)]
pub struct AddressByEntitiesCreated {
    pub rank: u64,
    pub address: Address,
    pub entities_created_count: u64,
}
