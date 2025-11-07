use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, method},
};

pub struct EthMockServer {
    pub server: MockServer,
    pub latest_block_number: u64,
}

impl EthMockServer {
    pub async fn start(latest_block_number: u64) -> Self {
        let server = MockServer::start().await;
        Self {
            server,
            latest_block_number,
        }
    }

    pub fn uri(&self) -> String {
        self.server.uri()
    }

    pub fn create_block_response(
        block_number: Option<u64>,
        block_hash: Option<&str>,
        transactions: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let number = block_number
            .map(|n| format!("0x{:x}", n))
            .unwrap_or_else(|| "0x1234".to_string());

        let hash = block_hash
            .unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000");

        let txs = transactions.unwrap_or_else(|| json!([]));

        json!({
            "number": number,
            "hash": hash,
            "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "nonce": "0x0000000000000000",
            "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
            "stateRoot": "0xd7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544",
            "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
            "miner": "0x0000000000000000000000000000000000000000",
            "difficulty": "0x0",
            "totalDifficulty": "0x0",
            "extraData": "0x",
            "size": "0x200",
            "gasLimit": "0x1c9c380",
            "gasUsed": "0x0",
            "timestamp": "0x60000000",
            "transactions": txs,
            "uncles": [],
            "mixHash": "0x143a3787fe8c25e3e97e83d33d5cf873222b977b250399ac663c0a452ef40b68"
        })
    }

    pub async fn mount_defaults(&self) {
        self.mount_block_number().await;
        self.mount_get_block_by_number_default().await;
        self.mount_get_block_receipts_default().await;
    }

    async fn mount_block_number(&self) {
        let block_hex = format!("0x{:x}", self.latest_block_number);

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": block_hex
        });

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_blockNumber"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }

    async fn mount_get_block_by_number_default(&self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": Self::create_block_response(None, None, None),
        });

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getBlockByNumber"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }

    async fn mount_get_block_receipts_default(&self) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": []
        });

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getBlockReceipts"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }

    pub async fn mount_block_by_number(
        &self,
        block_number: u64,
        block_hash: Option<&str>,
        transactions: Option<serde_json::Value>,
    ) {
        let block_hex = format!("0x{:x}", block_number);

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": Self::create_block_response(Some(block_number), block_hash, transactions),
        });

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getBlockByNumber",
                "params": [block_hex, true]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }

    pub async fn mount_block_receipts(&self, block_number: u64, receipts: serde_json::Value) {
        let block_hex = format!("0x{:x}", block_number);

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": receipts
        });

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getBlockReceipts",
                "params": [block_hex]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }
}
