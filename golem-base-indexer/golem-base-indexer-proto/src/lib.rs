#![allow(clippy::derive_partial_eq_without_eq)]

use const_hex::traits::ToHexExt;

use anyhow::{anyhow, Result};
use golem_base_indexer_logic::types::{
    AddressByEntitiesOwned, BiggestSpenders, BlockEntitiesCount, BlockStorageUsage, EntitiesFilter,
    Entity, EntityDataSize, EntityHistoryEntry, EntityHistoryFilter, EntityStatus,
    EntityWithExpTimestamp, FullEntity, ListEntitiesFilter, ListOperationsFilter,
    NumericAnnotation, NumericAnnotationWithRelations, OperationData, OperationFilter,
    OperationView, OperationsCount, OperationsFilter, PaginationMetadata, PaginationParams,
    StringAnnotation, StringAnnotationWithRelations,
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
        string_annotations: Vec<StringAnnotationWithRelations>,
        numeric_annotations: Vec<NumericAnnotationWithRelations>,
    ) -> Self {
        let status: v1::EntityStatus = entity.status.into();
        let data_size = entity.data.as_ref().map(|v| v.len() as u64);
        Self {
            key: entity.key.to_string(),
            data: entity.data.map(|v| v.encode_hex_with_prefix()),
            data_size,
            status: status.into(),
            owner: entity.owner.map(|v| v.to_checksum(None)),

            created_at_tx_hash: entity
                .created_at_tx_hash
                .as_ref()
                .map(ToHexExt::encode_hex_with_prefix),
            created_at_operation_index: entity.created_at_operation_index.map(|v| v.to_string()),
            created_at_block_number: entity.created_at_block_number,
            created_at_timestamp: entity.created_at_timestamp.map(|v| v.to_rfc3339()),

            updated_at_tx_hash: entity.updated_at_tx_hash.encode_hex_with_prefix(),
            updated_at_operation_index: entity.updated_at_operation_index.to_string(),
            updated_at_block_number: entity.updated_at_block_number,
            updated_at_timestamp: entity.updated_at_timestamp.to_rfc3339(),

            expires_at_timestamp: entity.expires_at_timestamp.map(|v| v.to_rfc3339()),
            expires_at_block_number: entity.expires_at_block_number,
            fees_paid: entity.fees_paid.to_string(),
            gas_used: entity.gas_used.to_string(),

            string_annotations: string_annotations.into_iter().map(Into::into).collect(),
            numeric_annotations: numeric_annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<StringAnnotationWithRelations> for v1::StringAnnotationWithRelations {
    fn from(value: StringAnnotationWithRelations) -> Self {
        Self {
            key: value.annotation.key,
            value: value.annotation.value,
            related_entities: value.related_entities,
        }
    }
}

impl From<NumericAnnotationWithRelations> for v1::NumericAnnotationWithRelations {
    fn from(value: NumericAnnotationWithRelations) -> Self {
        Self {
            key: value.annotation.key,
            value: value.annotation.value,
            related_entities: value.related_entities,
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

impl From<v1::StringAnnotation> for StringAnnotation {
    fn from(value: v1::StringAnnotation) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<v1::NumericAnnotation> for NumericAnnotation {
    fn from(value: v1::NumericAnnotation) -> Self {
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
impl From<OperationData> for v1::OperationType {
    fn from(value: OperationData) -> Self {
        match value {
            OperationData::Create(_, _) => Self::Create,
            OperationData::Update(_, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
        }
    }
}
impl From<v1::OperationType> for OperationData {
    fn from(value: v1::OperationType) -> Self {
        match value {
            v1::OperationType::Create => Self::Create(Vec::new().into(), 0),
            v1::OperationType::Update => Self::Update(Vec::new().into(), 0),
            v1::OperationType::Delete => Self::Delete,
            v1::OperationType::Extend => Self::Extend(0),
        }
    }
}

impl TryFrom<v1::ListOperationsRequest> for ListOperationsFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListOperationsRequest) -> Result<Self> {
        let operation_type = v1::OperationType::try_from(request.operation)
            .map_err(|_| anyhow!("Invalid operation"))?
            .into();

        Ok(Self {
            pagination: PaginationParams {
                page: request.page.unwrap_or(1).max(1),
                page_size: request.page_size.unwrap_or(100).clamp(1, 100),
            },
            operation_type: Some(operation_type),
            operations_filter: OperationsFilter {
                block_number_or_hash: request
                    .block_number_or_hash
                    .map(|v| {
                        v.parse()
                            .map_err(|_| anyhow!("Invalid block_number_or_hash"))
                    })
                    .transpose()?,
                transaction_hash: request
                    .transaction_hash
                    .map(|hash| {
                        hash.parse()
                            .map_err(|_| anyhow!("Invalid transaction_hash"))
                    })
                    .transpose()?,
                sender: request
                    .sender
                    .map(|addr| addr.parse().map_err(|_| anyhow!("Invalid sender")))
                    .transpose()?,
                entity_key: request
                    .entity_key
                    .map(|key| key.parse().map_err(|_| anyhow!("Invalid entity_key")))
                    .transpose()?,
            },
        })
    }
}

impl From<PaginationMetadata> for v1::Pagination {
    fn from(value: PaginationMetadata) -> Self {
        Self {
            page: value.pagination.page,
            page_size: value.pagination.page_size,
            total_pages: value.total_pages,
            total_items: value.total_items,
        }
    }
}

impl TryFrom<v1::CountOperationsRequest> for OperationsFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::CountOperationsRequest) -> Result<Self> {
        Ok(Self {
            block_number_or_hash: request
                .block_number_or_hash
                .map(|v| {
                    v.parse()
                        .map_err(|_| anyhow!("Invalid block_number_or_hash"))
                })
                .transpose()?,
            transaction_hash: request
                .transaction_hash
                .map(|hash| {
                    hash.parse()
                        .map_err(|_| anyhow!("Invalid transaction_hash"))
                })
                .transpose()?,
            sender: request
                .sender
                .map(|addr| addr.parse().map_err(|_| anyhow!("Invalid sender")))
                .transpose()?,
            entity_key: request
                .entity_key
                .map(|key| key.parse().map_err(|_| anyhow!("Invalid entity_key")))
                .transpose()?,
        })
    }
}

impl From<OperationsCount> for v1::CountOperationsResponse {
    fn from(counts: OperationsCount) -> Self {
        Self {
            create_count: counts.create_count,
            update_count: counts.update_count,
            delete_count: counts.delete_count,
            extend_count: counts.extend_count,
        }
    }
}

impl From<OperationView> for v1::Operation {
    fn from(v: OperationView) -> Self {
        let operation_type: v1::OperationType = (&v.op.operation).into();

        Self {
            entity_key: v.op.metadata.entity_key.to_string(),
            sender: v.op.metadata.sender.to_checksum(None),
            operation: operation_type.into(),
            data: v
                .op
                .operation
                .clone()
                .data()
                .map(ToHexExt::encode_hex_with_prefix),
            btl: v.op.operation.btl(),
            block_hash: v.op.metadata.block_hash.to_string(),
            block_number: v.block_number,
            transaction_hash: v.op.metadata.tx_hash.to_string(),
            index: v.op.metadata.index,
            gas_used: "0".into(),  // FIXME
            fees_paid: "0".into(), // FIXME
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

impl From<v1::EntityStatus> for EntityStatus {
    fn from(value: v1::EntityStatus) -> Self {
        match value {
            v1::EntityStatus::Active => Self::Active,
            v1::EntityStatus::Deleted => Self::Deleted,
            v1::EntityStatus::Expired => Self::Expired,
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

impl From<EntityWithExpTimestamp> for v1::EntityWithExpTimestamp {
    fn from(entity: EntityWithExpTimestamp) -> Self {
        let status: v1::EntityStatus = entity.status.into();

        Self {
            key: entity.key.to_string(),
            data: entity.data.as_ref().map(ToHexExt::encode_hex_with_prefix),
            status: status.into(),
            created_at_tx_hash: entity.created_at_tx_hash.map(|v| v.to_string()),
            last_updated_at_tx_hash: entity.last_updated_at_tx_hash.to_string(),
            expires_at_block_number: entity.expires_at_block_number,
            expires_at_timestamp: entity.expires_at_timestamp.map(|v| v.to_rfc3339()),
        }
    }
}

impl TryFrom<v1::GetEntityHistoryRequest> for EntityHistoryFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::GetEntityHistoryRequest) -> Result<Self> {
        Ok(Self {
            pagination: PaginationParams {
                page: request.page.unwrap_or(1).max(1),
                page_size: request.page_size.unwrap_or(100).clamp(1, 100),
            },
            entity_key: request
                .key
                .parse()
                .map_err(|_| anyhow!("Invalid entity_key"))?,
        })
    }
}

pub fn logic_status_to_str(s: &EntityStatus) -> String {
    match s {
        EntityStatus::Active => "ACTIVE",
        EntityStatus::Deleted => "DELETED",
        EntityStatus::Expired => "EXPIRED",
    }
    .to_owned()
}

impl From<EntityHistoryEntry> for v1::EntityHistoryEntry {
    fn from(v: EntityHistoryEntry) -> Self {
        let status: v1::EntityStatus = v.status.into();
        let operation: v1::OperationType = v.operation.into();

        Self {
            entity_key: v.entity_key.to_string(),
            block_number: v.block_number,
            block_hash: v.block_hash.to_string(),
            transaction_hash: v.transaction_hash.to_string(),
            tx_index: v.tx_index,
            op_index: v.op_index,
            block_timestamp: v.block_timestamp.to_rfc3339(),
            sender: v.sender.to_checksum(None),
            operation: operation.into(),
            data: v.data.map(|v| v.encode_hex_upper_with_prefix()),
            prev_data: v.prev_data.map(|v| v.encode_hex_upper_with_prefix()),
            status: status.into(),
            prev_status: v.prev_status.map(|s| logic_status_to_str(&s)),
            btl: v.btl.map(|v| v.to_string()),
            expires_at_block_number: v.expires_at_block_number,
            prev_expires_at_block_number: v.prev_expires_at_block_number,
            expires_at_timestamp: v.expires_at_timestamp.map(|v| v.to_rfc3339()),
            prev_expires_at_timestamp: v.prev_expires_at_timestamp.map(|v| v.to_rfc3339()),
            gas_used: "0".into(),  // FIXME
            fees_paid: "0".into(), // FIXME
        }
    }
}

impl TryFrom<v1::GetOperationRequest> for OperationFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::GetOperationRequest) -> Result<Self> {
        Ok(Self {
            tx_hash: request
                .tx_hash
                .parse()
                .map_err(|_| anyhow!("Invalid tx_hash"))?,
            op_index: request.op_index,
        })
    }
}

impl TryFrom<v1::ListEntitiesRequest> for ListEntitiesFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListEntitiesRequest) -> Result<Self> {
        let status: v1::EntityStatus = request.status.try_into()?;
        let string_annotation = match (
            request.string_annotation_key,
            request.string_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(StringAnnotation { key, value }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid string_annotation filter")),
        };
        let numeric_annotation = match (
            request.numeric_annotation_key,
            request.numeric_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(NumericAnnotation {
                key,
                value: value.parse()?,
            }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid numeric_annotation filter")),
        };
        Ok(Self {
            pagination: PaginationParams {
                page: request.page.unwrap_or(1).max(1),
                page_size: request.page_size.unwrap_or(100).clamp(1, 100),
            },
            entities_filter: EntitiesFilter {
                status: Some(status.into()),
                string_annotation,
                numeric_annotation,
            },
        })
    }
}

impl TryFrom<v1::CountEntitiesRequest> for EntitiesFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::CountEntitiesRequest) -> Result<Self> {
        let status: v1::EntityStatus = request.status.try_into()?;
        let string_annotation = match (
            request.string_annotation_key,
            request.string_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(StringAnnotation { key, value }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid string_annotation filter")),
        };
        let numeric_annotation = match (
            request.numeric_annotation_key,
            request.numeric_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(NumericAnnotation {
                key,
                value: value.parse()?,
            }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid numeric_annotation filter")),
        };
        Ok(Self {
            status: Some(status.into()),
            string_annotation,
            numeric_annotation,
        })
    }
}

impl TryFrom<v1::ListBiggestSpendersRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListBiggestSpendersRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl From<BiggestSpenders> for v1::BiggestSpender {
    fn from(value: BiggestSpenders) -> Self {
        Self {
            rank: value.rank,
            address: value.address.to_checksum(None),
            total_fees: value.total_fees.to_string(),
        }
    }
}

impl From<BlockEntitiesCount> for v1::BlockStatsCounts {
    fn from(value: BlockEntitiesCount) -> Self {
        Self {
            create_count: value.create_count,
            update_count: value.update_count,
            expire_count: value.expire_count,
            delete_count: value.delete_count,
            extend_count: value.extend_count,
        }
    }
}

impl From<BlockStorageUsage> for v1::BlockStatsStorage {
    fn from(value: BlockStorageUsage) -> Self {
        Self {
            block_bytes: value.block_bytes,
            total_bytes: value.total_bytes,
        }
    }
}

impl TryFrom<v1::ListEntitiesByBtlRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListEntitiesByBtlRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl TryFrom<v1::ListAddressByEntitiesOwnedRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListAddressByEntitiesOwnedRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl From<AddressByEntitiesOwned> for v1::AddressByEntitiesOwned {
    fn from(v: AddressByEntitiesOwned) -> Self {
        Self {
            address: v.address.to_checksum(None),
            entities_count: v.entities_count as u64,
        }
    }
}

impl TryFrom<v1::ListLargestEntitiesRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListLargestEntitiesRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl From<EntityDataSize> for v1::EntityDataSize {
    fn from(v: EntityDataSize) -> Self {
        Self {
            entity_key: v.entity_key.to_string(),
            data_size: v.data_size,
        }
    }
}
