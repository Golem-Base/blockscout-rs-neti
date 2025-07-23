use crate::proto::{
    golem_base_indexer_service_server::GolemBaseIndexerService as GolemBaseIndexer, *,
};
use bytes::Bytes;
use golem_base_indexer_logic::repository;
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
    ) -> Result<Response<Entity>, Status> {
        let inner = request.into_inner();

        let key = Bytes::from(inner.key).into();
        let entity = repository::entities::get_entity(&*self.db, key)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query entity");
                Status::internal("failed to query entity")
            })?
            .ok_or(Status::not_found("entity not found"))?;

        Ok(Response::new(entity.into()))
    }

    async fn get_operation(
        &self,
        request: Request<GetOperationRequest>,
    ) -> Result<Response<Operation>, Status> {
        let inner = request.into_inner();
        let tx_hash = hex::decode(inner.tx_hash)
            .map_err(|_| Status::invalid_argument("Invalid tx hash"))?
            .as_slice()
            .try_into()
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
        _request: Request<ListOperationsRequest>,
    ) -> Result<Response<ListOperationsResponse>, Status> {
        let operations = repository::operations::list_operations(&*self.db)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query operations");
                Status::internal("failed to query operations")
            })?;

        let items = operations.into_iter().map(Into::into).collect();

        Ok(Response::new(ListOperationsResponse { items }))
    }
}
