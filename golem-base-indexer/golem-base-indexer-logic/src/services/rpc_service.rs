use alloy::rpc::{
    client::{ClientBuilder, RpcClient},
    types::Block as RpcBlock,
};
use anyhow::{Context, Result};
use tracing::instrument;
use url::Url;

use crate::types::{ConsensusBlockInfo, ConsensusBlocksInfo};

impl From<RpcBlock> for ConsensusBlockInfo {
    fn from(v: RpcBlock) -> Self {
        Self {
            block_number: v.number(),
            timestamp: v.header.timestamp,
        }
    }
}

#[derive(Debug)]
pub struct RpcService {
    pub client: RpcClient,
}

impl RpcService {
    #[instrument]
    pub fn new(url: Url) -> Self {
        tracing::debug!(%url, "RpcService created");

        Self {
            client: ClientBuilder::default().http(url),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_consensus_blocks_info(&self) -> Result<ConsensusBlocksInfo> {
        let mut batch = self.client.new_batch();
        let latest = batch
            .add_call("eth_getBlockByNumber", &("latest", false))?
            .map_resp(|v: RpcBlock| v);
        let safe = batch
            .add_call("eth_getBlockByNumber", &("safe", false))?
            .map_resp(|v: RpcBlock| v);
        let finalized = batch
            .add_call("eth_getBlockByNumber", &("finalized", false))?
            .map_resp(|v: RpcBlock| v);

        batch.send().await?;

        let (latest, safe, finalized) = tokio::try_join!(latest, safe, finalized)
            .context("failed to get consensus blocks info")?;

        Ok(ConsensusBlocksInfo {
            latest: latest.into(),
            safe: safe.into(),
            finalized: finalized.into(),
        })
    }
}
