use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use url::Url;

use crate::types::ConsensusGasInfo;

/// Blockscout /api/v2/addresses/{address}/transactions response
#[derive(Debug, Clone, Deserialize)]
pub struct AddressTransactionsResponse {
    pub items: Vec<AddressTransaction>,
    pub next_page_params: Option<NextPageParams>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddressTransaction {
    pub hash: String,
    pub result: Option<String>,
    pub status: Option<String>,
    pub confirmations: Option<u64>,
    pub to: AddressInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddressInfo {
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NextPageParams {
    pub index: Option<u64>,
    pub value: Option<String>,
    pub filter: Option<String>,
    pub hash: Option<String>,
    pub inserted_at: Option<String>,
    pub block_number: Option<u64>,
    pub fee: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

/// Blockscout /api/v2/transactions/{txhash} response
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionResponse {
    pub hash: String,
    pub gas_used: Option<String>,
    pub gas_price: Option<String>,
}

pub struct BlockscoutService {
    pub client: Client,
    pub url: Url,
    pub batcher_address: String,
    pub batch_inbox_address: String,
}

impl BlockscoutService {
    #[instrument]
    pub fn new(url: Url, batcher_address: String, batch_inbox_address: String) -> Self {
        let client = Client::builder()
            .user_agent("golem-base-indexer/0.1")
            .build()
            .expect("valid reqwest client");

        tracing::debug!(%url, %batcher_address, %batch_inbox_address, "BlockscoutService created");

        Self {
            client,
            url,
            batcher_address,
            batch_inbox_address,
        }
    }

    #[instrument(skip(self))]
    async fn get_txlist(
        &self,
        from: &str,
        pagination: Option<NextPageParams>,
    ) -> Result<(Vec<AddressTransaction>, Option<NextPageParams>)> {
        let mut query = self.url.clone();
        query.set_path(&format!("/api/v2/addresses/{from}/transactions"));

        if let Some(pagination) = &pagination {
            let query_str = serde_urlencoded::to_string(pagination)?;
            query.set_query(Some(&query_str));
        }

        let response: AddressTransactionsResponse =
            self.client.get(query).send().await?.json().await?;
        let txlist = response.items;
        let pagination = response.next_page_params;

        Ok((txlist, pagination))
    }

    #[instrument(skip(self))]
    fn pick_tx_from_list(&self, txlist: Vec<AddressTransaction>) -> Option<AddressTransaction> {
        txlist.into_iter().find(|tx| {
            tx.to.hash.to_lowercase() == self.batch_inbox_address.to_lowercase()
                && tx.status.is_some()
                && tx.result.is_some()
                && tx.result.as_ref().unwrap() == "success"
                && tx.status.as_ref().unwrap() == "ok"
                && tx.confirmations.is_some()
                && tx.confirmations.as_ref().unwrap_or(&0) > &0
        })
    }

    #[instrument(skip(self))]
    async fn get_verified_tx(&self) -> Result<AddressTransaction> {
        let mut pagination: Option<NextPageParams> = None;
        let mut req_limit = 5; // 5*50=250 txs total, should be enough

        loop {
            let (txlist, new_pagination) =
                self.get_txlist(&self.batcher_address, pagination).await?;
            let tx = self.pick_tx_from_list(txlist);

            if let Some(tx) = tx {
                return Ok(tx);
            }
            pagination = new_pagination;
            if req_limit == 0 {
                anyhow::bail!("tx not found in txlist after max requests");
            }
            req_limit -= 1;
        }
    }

    #[instrument(skip(self))]
    async fn get_txinfo(&self, txhash: &str) -> Result<TransactionResponse> {
        let mut query = self.url.clone();
        query.set_path(&format!("/api/v2/transactions/{txhash}"));

        let response: TransactionResponse = self.client.get(query).send().await?.json().await?;

        if response.hash.to_lowercase() != txhash.to_lowercase() {
            return Err(anyhow!("mismatched txhash in txinfo response"));
        }

        Ok(response)
    }

    #[instrument(skip(self))]
    fn get_gas_info(&self, txinfo: TransactionResponse) -> Result<ConsensusGasInfo> {
        let gas_used = txinfo
            .gas_used
            .ok_or(anyhow!("missing gas_used"))?
            .parse::<u64>()?;
        let gas_price = txinfo
            .gas_price
            .ok_or(anyhow!("missing gas_used"))?
            .parse::<u64>()?;
        let transaction_fee = gas_used
            .checked_mul(gas_price)
            .ok_or(anyhow!("transaction_fee overflow"))?;

        Ok(ConsensusGasInfo {
            gas_used,
            gas_price,
            transaction_fee,
        })
    }

    #[instrument(skip(self))]
    pub async fn get_consensus_gas_info(&self) -> Result<ConsensusGasInfo> {
        let tx = self.get_verified_tx().await?;
        let txinfo = self.get_txinfo(&tx.hash).await?;

        self.get_gas_info(txinfo)
    }
}
