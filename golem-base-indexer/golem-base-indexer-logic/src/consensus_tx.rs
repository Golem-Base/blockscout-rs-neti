use crate::types::{ConsensusTx, Tx};
use anyhow::{anyhow, Result};

impl TryFrom<Tx> for ConsensusTx {
    type Error = anyhow::Error;

    fn try_from(tx: Tx) -> Result<Self> {
        Ok(Self {
            hash: tx.hash,
            from_address_hash: tx.from_address_hash,
            to_address_hash: tx.to_address_hash,
            block_number: tx.block_number.ok_or(anyhow!("Tx not in block yet"))?,
            block_hash: tx.block_hash.ok_or(anyhow!("Tx not in block yet"))?,
            block_timestamp: tx.block_timestamp.ok_or(anyhow!("Tx not in block yet"))?,
            input: tx.input,
            index: tx.index.ok_or(anyhow!("Tx not in block yet"))?,
        })
    }
}
