use crate::proto::{
    golem_base_indexer_service_server::GolemBaseIndexerService as GolemBaseIndexer, *,
};
use golem_base_indexer_logic::{
    repository::{self},
    types::{OperationsCounterFilter, OperationsFilter},
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
    ) -> Result<Response<Operation>, Status> {
        let inner = request.into_inner();
        let tx_hash = inner
            .tx_hash
            .parse()
            .map_err(|_| Status::invalid_argument("Invalid tx hash"))?;

        let operation = repository::operations::get_operation(&*self.db, tx_hash, inner.index)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query operation");
                Status::internal("failed to query operation")
            })?
            .ok_or(Status::not_found("operation not found"))?;

        Ok(Response::new(operation.into()))
    }

    async fn list_entities(
        &self,
        _request: Request<ListEntitiesRequest>,
    ) -> Result<Response<ListEntitiesResponse>, Status> {
        let entities = repository::entities::list_entities(&*self.db)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entities");
                Status::internal("failed to query entities")
            })?;

        let items = entities.into_iter().map(Into::into).collect();

        Ok(Response::new(ListEntitiesResponse { items }))
    }

    async fn list_operations(
        &self,
        request: Request<ListOperationsRequest>,
    ) -> Result<Response<ListOperationsResponse>, Status> {
        let inner = request.into_inner();
        let filter: OperationsFilter = inner
            .try_into()
            .map_err(|err| Status::invalid_argument(format!("Invalid operations filter: {err}")))?;

        let filter = filter
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
        let filter: OperationsCounterFilter = inner
            .try_into()
            .map_err(|e| Status::invalid_argument(format!("Invalid operations filter: {e}")))?;

        let filter = filter
            .try_into()
            .map_err(|err| Status::invalid_argument(format!("Invalid operations filter: {err}")))?;

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
}
