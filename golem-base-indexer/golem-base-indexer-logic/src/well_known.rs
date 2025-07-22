use alloy_primitives::{address, b256, Address, B256};

/// housekeeping tx in every block is sent to this address
pub const L1_BLOCK_CONTRACT_ADDRESS: Address =
    address!("0x4200000000000000000000000000000000000015");
/// transactions that manage storage entities must be sent to this address
pub const GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS: Address =
    address!("0x0000000000000000000000000000000060138453");
/// topic of GolemBaseStorageEntityBTLExtended event
pub const GOLEM_BASE_STORAGE_ENTITY_BTL_EXTENDED: B256 =
    b256!("0x835bfca6df78ffac92635dcc105a6a8c4fd715e054e18ef60448b0a6dce30c8d");
/// topic of GolemBaseStorageEntityDeleted event
pub const GOLEM_BASE_STORAGE_ENTITY_DELETED: B256 =
    b256!("0x0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93");
