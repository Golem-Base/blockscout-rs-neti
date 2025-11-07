use optimism_children_indexer_l3::types::Layer3Chains;

pub fn build_test_chain_config(
    chain_name: &str,
    chain_id: i64,
    rpc_url: &str,
    last_indexed_block: u64,
) -> Layer3Chains::Model {
    Layer3Chains::Model {
        chain_id,
        chain_name: chain_name.to_string(),
        l3_rpc_url: rpc_url.to_string(),
        l3_rpc_url_fallback: Default::default(),
        l3_message_passer: Default::default(),
        l3_standard_bridge: Default::default(),
        l2_portal_address: Default::default(),
        l3_batch_size: 20000,
        l3_last_indexed_block: last_indexed_block as i64,
        l3_latest_block: None,
        l3_latest_block_updated_at: None,
        enabled: true,
        inserted_at: Default::default(),
        updated_at: Default::default(),
    }
}
