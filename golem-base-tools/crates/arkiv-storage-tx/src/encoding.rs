use alloy_primitives::Bytes;
use alloy_rlp::{Bytes as RLPBytes, encode};
use brotli::{BrotliCompress, enc::BrotliEncoderParams};

use super::storage_tx::StorageTransaction;

impl TryFrom<StorageTransaction> for Bytes {
    type Error = std::io::Error;

    fn try_from(src: StorageTransaction) -> Result<Self, Self::Error> {
        let input = encode(src);
        let mut outbuf = Vec::<u8>::new();
        let params = BrotliEncoderParams::default();
        BrotliCompress(&mut &input[..], &mut outbuf, &params)?;

        Ok(outbuf.into())
    }
}

impl TryFrom<StorageTransaction> for RLPBytes {
    type Error = std::io::Error;

    fn try_from(src: StorageTransaction) -> Result<Self, Self::Error> {
        let bytes: Bytes = src.try_into()?;
        Ok(bytes.into())
    }
}
