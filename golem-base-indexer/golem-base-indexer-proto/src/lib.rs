#![allow(clippy::derive_partial_eq_without_eq)]

use const_hex::traits::ToHexExt;

use golem_base_indexer_logic::types::{
    Entity, EntityStatus, FullEntity, NumericAnnotation, Operation, OperationData, StringAnnotation,
};

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

use blockscout::golem_base_indexer::v1;

impl v1::FullEntity {
    pub fn new(
        entity: FullEntity,
        string_annotations: Vec<StringAnnotation>,
        numeric_annotations: Vec<NumericAnnotation>,
    ) -> Self {
        let status: v1::EntityStatus = entity.status.into();
        let data_size = entity.data.as_ref().map(|v| v.len() as u64);
        Self {
            key: entity.key.to_string(),
            data: entity.data.map(|v| v.encode_hex_with_prefix()),
            data_size,
            status: status.into(),
            owner: entity.owner.to_checksum(None),

            created_at_tx_hash: entity
                .created_at_tx_hash
                .as_ref()
                .map(ToHexExt::encode_hex_with_prefix),
            created_at_operation_index: entity.created_at_operation_index.map(|v| v.to_string()),
            created_at_block_number: entity.created_at_block_number,
            created_at_timestamp: entity.created_at_timestamp.map(|v| v.to_rfc3339()),

            expires_at_timestamp: entity.expires_at_timestamp.to_rfc3339(),
            expires_at_block_number: entity.expires_at_block_number,
            fees_paid: entity.fees_paid.to_string(),
            gas_used: entity.gas_used.to_string(),

            string_annotations: string_annotations.into_iter().map(Into::into).collect(),
            numeric_annotations: numeric_annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<StringAnnotation> for v1::StringAnnotation {
    fn from(value: StringAnnotation) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<NumericAnnotation> for v1::NumericAnnotation {
    fn from(value: NumericAnnotation) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<&OperationData> for v1::OperationType {
    fn from(value: &OperationData) -> Self {
        match value {
            OperationData::Create(_, _) => Self::Create,
            OperationData::Update(_, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
        }
    }
}

impl From<Operation> for v1::Operation {
    fn from(op: Operation) -> Self {
        let operation_type: v1::OperationType = (&op.operation).into();

        Self {
            entity_key: op.metadata.entity_key.to_string(),
            sender: op.metadata.sender.to_checksum(None),
            operation: operation_type.into(),
            data: op
                .operation
                .clone()
                .data()
                .map(ToHexExt::encode_hex_with_prefix),
            btl: op.operation.btl(),
            block_hash: op.metadata.block_hash.to_string(),
            transaction_hash: op.metadata.tx_hash.to_string(),
            index: op.metadata.index,
        }
    }
}

impl From<EntityStatus> for v1::EntityStatus {
    fn from(value: EntityStatus) -> Self {
        match value {
            EntityStatus::Active => Self::Active,
            EntityStatus::Deleted => Self::Deleted,
            EntityStatus::Expired => Self::Expired,
        }
    }
}

impl From<Entity> for v1::Entity {
    fn from(entity: Entity) -> Self {
        let status: v1::EntityStatus = entity.status.into();

        Self {
            key: entity.key.to_string(),
            data: entity.data.as_ref().map(ToHexExt::encode_hex_with_prefix),
            status: status.into(),
            created_at_tx_hash: entity.created_at_tx_hash.map(|v| v.to_string()),
            last_updated_at_tx_hash: entity.last_updated_at_tx_hash.to_string(),
            expires_at_block_number: entity.expires_at_block_number,
        }
    }
}
