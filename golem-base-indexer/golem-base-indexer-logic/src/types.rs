use alloy_primitives::B256;
use chrono::{DateTime, Utc};

pub use alloy_primitives::{Address, BlockHash, BlockNumber, TxHash, U256 as CurrencyAmount};
pub use alloy_rlp::Bytes;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub type Timestamp = DateTime<Utc>;

pub type EntityKey = B256;

#[derive(Clone, Debug)]
pub struct FullAttribute<T: std::fmt::Debug> {
    pub entity_key: EntityKey,
    pub operation_tx_hash: TxHash,
    pub operation_index: u64,
    pub attribute: Attribute<T>,
}

#[derive(Clone, Debug)]
pub struct AttributeWithRelations<T: std::fmt::Debug> {
    pub attribute: Attribute<T>,
    pub related_entities: u64,
}

#[derive(Clone, Debug)]
pub struct Attribute<T: std::fmt::Debug> {
    pub key: String,
    pub value: T,
}

impl<T: std::fmt::Debug> From<FullAttribute<T>> for Attribute<T> {
    fn from(v: FullAttribute<T>) -> Self {
        v.attribute
    }
}

pub type StringAttribute = Attribute<String>;
pub type NumericAttribute = Attribute<u64>;

pub type FullStringAttribute = FullAttribute<String>;
pub type FullNumericAttribute = FullAttribute<u64>;

pub type StringAttributeWithRelations = AttributeWithRelations<String>;
pub type NumericAttributeWithRelations = AttributeWithRelations<u64>;

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
    pub block_number: BlockNumber,
    pub tx_index: u64,
    pub cost: Option<CurrencyAmount>,
}

#[derive(Clone, Debug)]
pub struct OperationView {
    pub op: Operation,
    pub block_timestamp: Timestamp,
    pub expires_at_timestamp: Option<Timestamp>,
    pub expires_at_timestamp_sec: Option<u64>,
}

#[derive(Clone, Debug)]
pub enum OperationData {
    Create(Bytes, BlockNumber, String),
    Update(Bytes, BlockNumber, String),
    Delete,
    Extend(BlockNumber),
    ChangeOwner(Address),
}

impl OperationData {
    pub fn create(data: Bytes, btl: BlockNumber, content_type: &str) -> Self {
        Self::Create(data, btl, content_type.to_string())
    }
    pub fn update(data: Bytes, btl: BlockNumber, content_type: &str) -> Self {
        Self::Update(data, btl, content_type.to_string())
    }
    pub fn delete() -> Self {
        Self::Delete
    }
    pub fn extend(btl: BlockNumber) -> Self {
        Self::Extend(btl)
    }
    pub fn data(&self) -> Option<&Bytes> {
        match self {
            Self::Create(data, _, _) => Some(data),
            Self::Update(data, _, _) => Some(data),
            Self::Delete => None,
            Self::Extend(_) => None,
            Self::ChangeOwner(_) => None,
        }
    }
    pub fn btl(&self) -> Option<u64> {
        match self {
            Self::Create(_, btl, _) => Some(*btl),
            Self::Update(_, btl, _) => Some(*btl),
            Self::Delete => None,
            Self::Extend(btl) => Some(*btl),
            Self::ChangeOwner(_) => None,
        }
    }
    pub fn new_owner(&self) -> Option<Address> {
        if let Self::ChangeOwner(owner) = self {
            Some(*owner)
        } else {
            None
        }
    }

