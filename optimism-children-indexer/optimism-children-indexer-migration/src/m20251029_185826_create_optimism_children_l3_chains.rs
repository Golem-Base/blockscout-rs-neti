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
            l3_rpc_url_backup TEXT,
            l3_rpc_batch_size INTEGER DEFAULT 2000,  -- Blocks to fetch per iteration
            
            -- Contract Addresses
            l3_message_passer VARCHAR(42) DEFAULT '0x4200000000000000000000000000000000000016',
            l3_standard_bridge VARCHAR(42) DEFAULT '0x4200000000000000000000000000000000000010',
            l2_portal_address VARCHAR(42) NOT NULL,  -- Portal on L2 (this chain) where withdrawals finalize
            
            -- Indexing Status
            l3_last_indexed_block BIGINT DEFAULT 0,
            l3_latest_block BIGINT,  -- Last known block from eth_blockNumber (updated each iteration)
            l3_latest_block_updated_at TIMESTAMP,  -- When we last fetched latest block
            
            -- Chain State
            enabled BOOLEAN DEFAULT true,
            
            -- Timestamps
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );
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
