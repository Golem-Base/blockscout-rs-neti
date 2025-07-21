use crate::proto::golem_base_indexer_service_server::GolemBaseIndexerService as GolemBaseIndexer;
use crate::proto::*;
use bytes::Bytes;
use golem_base_indexer_logic::repository;
use sea_orm::{ActiveEnum, DatabaseConnection};
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

        Ok(Response::new(Entity {
            key: format!("0x{:x}", Bytes::from(entity.key)),
            data: entity.data.map(|v| format!("0x{:x}", Bytes::from(v))),
            status: entity.status.into_value(),
            created_at_tx_hash: entity
                .created_at_tx_hash
                .map(|v| format!("0x{:x}", Bytes::from(v))),
            last_updated_at_tx_hash: format!("0x{:x}", Bytes::from(entity.last_updated_at_tx_hash)),
            expires_at_block_number: entity
                .expires_at_block_number
                .try_into()
                .expect("block number is always non-negative"),
        }))
    }

    async fn get_operation(
        &self,
        request: Request<GetOperationRequest>,
    ) -> Result<Response<Operation>, Status> {
        let inner = request.into_inner();

        let tx_hash = Bytes::from(inner.tx_hash).into();
        let index = inner
            .index
            .try_into()
            .map_err(|_| Status::invalid_argument("Index out of range"))?;
        let operation = repository::operations::get_operation(&*self.db, tx_hash, index)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query operation");
                Status::internal("failed to query operation")
            })?
            .ok_or(Status::not_found("operation not found"))?;

        Ok(Response::new(Operation {
            entity_key: format!("0x{:x}", Bytes::from(operation.entity_key)),
            sender: format!("0x{:x}", Bytes::from(operation.sender)),
            operation: operation.operation.into_value(),
            data: operation.data.map(|v| format!("0x{:x}", Bytes::from(v))),
            btl: operation
                .btl
                .map(|v| v.try_into().expect("Will always fit")),
            block_hash: format!("0x{:x}", Bytes::from(operation.block_hash)),
            transaction_hash: format!("0x{:x}", Bytes::from(operation.transaction_hash)),
            index: operation
                .index
                .try_into()
                .expect("Index is always non-negative"),
        }))
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

        let items = entities
            .into_iter()
            .map(|entity| Entity {
                key: format!("0x{:x}", Bytes::from(entity.key)),
                data: entity.data.map(|v| format!("0x{:x}", Bytes::from(v))),
                status: entity.status.into_value(),
                created_at_tx_hash: entity
                    .created_at_tx_hash
                    .map(|v| format!("0x{:x}", Bytes::from(v))),
                last_updated_at_tx_hash: format!(
                    "0x{:x}",
                    Bytes::from(entity.last_updated_at_tx_hash)
                ),
                expires_at_block_number: entity
                    .expires_at_block_number
                    .try_into()
                    .expect("block number is always non-negative"),
            })
            .collect();

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

        let items = operations
            .into_iter()
            .map(|operation| Operation {
                entity_key: format!("0x{:x}", Bytes::from(operation.entity_key)),
                sender: format!("0x{:x}", Bytes::from(operation.sender)),
                operation: operation.operation.into_value(),
                data: operation.data.map(|v| format!("0x{:x}", Bytes::from(v))),
                btl: operation
                    .btl
                    .map(|v| v.try_into().expect("Will always fit")),
                block_hash: format!("0x{:x}", Bytes::from(operation.block_hash)),
                transaction_hash: format!("0x{:x}", Bytes::from(operation.transaction_hash)),
                index: operation
                    .index
                    .try_into()
                    .expect("Index is always non-negative"),
            })
            .collect();

        Ok(Response::new(ListOperationsResponse { items }))
    }
}
