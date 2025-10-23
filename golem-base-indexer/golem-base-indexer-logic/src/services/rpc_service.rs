use std::sync::Arc;

use alloy::rpc::{
    client::{ClientBuilder, RpcClient},
    types::Block as RpcBlock,
};
use anyhow::{Context, Result};
use moka::future::Cache;
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

#[derive(Debug, Clone)]
pub struct RpcService {
    pub client: RpcClient,
    pub cache: Arc<Cache<String, ConsensusBlocksInfo>>,
}

impl RpcService {
    #[instrument]
    pub fn new(url: Url, cache_ttl: u64) -> Self {
        tracing::debug!(%url, "RpcService created");

        Self {
            client: ClientBuilder::default().http(url),
            cache: Arc::new(
                Cache::builder()
                    .time_to_live(std::time::Duration::from_secs(cache_ttl))
                    .max_capacity(1_000)
                    .build(),
            ),
        }
    }

    #[instrument(skip(self))]
    async fn get_consensus_blocks_info(&self) -> Result<ConsensusBlocksInfo> {
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

    pub async fn get_consensus_blocks_info_cached(&self) -> Result<ConsensusBlocksInfo> {
        let key = "consensus_blocks_info".to_string();

        let cache = self.cache.clone();
        let s = self.clone();

        let res = cache
            .get_with(key.clone(), async move {
                match s.get_consensus_blocks_info().await {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::error!(%e, "failed to get consensus blocks info");
                        Default::default()
                    }
                }
            })
            .await;

        Ok(res)
    }
}
