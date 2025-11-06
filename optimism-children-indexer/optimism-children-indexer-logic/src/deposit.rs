use alloy_primitives::{keccak256, BlockHash, Bytes, B256, U256};
use alloy_sol_types::SolValue;
use anyhow::{ensure, Result};

use crate::types::DepositV0;

// keccak256(bytes32(uint256(0)), keccak256(l1BlockHash, bytes32(uint256(l1LogIndex)))).
pub fn source_hash(l1_block_hash: BlockHash, l1_log_index: U256) -> B256 {
    let inner_encoded = (l1_block_hash, B256::from(l1_log_index)).abi_encode();
    let inner_hash = keccak256(&inner_encoded);
    let outer_encoded = (B256::ZERO, inner_hash).abi_encode();
    keccak256(&outer_encoded)
}

impl TryFrom<Bytes> for DepositV0 {
    type Error = anyhow::Error;

    fn try_from(encoded: Bytes) -> Result<Self> {
        ensure!(encoded.len() >= 73, "Invalid length of deposit data");

        let _offset = U256::from_be_slice(encoded[0..32].try_into().unwrap());
        let length: usize = U256::from_be_slice(encoded[32..64].try_into().unwrap())
            .try_into()
            .unwrap();
        let mint = U256::from_be_slice(encoded[64..96].try_into().unwrap());
        let value = U256::from_be_slice(encoded[96..128].try_into().unwrap());
        let gas_limit = u64::from_be_bytes(encoded[128..136].try_into().unwrap());
        let is_creation = encoded[136] != 0;
        let calldata = Bytes::copy_from_slice(&encoded[137..(length + 64)]);

        Ok(Self {
            mint,
            value,
            gas_limit,
            is_creation,
            calldata,
        })
    }
}