    pub fn content_type(&self) -> Option<String> {
        match self {
            Self::Create(_, _, content_type) => Some(content_type.clone()),
            Self::Update(_, _, content_type) => Some(content_type.clone()),
            _ => None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityStatus {
    Active,
    Deleted,
    Expired,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub key: EntityKey,
    pub content_type: Option<String>,
    pub data: Option<Bytes>,
    pub owner: Option<Address>,
    pub status: EntityStatus,
    pub created_at_tx_hash: Option<TxHash>,
    pub last_updated_at_tx_hash: TxHash,
    pub expires_at_block_number: Option<BlockNumber>,
    pub cost: CurrencyAmount,
}

#[derive(Debug, Clone)]
pub struct EntityWithExpTimestamp {
    pub key: EntityKey,
    pub content_type: Option<String>,
    pub data: Option<Bytes>,
    pub owner: Option<Address>,
    pub status: EntityStatus,
    pub created_at_tx_hash: Option<TxHash>,
    pub last_updated_at_tx_hash: TxHash,
    pub expires_at_block_number: Option<BlockNumber>,
    pub expires_at_timestamp: Option<Timestamp>,
    pub expires_at_timestamp_sec: Option<u64>,
    pub cost: CurrencyAmount,
}

#[derive(Debug, Clone)]
pub struct FullEntity {
    pub key: EntityKey,
    pub content_type: Option<String>,
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

    pub expires_at_block_number: Option<BlockNumber>,
    pub expires_at_timestamp: Option<Timestamp>,
    pub expires_at_timestamp_sec: Option<u64>,

    pub owner: Option<Address>,
    pub creator: Option<Address>,
    pub cost: CurrencyAmount,
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
    pub operation_type: Option<OperationType>,
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
    pub changeowner_count: u64,
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
    pub string_attribute: Option<StringAttribute>,
    pub numeric_attribute: Option<NumericAttribute>,
    pub owner: Option<Address>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Extend,
    ChangeOwner,
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
    pub created_entities: u64,
    pub owned_entities: u64,
    pub size_of_active_entities: u64,
    pub active_entities: u64,
}

#[derive(Debug, Clone)]
pub struct AddressTxsCount {
    pub total_transactions: u64,
    pub failed_transactions: u64,
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
    pub owner: Option<Address>,
    pub prev_owner: Option<Address>,
    pub sender: Address,
    pub data: Option<Bytes>,
    pub prev_data: Option<Bytes>,
    pub operation: OperationType,
    pub status: EntityStatus,
    pub prev_status: Option<EntityStatus>,
    pub expires_at_block_number: Option<BlockNumber>,
    pub prev_expires_at_block_number: Option<BlockNumber>,
    pub expires_at_timestamp: Option<Timestamp>,
    pub expires_at_timestamp_sec: Option<u64>,
    pub prev_expires_at_timestamp: Option<Timestamp>,
    pub prev_expires_at_timestamp_sec: Option<u64>,
    pub btl: Option<u64>,
    pub content_type: Option<String>,
    pub prev_content_type: Option<String>,
    pub cost: Option<CurrencyAmount>,
    pub total_cost: Option<CurrencyAmount>,
}

#[derive(Debug, Clone)]
pub struct BlockEntitiesCount {
    pub create_count: u64,
    pub update_count: u64,
    pub expire_count: u64,
    pub delete_count: u64,
    pub extend_count: u64,
    pub changeowner_count: u64,
}

#[derive(Debug, Clone)]
pub struct BlockStorageUsage {
    pub block_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct AddressActivity {
    pub first_seen_timestamp: Option<DateTime<Utc>>,
    pub last_seen_timestamp: Option<DateTime<Utc>>,
    pub first_seen_block: Option<u64>,
    pub last_seen_block: Option<u64>,
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

#[derive(Debug, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct FullOperationIndex {
    pub block_number: BlockNumber,
    pub tx_index: u64,
    pub op_index: u64,
}

#[derive(Debug, Clone)]
pub struct AddressLeaderboardRanks {
    pub biggest_spenders: u64,
    pub entities_created: u64,
    pub entities_owned: u64,
    pub data_owned: u64,
    pub top_accounts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPoint {
    pub date: String,
    pub date_to: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartInfo {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTransactionPoint {
    pub block_number: u64,
    pub tx_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockOperationPoint {
    pub block_number: u64,
    pub create_count: u64,
    pub update_count: u64,
    pub delete_count: u64,
    pub extend_count: u64,
    pub changeowner_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockGasUsageLimitPoint {
    pub block_number: u64,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub gas_usage_percentage: f64,
}

// Leaderboards
#[derive(Debug, Clone)]
pub struct LeaderboardBiggestSpendersItem {
    pub rank: u64,
    pub address: Address,
    pub total_fees: CurrencyAmount,
}

#[derive(Debug, Clone)]
pub struct LeaderboardEntitiesCreatedItem {
    pub rank: u64,
    pub address: Address,
    pub entities_created_count: u64,
}

#[derive(Debug, Clone)]
pub struct LeaderboardEntitiesOwnedItem {
    pub rank: u64,
    pub address: Address,
    pub entities_count: u64,
}

#[derive(Debug, Clone)]
pub struct LeaderboardTopAccountsItem {
    pub rank: u64,
    pub address: Address,
    pub balance: CurrencyAmount,
    pub tx_count: u64,
}

#[derive(Debug, Clone)]
pub struct LeaderboardDataOwnedItem {
    pub rank: u64,
    pub address: Address,
    pub data_size: u64,
}

#[derive(Debug, Clone)]
pub struct LeaderboardLargestEntitiesItem {
    pub rank: u64,
    pub entity_key: EntityKey,
    pub data_size: u64,
}

#[derive(Debug, Clone)]
pub struct LeaderboardEffectivelyLargestEntitiesItem {
    pub rank: u64,
    pub entity_key: EntityKey,
    pub data_size: u64,
    pub lifespan: BlockNumber,
}

#[derive(Debug, Clone)]
pub struct LogIndex {
    pub transaction_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
}

#[derive(Debug, Clone)]
pub struct LogEventIndex {
    pub transaction_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
    pub op_index: u64,
    pub signature_hash: B256,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct EntityDataHistogram {
    pub bucket: u64,
    pub bin_start: u64,
    pub bin_end: u64,
    pub count: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ConsensusBlockInfo {
    pub block_number: u64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ConsensusBlocksInfo {
    pub latest: ConsensusBlockInfo,
    pub safe: ConsensusBlockInfo,
    pub finalized: ConsensusBlockInfo,
}

#[derive(Clone, Debug, Default)]
pub struct ConsensusGasInfo {
    pub gas_used: u64,
    pub gas_price: u64,
    pub transaction_fee: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ConsensusInfo {
    pub blocks: ConsensusBlocksInfo,
    pub gas: ConsensusGasInfo,
}

#[derive(Clone, Debug, Default)]
pub struct EntitiesAverages {
    pub average_entitiy_size: u64,
    pub average_entity_btl: u64,
}
