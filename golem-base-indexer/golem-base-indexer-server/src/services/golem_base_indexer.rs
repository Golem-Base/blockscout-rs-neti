use crate::proto::{
    golem_base_indexer_service_server::GolemBaseIndexerService as GolemBaseIndexer, *,
};
use golem_base_indexer_logic::{
    repository::{self},
    types::{ListOperationsFilter, OperationsFilter},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct GolemBaseIndexerService {
    db: Arc<DatabaseConnection>,
}

impl GolemBaseIndexerService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl GolemBaseIndexer for GolemBaseIndexerService {
    async fn get_entity(
        &self,
        request: Request<GetEntityRequest>,
    ) -> Result<Response<FullEntity>, Status> {
        let inner = request.into_inner();

        let key = inner
            .key
            .parse()
            .map_err(|_| Status::invalid_argument("Invalid entity key"))?;

        let entity = repository::entities::get_full_entity(&*self.db, key)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity");
                Status::internal(format!("failed to query entity - {err}"))
            })?
            .ok_or(Status::not_found("entity not found"))?;

        let string_annotations =
            repository::annotations::find_active_string_annotations(&*self.db, key)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query annotations");
                    Status::internal("failed to query annotations")
                })?;

        let numeric_annotations =
            repository::annotations::find_active_numeric_annotations(&*self.db, key)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query annotations");
                    Status::internal("failed to query annotations")
                })?;

        let entity = FullEntity::new(entity, string_annotations, numeric_annotations);

        Ok(Response::new(entity))
    }

    async fn get_operation(
        &self,
        request: Request<GetOperationRequest>,
    ) -> Result<Response<v1::EntityHistoryEntry>, Status> {
        let inner = request.into_inner();

        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entity operation filter: {err}"))
        })?;

        let operation = repository::entities::get_entity_operation(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity operation");
                Status::internal("failed to query entity operation")
            })?
            .ok_or(Status::not_found("operation not found"))?;

        Ok(Response::new(operation.into()))
    }

    async fn list_entities(
        &self,
        request: Request<ListEntitiesRequest>,
    ) -> Result<Response<ListEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            tracing::error!(?err, "Invalid filter");
            Status::invalid_argument("Invalid filter")
        })?;
        let (entities, pagination) = repository::entities::list_entities(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entities");
                Status::internal("failed to query entities")
            })?;

        let items = entities.into_iter().map(Into::into).collect();
        let pagination = pagination.into();

        Ok(Response::new(ListEntitiesResponse {
            items,
            pagination: Some(pagination),
        }))
    }

    async fn count_entities(
        &self,
        request: Request<CountEntitiesRequest>,
    ) -> Result<Response<CountEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            tracing::error!(?err, "Invalid filter");
            Status::invalid_argument("Invalid filter")
        })?;
        let count = repository::entities::count_entities(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entities");
                Status::internal("failed to query entities")
            })?;

        Ok(Response::new(CountEntitiesResponse { count }))
    }

    async fn list_operations(
        &self,
        request: Request<ListOperationsRequest>,
    ) -> Result<Response<ListOperationsResponse>, Status> {
        let inner = request.into_inner();
        let filter: ListOperationsFilter = inner
            .try_into()
            .map_err(|err| Status::invalid_argument(format!("Invalid operations filter: {err}")))?;

        let (operations, pagination) = repository::operations::list_operations(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query operations");
                Status::internal("failed to query operations")
            })?;

        let items = operations.into_iter().map(Into::into).collect();
        let pagination = pagination.into();

        Ok(Response::new(ListOperationsResponse {
            items,
            // for some reason, the protobuf forces pagination to be an option
            pagination: Some(pagination),
        }))
    }

    async fn count_operations(
        &self,
        request: Request<CountOperationsRequest>,
    ) -> Result<Response<CountOperationsResponse>, Status> {
        let inner = request.into_inner();
        let filter: OperationsFilter = inner
            .try_into()
            .map_err(|e| Status::invalid_argument(format!("Invalid operations filter: {e}")))?;

        let operations_count = repository::operations::count_operations(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to count operations");
                Status::internal("failed to count operations")
            })?;

        let operations_count = operations_count.into();

        Ok(Response::new(operations_count))
    }

    async fn get_entity_history(
        &self,
        request: Request<GetEntityHistoryRequest>,
    ) -> Result<Response<GetEntityHistoryResponse>, Status> {
        let inner = request.into_inner();

        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entity history filter: {err}"))
        })?;

        let (items, pagination) = repository::entities::get_entity_history(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity history");
                Status::internal("failed to query entity history")
            })?;

        Ok(Response::new(GetEntityHistoryResponse {
            items: items.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn address_stats(
        &self,
        request: Request<AddressStatsRequest>,
    ) -> Result<Response<AddressStatsResponse>, Status> {
        let AddressStatsRequest { address } = request.into_inner();
        let address = address.parse().map_err(|err| {
            tracing::error!(?err, "invalid address");
            Status::invalid_argument("invalid address")
        })?;

        let entities_counts = repository::address::count_entities(&*self.db, address)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to count entities");
                Status::internal("failed to count entities")
            })?;

        let tx_counts = repository::address::count_txs(&*self.db, address)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to count txs");
                Status::internal("failed to count txs")
            })?;

        let filter = OperationsFilter {
            sender: Some(address),
            ..Default::default()
        };

        let operations_counts = repository::operations::count_operations(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to count operations");
                Status::internal("failed to count operations")
            })?;

        Ok(Response::new(AddressStatsResponse {
            created_entities: entities_counts.total_entities,
            active_entities: entities_counts.active_entities,
            size_of_active_entities: entities_counts.size_of_active_entities,
            total_transactions: tx_counts.total_transactions,
            failed_transactions: tx_counts.failed_transactions,
            operations_counts: Some(operations_counts.into()),
        }))
    }

    async fn list_biggest_spenders(
        &self,
        request: Request<ListBiggestSpendersRequest>,
    ) -> Result<Response<ListBiggestSpendersResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid biggest spenders filter: {err}"))
        })?;

        let (spenders, pagination) =
            repository::transactions::list_biggest_spenders(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query biggest spenders");
                    Status::internal("failed to query biggest spenders")
                })?;

        Ok(Response::new(ListBiggestSpendersResponse {
            items: spenders.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn block_stats(
        &self,
        request: Request<BlockStatsRequest>,
    ) -> Result<Response<BlockStatsResponse>, Status> {
        let BlockStatsRequest { block_number } = request.into_inner();
        let block_number = block_number.parse().map_err(|err| {
            tracing::error!(?err, "invalid block number");
            Status::invalid_argument("invalid block number")
        })?;

        // Get entity counts
        let counts = repository::block::count_entities(&*self.db, block_number)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query block stats counts");
                Status::internal("failed to query block stats counts")
            })?;

        // Get storage usage
        let storage = repository::block::storage_usage(&*self.db, block_number)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query block storage usage");
                Status::internal("failed to query block storage usage")
            })?;

        Ok(Response::new(BlockStatsResponse {
            counts: Some(counts.into()),
            storage: Some(storage.into()),
        }))
    }

    async fn list_entities_by_btl(
        &self,
        request: Request<ListEntitiesByBtlRequest>,
    ) -> Result<Response<ListEntitiesByBtlResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities by btl filter: {err}"))
        })?;

        let (entities, pagination) = repository::entities::list_entities_by_btl(&*self.db, filter)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entities by btl");
                Status::internal("failed to query entities by btl")
            })?;

        Ok(Response::new(ListEntitiesByBtlResponse {
            items: entities.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn list_address_by_entities_owned(
        &self,
        request: Request<ListAddressByEntitiesOwnedRequest>,
    ) -> Result<Response<ListAddressByEntitiesOwnedResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities owned filter: {err}"))
        })?;

        let (entities_owned, pagination) =
            repository::entities::list_addresses_by_entities_owned(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query addresses by entities owned");
                    Status::internal("failed to query addresses by entities owned")
                })?;

        Ok(Response::new(ListAddressByEntitiesOwnedResponse {
            items: entities_owned.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn list_largest_entities(
        &self,
        request: Request<ListLargestEntitiesRequest>,
    ) -> Result<Response<ListLargestEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid largest entities filter: {err}"))
        })?;

        let (largest_entities, pagination) =
            repository::entities::list_largest_entities(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query largest entities");
                    Status::internal("failed to query largest entities")
                })?;

        Ok(Response::new(ListLargestEntitiesResponse {
            items: largest_entities.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn list_effectively_largest_entities(
        &self,
        request: Request<ListEffectivelyLargestEntitiesRequest>,
    ) -> Result<Response<ListEffectivelyLargestEntitiesResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!(
                "Invalid effectively largest entities filter: {err}"
            ))
        })?;

        let (largest_entities, pagination) =
            repository::entities::list_effectively_largest_entities(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query effectively largest entities");
                    Status::internal("failed to query effectively largest entities")
                })?;

        Ok(Response::new(ListEffectivelyLargestEntitiesResponse {
            items: largest_entities.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }

    async fn list_address_by_entities_created(
        &self,
        request: Request<ListAddressByEntitiesCreatedRequest>,
    ) -> Result<Response<ListAddressByEntitiesCreatedResponse>, Status> {
        let inner = request.into_inner();
        let filter = inner.try_into().map_err(|err| {
            Status::invalid_argument(format!("Invalid entities created filter: {err}"))
        })?;

        let (entities_created, pagination) =
            repository::operations::list_addresses_by_create_operations(&*self.db, filter)
                .await
                .map_err(|err| {
                    tracing::error!(?err, "failed to query addresses by entities created");
                    Status::internal("failed to query addresses by entities created")
                })?;

        Ok(Response::new(ListAddressByEntitiesCreatedResponse {
            items: entities_created.into_iter().map(Into::into).collect(),
            pagination: Some(pagination.into()),
        }))
    }
}
