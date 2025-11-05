use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("
        CREATE TABLE optimism_children_l3_chains (
            chain_id BIGINT PRIMARY KEY,
            chain_name VARCHAR(128) UNIQUE NOT NULL,
            
            -- RPC Configuration
            l3_rpc_url TEXT NOT NULL,
            l3_rpc_url_fallback TEXT,

            -- Contract Addresses
            l3_message_passer BYTEA DEFAULT decode('4200000000000000000000000000000000000016', 'hex') NOT NULL,
            l3_standard_bridge BYTEA DEFAULT decode('4200000000000000000000000000000000000010', 'hex') NOT NULL,
            l2_portal_address BYTEA NOT NULL,  -- Portal on L2 (this chain) where withdrawals finalize

            -- Indexer Configuration and Status
            l3_batch_size INTEGER DEFAULT 2000 NOT NULL,  -- Blocks to fetch per iteration
            l3_last_indexed_block BIGINT DEFAULT 0 NOT NULL,
            l3_latest_block BIGINT,  -- Last known block from eth_blockNumber (updated each iteration)
            l3_latest_block_updated_at TIMESTAMP,  -- When we last fetched latest block
            
            -- Chain State
            enabled BOOLEAN DEFAULT true NOT NULL,
            
            -- Timestamps
            created_at TIMESTAMP DEFAULT NOW() NOT NULL,
            updated_at TIMESTAMP DEFAULT NOW() NOT NULL
        );

        CREATE INDEX idx_optimism_children_l3_chains_enabled
            ON optimism_children_l3_chains(chain_id)
            WHERE enabled = true;

        CREATE INDEX idx_optimism_children_l3_chains_l2_portal_address
            ON optimism_children_l3_chains(l2_portal_address);
        ")
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE optimism_children_l3_chains")
            .await?;

        Ok(())
    }
}
