use alloy_primitives::B256;

pub use alloy_primitives::{Address, BlockHash, BlockNumber, TxHash};
pub use alloy_rlp::Bytes;

pub type EntityKey = B256;

#[derive(Clone, Debug)]
pub struct Annotation<T: std::fmt::Debug> {
    pub entity_key: EntityKey,
    pub operation_tx_hash: TxHash,
    pub operation_index: u64,
    pub key: String,
    pub value: T,
}

pub type StringAnnotation = Annotation<String>;
pub type NumericAnnotation = Annotation<u64>;

#[derive(Clone, Debug)]
pub struct Log {
    pub data: Bytes,
    pub index: u64,
    pub first_topic: Option<B256>,
    pub second_topic: Option<B256>,
    pub third_topic: Option<B256>,
    pub fourth_topic: Option<B256>,
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
    pub tx_hash: TxHash,
    pub block_hash: BlockHash,
    pub index: u64,
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
    pub fn btl(self) -> Option<u64> {
        match self {
            Self::Create(_, btl) => Some(btl),
            Self::Update(_, btl) => Some(btl),
            Self::Delete => None,
            Self::Extend(btl) => Some(btl),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub hash: TxHash,
    pub from_address_hash: Address,
    pub to_address_hash: Address,
    pub block_number: BlockNumber,
    pub block_hash: BlockHash,
    pub input: Bytes,
    pub index: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum EntityStatus {
    Active,
    Deleted,
    Expired,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub key: EntityKey,
    pub data: Option<Bytes>,
    pub status: EntityStatus,
    pub created_at_tx_hash: Option<TxHash>,
    pub last_updated_at_tx_hash: TxHash,
    pub expires_at_block_number: BlockNumber,
}
