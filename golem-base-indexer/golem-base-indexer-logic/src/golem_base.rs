use crate::{
    types::{Block, BlockNumber, Bytes, EntityKey, Timestamp, TxHash},
    well_known::SECS_PER_BLOCK,
};
use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use anyhow::Result;
use chrono::Duration;
use golem_base_sdk::keccak256;

pub fn block_timestamp(number: BlockNumber, reference_block: &Block) -> Option<Timestamp> {
    let diff = (number as i64).checked_sub(reference_block.number as i64)?;
    let secs = diff.checked_mul(SECS_PER_BLOCK)?;
    let duration = Duration::try_seconds(secs)?;

    reference_block.timestamp.checked_add_signed(duration)
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

pub fn format_duration(d: chrono::Duration) -> String {
    let mut secs = d.num_seconds().abs();
    let units = [(86_400, "d"), (3_600, "h"), (60, "m"), (1, "s")];

    let mut parts = vec![];
    for (unit_secs, label) in units {
        let val = secs / unit_secs;
        if val > 0 {
            parts.push(format!("{val}{label}"));
            secs %= unit_secs;
        }
    }

    if parts.is_empty() {
        return "0s".to_string();
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use crate::golem_base::{entity_key, format_duration};
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
    fn format_duration_is_correct() {
        assert_eq!(format_duration(chrono::Duration::seconds(0)), "0s");
        assert_eq!(format_duration(chrono::Duration::milliseconds(900)), "0s");
        assert_eq!(format_duration(chrono::Duration::seconds(1)), "1s");
        assert_eq!(format_duration(chrono::Duration::seconds(60)), "1m");
        assert_eq!(format_duration(chrono::Duration::seconds(60 + 1)), "1m 1s");
        assert_eq!(format_duration(chrono::Duration::seconds(3_600)), "1h");
        assert_eq!(
            format_duration(chrono::Duration::seconds(3_600 + 60 + 1)),
            "1h 1m 1s"
        );
        assert_eq!(format_duration(chrono::Duration::seconds(86_400)), "1d");
        assert_eq!(
            format_duration(chrono::Duration::seconds(86_400 + 3_600 + 60 + 1)),
            "1d 1h 1m 1s"
        );
    }
}
