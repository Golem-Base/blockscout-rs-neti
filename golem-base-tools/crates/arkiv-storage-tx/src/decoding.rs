use alloy_primitives::Bytes;
use alloy_rlp::{Bytes as RLPBytes, Decodable};
use brotli::BrotliDecompress;

use super::{error::Error, storage_tx::StorageTransaction};

// FIXME make it generic
impl TryFrom<&Bytes> for StorageTransaction {
    type Error = Error;

    fn try_from(src: &Bytes) -> Result<Self, Self::Error> {
        let mut buf = Vec::<u8>::new();
        BrotliDecompress(&mut &src[..], &mut buf)?;

        Self::decode(&mut buf.as_slice()).map_err(Into::into)
    }
}

impl TryFrom<&RLPBytes> for StorageTransaction {
    type Error = Error;

    fn try_from(src: &RLPBytes) -> Result<Self, Self::Error> {
        let mut buf = Vec::<u8>::new();
        BrotliDecompress(&mut &src[..], &mut buf)?;

        Self::decode(&mut buf.as_slice()).map_err(Into::into)
    }
}
