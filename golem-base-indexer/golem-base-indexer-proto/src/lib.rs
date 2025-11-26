#![allow(clippy::derive_partial_eq_without_eq)]

use const_hex::traits::ToHexExt;

use anyhow::{anyhow, Result};
use golem_base_indexer_logic::types::{
    AddressLeaderboardRanks, BlockEntitiesCount, BlockGasUsageLimitPoint, BlockOperationPoint,
    BlockStorageUsage, BlockTransactionPoint, ChartInfo, ChartPoint, ConsensusInfo,
    EntitiesAverages, EntitiesFilter, Entity, EntityDataHistogram, EntityHistoryEntry,
    EntityHistoryFilter, EntityStatus, EntityWithExpTimestamp, FullEntity,
    LeaderboardBiggestSpendersItem, LeaderboardDataOwnedItem,
    LeaderboardEffectivelyLargestEntitiesItem, LeaderboardEntitiesCreatedItem,
    LeaderboardEntitiesOwnedItem, LeaderboardLargestEntitiesItem, LeaderboardTopAccountsItem,
    ListEntitiesFilter, ListOperationsFilter, NumericAttribute, NumericAttributeWithRelations,
    OperationData, OperationFilter, OperationType, OperationView, OperationsCount,
    OperationsFilter, PaginationMetadata, PaginationParams, StringAttribute,
    StringAttributeWithRelations, Transaction,
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
        string_attributes: Vec<StringAttributeWithRelations>,
        numeric_attributes: Vec<NumericAttributeWithRelations>,
    ) -> Self {
        let status: v1::EntityStatus = entity.status.into();
        let data_size = entity.data.as_ref().map(|v| v.len() as u64);
        Self {
            key: entity.key.to_string(),
            content_type: entity.content_type,
            data: entity.data.map(|v| v.encode_hex_with_prefix()),
            data_size,
            status: status.into(),
            owner: entity.owner.map(|v| v.to_checksum(None)),
            creator: entity.creator.map(|v| v.to_checksum(None)),

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
            expires_at_timestamp_sec: entity.expires_at_timestamp_sec.map(|v| v.to_string()),
            expires_at_block_number: entity.expires_at_block_number,
            fees_paid: entity.fees_paid.to_string(),
            gas_used: entity.gas_used.to_string(),

            string_annotations: string_attributes.into_iter().map(Into::into).collect(),
            numeric_annotations: numeric_attributes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<StringAttributeWithRelations> for v1::StringAnnotationWithRelations {
    fn from(value: StringAttributeWithRelations) -> Self {
        Self {
            key: value.attribute.key,
            value: value.attribute.value,
            related_entities: value.related_entities,
        }
    }
}

impl From<NumericAttributeWithRelations> for v1::NumericAnnotationWithRelations {
    fn from(value: NumericAttributeWithRelations) -> Self {
        Self {
            key: value.attribute.key,
            value: value.attribute.value,
            related_entities: value.related_entities,
        }
    }
}

impl From<StringAttribute> for v1::StringAnnotation {
    fn from(value: StringAttribute) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<NumericAttribute> for v1::NumericAnnotation {
    fn from(value: NumericAttribute) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<v1::StringAnnotation> for StringAttribute {
    fn from(value: v1::StringAnnotation) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<v1::NumericAnnotation> for NumericAttribute {
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
            OperationData::Create(_, _, _) => Self::Create,
            OperationData::Update(_, _, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
            OperationData::ChangeOwner(_) => Self::Changeowner,
        }
    }
}
impl From<OperationData> for v1::OperationType {
    fn from(value: OperationData) -> Self {
        match value {
            OperationData::Create(_, _, _) => Self::Create,
            OperationData::Update(_, _, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
            OperationData::ChangeOwner(_) => Self::Changeowner,
        }
    }
}

impl From<v1::operation_type_filter::OperationTypeFilter> for Option<OperationType> {
    fn from(value: v1::operation_type_filter::OperationTypeFilter) -> Self {
        match value {
            v1::operation_type_filter::OperationTypeFilter::Create => Some(OperationType::Create),
            v1::operation_type_filter::OperationTypeFilter::Update => Some(OperationType::Update),
            v1::operation_type_filter::OperationTypeFilter::Delete => Some(OperationType::Delete),
            v1::operation_type_filter::OperationTypeFilter::Extend => Some(OperationType::Extend),
            v1::operation_type_filter::OperationTypeFilter::Changeowner => {
                Some(OperationType::ChangeOwner)
            }
            v1::operation_type_filter::OperationTypeFilter::All => None,
        }
    }
}

impl From<OperationType> for v1::OperationType {
    fn from(value: OperationType) -> Self {
        match value {
            OperationType::Create => Self::Create,
            OperationType::Update => Self::Update,
            OperationType::Delete => Self::Delete,
            OperationType::Extend => Self::Extend,
            OperationType::ChangeOwner => Self::Changeowner,
        }
    }
}

impl From<v1::OperationType> for OperationType {
    fn from(value: v1::OperationType) -> Self {
        match value {
            v1::OperationType::Create => Self::Create,
            v1::OperationType::Update => Self::Update,
            v1::OperationType::Delete => Self::Delete,
            v1::OperationType::Extend => Self::Extend,
            v1::OperationType::Changeowner => Self::ChangeOwner,
        }
    }
}

impl TryFrom<v1::ListOperationsRequest> for ListOperationsFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListOperationsRequest) -> Result<Self> {
        let operation_type =
            v1::operation_type_filter::OperationTypeFilter::try_from(request.operation)
                .map_err(|_| anyhow!("Invalid operation"))?
                .into();

        Ok(Self {
            pagination: PaginationParams {
                page: request.page.unwrap_or(1).max(1),
                page_size: request.page_size.unwrap_or(100).clamp(1, 100),
            },
            operation_type,
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
            changeowner_count: counts.changeowner_count,
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
            block_number: v.op.metadata.block_number,
            transaction_hash: v.op.metadata.tx_hash.to_string(),
            index: v.op.metadata.index,
            gas_used: "0".into(),  // FIXME
            fees_paid: "0".into(), // FIXME
            content_type: v.op.operation.content_type(),
            expires_at_timestamp: v.expires_at_timestamp.map(|v| v.to_rfc3339()),
            expires_at_timestamp_sec: v.expires_at_timestamp_sec,
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
            v1::EntityStatus::Active => EntityStatus::Active,
            v1::EntityStatus::Deleted => EntityStatus::Deleted,
            v1::EntityStatus::Expired => EntityStatus::Expired,
        }
    }
}

impl From<v1::entity_status_filter::EntityStatusFilter> for Option<EntityStatus> {
    fn from(value: v1::entity_status_filter::EntityStatusFilter) -> Self {
        match value {
            v1::entity_status_filter::EntityStatusFilter::Active => Some(EntityStatus::Active),
            v1::entity_status_filter::EntityStatusFilter::Deleted => Some(EntityStatus::Deleted),
            v1::entity_status_filter::EntityStatusFilter::Expired => Some(EntityStatus::Expired),
            v1::entity_status_filter::EntityStatusFilter::All => None,
        }
    }
}

impl From<Entity> for v1::Entity {
    fn from(entity: Entity) -> Self {
        let status: v1::EntityStatus = entity.status.into();

        Self {
            key: entity.key.to_string(),
            content_type: entity.content_type,
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
            content_type: entity.content_type,
            data: entity.data.as_ref().map(ToHexExt::encode_hex_with_prefix),
            status: status.into(),
            created_at_tx_hash: entity.created_at_tx_hash.map(|v| v.to_string()),
            last_updated_at_tx_hash: entity.last_updated_at_tx_hash.to_string(),
            expires_at_block_number: entity.expires_at_block_number,
            expires_at_timestamp: entity.expires_at_timestamp.map(|v| v.to_rfc3339()),
            expires_at_timestamp_sec: entity.expires_at_timestamp_sec,
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
            expires_at_timestamp_sec: v.expires_at_timestamp_sec,
            prev_expires_at_timestamp: v.prev_expires_at_timestamp.map(|v| v.to_rfc3339()),
            prev_expires_at_timestamp_sec: v.prev_expires_at_timestamp_sec,
            gas_used: "0".into(),  // FIXME
            fees_paid: "0".into(), // FIXME
            prev_owner: v.prev_owner.map(|v| v.to_checksum(None)),
            owner: v.owner.map(|v| v.to_checksum(None)),
            content_type: v.content_type,
            prev_content_type: v.prev_content_type,
            cost: v.cost.map(|cost_u256| cost_u256.to_string()),
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
        let status: v1::entity_status_filter::EntityStatusFilter = request.status.try_into()?;
        let string_attribute = match (
            request.string_annotation_key,
            request.string_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(StringAttribute { key, value }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid string_attribute filter")),
        };
        let numeric_attribute = match (
            request.numeric_annotation_key,
            request.numeric_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(NumericAttribute {
                key,
                value: value.parse()?,
            }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid numeric_attribute filter")),
        };
        Ok(Self {
            pagination: PaginationParams {
                page: request.page.unwrap_or(1).max(1),
                page_size: request.page_size.unwrap_or(100).clamp(1, 100),
            },
            entities_filter: EntitiesFilter {
                status: status.into(),
                string_attribute,
                numeric_attribute,
                owner: request.owner.map(|v| v.parse()).transpose()?,
            },
        })
    }
}

impl TryFrom<v1::CountEntitiesRequest> for EntitiesFilter {
    type Error = anyhow::Error;

    fn try_from(request: v1::CountEntitiesRequest) -> Result<Self> {
        let status: v1::entity_status_filter::EntityStatusFilter = request.status.try_into()?;
        let string_attribute = match (
            request.string_annotation_key,
            request.string_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(StringAttribute { key, value }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid string_attribute filter")),
        };
        let numeric_attribute = match (
            request.numeric_annotation_key,
            request.numeric_annotation_value,
        ) {
            (Some(key), Some(value)) => Some(NumericAttribute {
                key,
                value: value.parse()?,
            }),
            (None, None) => None,
            _ => return Err(anyhow!("Invalid numeric_attribute filter")),
        };
        Ok(Self {
            status: status.into(),
            string_attribute,
            numeric_attribute,
            owner: request.owner.map(|v| v.parse()).transpose()?,
        })
    }
}

impl TryFrom<v1::PaginationRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::PaginationRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
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
            changeowner_count: value.changeowner_count,
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

impl TryFrom<v1::ListCustomContractTransactionsRequest> for PaginationParams {
    type Error = anyhow::Error;

    fn try_from(request: v1::ListCustomContractTransactionsRequest) -> Result<Self> {
        Ok(Self {
            page: request.page.unwrap_or(1).max(1),
            page_size: request.page_size.unwrap_or(100).clamp(1, 100),
        })
    }
}

impl From<Transaction> for v1::Transaction {
    fn from(v: Transaction) -> Self {
        Self {
            hash: v.hash.to_string(),
            from_address_hash: v.from_address_hash.to_checksum(None),
            to_address_hash: v.to_address_hash.map(|v| v.to_checksum(None)),
            status: v.status.map(|v| v as u64),
            block_hash: v.block_hash.map(|v| v.to_string()),
            block_number: v.block_number,
            block_consensus: v.block_consensus,
            index: v.index,
            cumulative_gas_used: v.cumulative_gas_used.map(|v| v.to_string()),
            gas_price: v.gas_price.map(|v| v.to_string()),
            block_timestamp: v.block_timestamp.map(|v| v.to_rfc3339()),
            error: v.error,
            value: v.value.to_string(),
            input: v.input.encode_hex_with_prefix(),
            created_contract_address_hash: v
                .created_contract_address_hash
                .map(|v| v.to_checksum(None)),
            r#type: v.r#type.map(|v| v as u64),
            l1_transaction_origin: v.l1_transaction_origin.map(|v| v.to_checksum(None)),
            l1_block_number: v.l1_block_number,
        }
    }
}

impl From<LeaderboardTopAccountsItem> for v1::LeaderboardTopAccountsItem {
    fn from(v: LeaderboardTopAccountsItem) -> Self {
        Self {
            rank: v.rank,
            address: v.address.to_checksum(None),
            balance: v.balance.to_string(),
            tx_count: v.tx_count.to_string(),
        }
    }
}

impl From<AddressLeaderboardRanks> for v1::AddressLeaderboardRanksResponse {
    fn from(ranks: AddressLeaderboardRanks) -> Self {
        Self {
            biggest_spenders: ranks.biggest_spenders,
            entities_created: ranks.entities_created,
            entities_owned: ranks.entities_owned,
            data_owned: ranks.data_owned,
            top_accounts: ranks.top_accounts,
        }
    }
}

// Charts
impl From<ChartInfo> for v1::ChartInfo {
    fn from(v: ChartInfo) -> Self {
        Self {
            id: v.id,
            title: v.title,
            description: v.description,
        }
    }
}

impl From<ChartPoint> for v1::ChartPoint {
    fn from(v: ChartPoint) -> Self {
        Self {
            date: v.date,
            date_to: v.date_to,
            value: v.value,
        }
    }
}

impl From<BlockTransactionPoint> for v1::BlockTransactionPoint {
    fn from(v: BlockTransactionPoint) -> Self {
        Self {
            block_number: v.block_number,
            tx_count: v.tx_count,
        }
    }
}

impl From<BlockOperationPoint> for v1::BlockOperationPoint {
    fn from(v: BlockOperationPoint) -> Self {
        Self {
            block_number: v.block_number,
            create_count: v.create_count,
            update_count: v.update_count,
            delete_count: v.delete_count,
            extend_count: v.extend_count,
            changeowner_count: v.changeowner_count,
        }
    }
}

impl From<BlockGasUsageLimitPoint> for v1::ChartBlockGasUsageLimitPoint {
    fn from(v: BlockGasUsageLimitPoint) -> Self {
        Self {
            block_number: v.block_number,
            gas_used: v.gas_used,
            gas_limit: v.gas_limit,
            gas_usage_percentage: v.gas_usage_percentage.to_string(),
        }
    }
}

// Leaderboards
impl From<LeaderboardBiggestSpendersItem> for v1::LeaderboardBiggestSpendersItem {
    fn from(v: LeaderboardBiggestSpendersItem) -> Self {
        Self {
            rank: v.rank,
            address: v.address.to_checksum(None),
            total_fees: v.total_fees.to_string(),
        }
    }
}

impl From<LeaderboardEntitiesCreatedItem> for v1::LeaderboardEntitiesCreatedItem {
    fn from(v: LeaderboardEntitiesCreatedItem) -> Self {
        Self {
            rank: v.rank,
            address: v.address.to_checksum(None),
            entities_created_count: v.entities_created_count,
        }
    }
}

impl From<LeaderboardEntitiesOwnedItem> for v1::LeaderboardEntitiesOwnedItem {
    fn from(v: LeaderboardEntitiesOwnedItem) -> Self {
        Self {
            rank: v.rank,
            address: v.address.to_checksum(None),
            entities_count: v.entities_count,
        }
    }
}

impl From<LeaderboardDataOwnedItem> for v1::LeaderboardDataOwnedItem {
    fn from(v: LeaderboardDataOwnedItem) -> Self {
        Self {
            rank: v.rank,
            address: v.address.to_checksum(None),
            data_size: v.data_size,
        }
    }
}

impl From<LeaderboardLargestEntitiesItem> for v1::LeaderboardLargestEntitiesItem {
    fn from(v: LeaderboardLargestEntitiesItem) -> Self {
        Self {
            rank: v.rank,
            entity_key: v.entity_key.to_string(),
            data_size: v.data_size,
        }
    }
}

impl From<LeaderboardEffectivelyLargestEntitiesItem>
    for v1::LeaderboardEffectivelyLargestEntitiesItem
{
    fn from(v: LeaderboardEffectivelyLargestEntitiesItem) -> Self {
        Self {
            rank: v.rank,
            entity_key: v.entity_key.to_string(),
            data_size: v.data_size,
            lifespan: v.lifespan,
        }
    }
}

impl From<EntityDataHistogram> for v1::EntityDataHistogram {
    fn from(v: EntityDataHistogram) -> Self {
        Self {
            bucket: v.bucket,
            bin_start: v.bin_start,
            bin_end: v.bin_end,
            count: v.count,
        }
    }
}

impl From<ConsensusInfo> for v1::ConsensusInfoResponse {
    fn from(v: ConsensusInfo) -> Self {
        Self {
            unsafe_block_number: v.blocks.latest.block_number,
            unsafe_block_timestamp: v.blocks.latest.timestamp.to_string(),
            safe_block_number: v.blocks.safe.block_number,
            safe_block_timestamp: v.blocks.safe.timestamp.to_string(),
            finalized_block_number: v.blocks.finalized.block_number,
            finalized_block_timestamp: v.blocks.finalized.timestamp.to_string(),

            rollup_gas_used: v.gas.gas_used.to_string(),
            rollup_gas_price: v.gas.gas_price.to_string(),
            rollup_transaction_fee: v.gas.transaction_fee.to_string(),
        }
    }
}

impl From<EntitiesAverages> for v1::EntitiesAveragesResponse {
    fn from(v: EntitiesAverages) -> Self {
        Self {
            average_entity_size: v.average_entitiy_size,
            average_entity_btl: v.average_entity_btl,
        }
    }
}
