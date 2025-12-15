pub use alloy::primitives::U256;
use anyhow::Result;
pub use optimism_children_indexer_entity::{
    optimism_children_l3_chains as Layer3Chains, optimism_children_l3_deposits,
    optimism_children_l3_withdrawals,
};
use sea_orm::{Set, entity::prelude::*};
use std::str::FromStr;

/// Type used for chain IDs
pub type ChainId = i64;

/// Type returned from indexer task to indexer on successful pass
pub type Layer3IndexerTaskOutput = (Layer3Chains::Model, Vec<Layer3IndexerTaskOutputItem>);

/// Item type returned from indexer task to indexer on sucessful pass
#[derive(Debug)]
pub enum Layer3IndexerTaskOutputItem {
    Deposit(Layer3Deposit),
    Withdrawal(Layer3Withdrawal),
}

/// Deposit transaction (L2 -> L3)
#[derive(Debug, PartialEq, Eq)]
pub struct Layer3Deposit {
    pub chain_id: i64,
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub block_number: i64,
    pub block_hash: Vec<u8>,
    pub tx_hash: Vec<u8>,
    pub source_hash: Vec<u8>,
    pub success: bool,
}

impl From<Layer3Deposit> for optimism_children_l3_deposits::ActiveModel {
    fn from(v: Layer3Deposit) -> Self {
        Self {
            id: Default::default(),
            chain_id: Set(v.chain_id),
            from: Set(v.from),
            to: Set(v.to),
            block_number: Set(v.block_number),
            block_hash: Set(v.block_hash),
            tx_hash: Set(v.tx_hash),
            source_hash: Set(v.source_hash),
            success: Set(v.success),
            inserted_at: Default::default(),
        }
    }
}

/// Withdrawal event (L3 -> L2)
#[derive(Debug, PartialEq, Eq)]
pub struct Layer3Withdrawal {
    pub chain_id: i64,
    pub block_number: i64,
    pub block_hash: Vec<u8>,
    pub tx_hash: Vec<u8>,
    pub nonce: U256,
    pub sender: Vec<u8>,
    pub target: Vec<u8>,
    pub value: U256,
    pub gas_limit: U256,
    pub data: Vec<u8>,
    pub withdrawal_hash: Vec<u8>,
}

impl TryFrom<Layer3Withdrawal> for optimism_children_l3_withdrawals::ActiveModel {
    type Error = anyhow::Error;

    fn try_from(v: Layer3Withdrawal) -> Result<Self> {
        Ok(Self {
            id: Default::default(),
            chain_id: Set(v.chain_id),
            block_number: Set(v.block_number),
            block_hash: Set(v.block_hash),
            tx_hash: Set(v.tx_hash),
            nonce: Set(BigDecimal::from_str(&v.nonce.to_string())?),
            sender: Set(v.sender),
            target: Set(v.target),
            value: Set(BigDecimal::from_str(&v.value.to_string())?),
            gas_limit: Set(BigDecimal::from_str(&v.gas_limit.to_string())?),
            data: Set(v.data),
            withdrawal_hash: Set(v.withdrawal_hash),
            inserted_at: Default::default(),
        })
    }
}
