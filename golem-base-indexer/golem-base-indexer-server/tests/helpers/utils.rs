use anyhow::Result;
use bytes::Bytes;
use golem_base_indexer_logic::mat_view_scheduler::MatViewScheduler;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub fn bytes_to_hex(bytes: &Bytes) -> String {
    let slice = bytes.as_ref();
    let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
    format!("0x{hex}")
}

pub async fn refresh_leaderboards(db: Arc<DatabaseConnection>) -> Result<()> {
    let scheduler = MatViewScheduler::new(db);
    let views = scheduler
        .get_mat_view_settings()
        .into_iter()
        .filter(|v| v.name.contains("leaderboard"));
    for view in views {
        scheduler.refresh_named_view(&view.name).await;
    }
    Ok(())
}

pub async fn refresh_timeseries(db: Arc<DatabaseConnection>) -> Result<()> {
    let scheduler = MatViewScheduler::new(db);
    let views = scheduler
        .get_mat_view_settings()
        .into_iter()
        .filter(|v| v.name.contains("timeseries"));
    for view in views {
        scheduler.refresh_named_view(&view.name).await;
    }
    Ok(())
}

pub fn iso_to_ts_sec(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .unwrap()
        .timestamp()
        .to_string()
}

// Helper to generate RPC block response for wiremock
pub fn gen_block_resp(block_number: u64, timestamp: u64, rpc_id: usize) -> serde_json::Value {
    let block_number_hex = format!("0x{block_number:x}");
    let timestamp_hex = format!("0x{timestamp:x}");

    serde_json::json!({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "result": {
            "hash": format!("0x{:064x}", block_number + 0x1000000000000000),
            "number": block_number_hex,
            "timestamp": timestamp_hex,
            "parentHash": format!("0x{:064x}", block_number - 1 + 0x1000000000000000),
            "difficulty": "0x0",
            "totalDifficulty": "0x0",
            "gasLimit": "0x1000000",
            "gasUsed": "0x0",
            "miner": "0x0000000000000000000000000000000000000000",
            "nonce": "0x0000000000000000",
            "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            "stateRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "receiptsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "transactionsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "extraData": "0x",
            "size": "0x0",
            "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "transactions": [],
            "uncles": []
        }
    })
}
