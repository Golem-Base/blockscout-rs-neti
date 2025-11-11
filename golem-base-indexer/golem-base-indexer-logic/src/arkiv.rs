use crate::{
    types::{Block, BlockNumber, Bytes, EntityKey, Timestamp, TxHash},
    well_known::SECS_PER_BLOCK,
};
use alloy_primitives::{keccak256, U256};
use alloy_sol_types::SolValue;
use anyhow::Result;
use chrono::Duration;

pub fn block_timestamp(number: BlockNumber, reference_block: &Block) -> Option<Timestamp> {
    let diff = (number as i64).checked_sub(reference_block.number as i64)?;
    let secs = diff.checked_mul(SECS_PER_BLOCK)?;
    let duration = Duration::try_seconds(secs)?;

    reference_block.timestamp.checked_add_signed(duration)
}

pub fn block_timestamp_sec(number: BlockNumber, reference_block: &Block) -> Option<u64> {
    let diff_blocks = number.saturating_sub(reference_block.number);
    let diff_secs = diff_blocks.saturating_mul(SECS_PER_BLOCK as u64);
    let base_secs = reference_block.timestamp.timestamp() as u64;

    base_secs.checked_add(diff_secs)
}

pub fn entity_key(tx_hash: TxHash, data: Bytes, create_op_idx: u64) -> EntityKey {
    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(tx_hash.as_slice());
    buf.extend_from_slice(&data);

    let idx: U256 = create_op_idx
        .try_into()
        .expect("Array index is always positive");
    buf.extend_from_slice(&idx.to_be_bytes::<32>());
    keccak256(buf)
}

pub fn decode_extend_log_data(data: &Bytes) -> Result<u64> {
    type EventArgs = (U256, U256);
    let (_, expires_at_block_number) = EventArgs::abi_decode(data)?;
    Ok(expires_at_block_number.try_into()?)
}

#[cfg(test)]
mod tests {
    use crate::arkiv::{block_timestamp, block_timestamp_sec, entity_key, Block};
    use alloy_primitives::{b256, bytes};

    #[test]
    fn entity_key_calculated_correctly() {
        let expected_key =
            b256!("0x35d1ae22f8813a630b1a4d6b8660113ed226d684511747b35dd764c7f96251c5");
        let tx_hash = b256!("0x296508b5285b8596691435c7089e591d2fad7d3756279820347696cdb09197a4");
        let data = bytes!("0x74657374");
        let create_op_idx = 0;
        assert_eq!(
            expected_key,
            entity_key(tx_hash, data.into(), create_op_idx)
        );

        let expected_key =
            b256!("0xa659f43417c43e9da5801d9b0ab8680bbe5d5dff4c2094795b7bb58c76fed489");
        let tx_hash = b256!("0x5f9477df89b0e5649365e0c012670cbcb04bb02766117a4d7f031d10b3234866");
        let data = bytes!("74736574");
        let create_op_idx = 1;
        assert_eq!(
            expected_key,
            entity_key(tx_hash, data.into(), create_op_idx)
        );
    }

    #[test]
    fn block_timestamp_sec_calculated() {
        let date = chrono::DateTime::from_timestamp(1750000000, 0).unwrap();
        assert_eq!(date.timestamp(), 1750000000);
        let reference_block = Block {
            hash: alloy_primitives::BlockHash::ZERO,
            number: 1,
            timestamp: date,
        };
        // MAX that FE will take as an input
        let blocks_into_the_future: u64 = 900_000_000_000_000;
        let target_block = reference_block.number + blocks_into_the_future;
        // 1_750_000_000 + 9000000000000000 * 2
        let expected = Some(1_800_001_750_000_000);

        let result = block_timestamp_sec(target_block, &reference_block);
        assert_eq!(result, expected);

        let blocks_into_the_future: u64 = 900_000_000_000_000 * 10_000;
        let target_block = reference_block.number + blocks_into_the_future;
        let result = block_timestamp_sec(target_block, &reference_block);
        // 585+ billion years into the future
        let expected = Some(18_000_000_001_750_000_000);
        assert_eq!(result, expected);

        let result = block_timestamp_sec(u64::MAX, &reference_block);
        assert_eq!(result, None);
    }

    #[test]
    fn block_timestamp_and_block_timestamp_sec_match() {
        let date = chrono::DateTime::from_timestamp(1_750_000_000, 0).unwrap();
        let reference_block = Block {
            hash: alloy_primitives::BlockHash::ZERO,
            number: 1,
            timestamp: date,
        };
        for blocks_into_the_future in [0, 1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
            let target_block = reference_block.number + blocks_into_the_future;
            let ts = block_timestamp(target_block, &reference_block).unwrap();
            let ts_sec = block_timestamp_sec(target_block, &reference_block).unwrap();
            assert_eq!(ts.timestamp() as u64, ts_sec);
            assert_eq!(
                chrono::DateTime::from_timestamp(ts_sec as i64, 0).unwrap(),
                ts
            );
        }
    }
}
