use alloy_primitives::{Address, B256};
use alloy_rlp::{Bytes, RlpDecodable, RlpEncodable};

#[derive(Debug, Clone, RlpEncodable, RlpDecodable)]
pub struct Attribute<T> {
    pub key: Key,
    pub value: T,
}
pub type StringAttribute = Attribute<String>;
pub type NumericAttribute = Attribute<u64>;
pub type Hash = B256;
pub type Key = String;

#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Create {
    pub btl: u64,
    pub content_type: String,
    pub payload: Bytes,
    pub string_attributes: Vec<StringAttribute>,
    pub numeric_attributes: Vec<NumericAttribute>,
}

#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Update {
    pub entity_key: Hash,
    pub content_type: String,
    pub btl: u64,
    pub payload: Bytes,
    pub string_attributes: Vec<StringAttribute>,
    pub numeric_attributes: Vec<NumericAttribute>,
}

pub type Delete = Hash;

#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct Extend {
    pub entity_key: Hash,
    pub number_of_blocks: u64,
}

#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct ChangeOwner {
    pub entity_key: Hash,
    pub new_owner: Address,
}

#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct StorageTransaction {
    pub creates: Vec<Create>,
    pub updates: Vec<Update>,
    pub deletes: Vec<Delete>,
    pub extensions: Vec<Extend>,
    pub change_owners: Vec<ChangeOwner>,
}
