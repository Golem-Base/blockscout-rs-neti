#![allow(clippy::derive_partial_eq_without_eq)]
pub mod blockscout {
    pub mod golem_base_indexer {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.golem_base_indexer.v1.rs"
            ));
        }
    }
}

impl From<&golem_base_indexer_logic::types::OperationData>
    for blockscout::golem_base_indexer::v1::OperationType
{
    fn from(value: &golem_base_indexer_logic::types::OperationData) -> Self {
        use golem_base_indexer_logic::types::OperationData::*;
        match value {
            Create(_, _) => Self::Create,
            Update(_, _) => Self::Update,
            Delete => Self::Delete,
            Extend(_) => Self::Extend,
        }
    }
}

impl From<golem_base_indexer_logic::types::Operation>
    for blockscout::golem_base_indexer::v1::Operation
{
    fn from(op: golem_base_indexer_logic::types::Operation) -> Self {
        let operation_type: blockscout::golem_base_indexer::v1::OperationType =
            (&op.operation).into();

        Self {
            entity_key: format!("0x{:x}", op.metadata.entity_key),
            sender: op.metadata.sender.to_checksum(None), // FIXME provide chain id?
            operation: operation_type.into(),
            data: op.operation.clone().data().map(|v| format!("0x{v:x}")),
            btl: op.operation.btl(),
            block_hash: format!("0x{:x}", op.metadata.block_hash),
            transaction_hash: format!("0x{:x}", op.metadata.tx_hash),
            index: op.metadata.index,
        }
    }
}

impl From<&golem_base_indexer_logic::types::EntityStatus>
    for blockscout::golem_base_indexer::v1::EntityStatus
{
    fn from(value: &golem_base_indexer_logic::types::EntityStatus) -> Self {
        use golem_base_indexer_logic::types::EntityStatus::*;
        match value {
            Active => Self::Active,
            Deleted => Self::Deleted,
            Expired => Self::Expired,
        }
    }
}

// FIXME blockscout has something called display_bytes?
impl From<golem_base_indexer_logic::types::Entity> for blockscout::golem_base_indexer::v1::Entity {
    fn from(entity: golem_base_indexer_logic::types::Entity) -> Self {
        let status: blockscout::golem_base_indexer::v1::EntityStatus = (&entity.status).into();

        Self {
            key: format!("0x{:x}", entity.key),
            data: entity.data.map(|v| format!("0x{v:x}")),
            status: status.into(),
            created_at_tx_hash: entity.created_at_tx_hash.map(|v| format!("0x{v:x}")),
            last_updated_at_tx_hash: format!("0x{:x}", entity.last_updated_at_tx_hash),
            expires_at_block_number: entity.expires_at_block_number,
        }
    }
}
