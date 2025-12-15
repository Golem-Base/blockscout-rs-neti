use alloy_primitives::{address, b256, Address, B256};

pub const SECS_PER_BLOCK: i64 = 2;

pub const TRANSACTION_DEPOSITED_SIG: B256 =
    b256!("0xb3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32");

pub const ARKIV_HOUSEKEEPING_ADDRESS: Address =
    address!("deaddeaddeaddeaddeaddeaddeaddeaddead0001");

pub const OPTIMISM_L3_TO_L2_MESSAGE_PASSER_ADDRESS: Address =
    address!("0x4200000000000000000000000000000000000016");
