use alloy_primitives::{address, b256, Address, B256};

pub const SECS_PER_BLOCK: i64 = 2;

pub const TRANSACTION_DEPOSITED_SIG: B256 =
    b256!("0xb3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32");

pub const WITHDRAWAL_PROVEN_SIG: B256 =
    b256!("0x67a6208cfcc0801d50f6cbe764733f4fddf66ac0b04442061a8a8c0cb6b63f62");

pub const WITHDRAWAL_FINALIZED_SIG: B256 =
    b256!("0xdb5c7652857aa163daadd670e116628fb42e869d8ac4251ef8971d9e5727df1b");

pub const ARKIV_HOUSEKEEPING_ADDRESS: Address =
    address!("deaddeaddeaddeaddeaddeaddeaddeaddead0001");

pub const OPTIMISM_L3_TO_L2_MESSAGE_PASSER_ADDRESS: Address =
    address!("0x4200000000000000000000000000000000000016");
