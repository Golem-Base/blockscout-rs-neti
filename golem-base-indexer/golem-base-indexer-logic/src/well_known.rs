use alloy_primitives::{address, Address};

pub const SECS_PER_BLOCK: i64 = 2;

/// housekeeping tx in every block is sent to this address
pub const L1_BLOCK_CONTRACT_ADDRESS: Address =
    address!("0x4200000000000000000000000000000000000015");
// housekeeping tx sender
pub const L1_BLOCK_CONTRACT_SENDER_ADDRESS: Address =
    address!("0xDeaDDEaDDeAdDeAdDEAdDEaddeAddEAdDEAd0001");
/// deposit address
pub const DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("0x4200000000000000000000000000000000000007");
/// transactions that manage storage entities must be sent to this address
pub const GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS: Address =
    address!("0x00000000000000000000000000000061726B6976");
