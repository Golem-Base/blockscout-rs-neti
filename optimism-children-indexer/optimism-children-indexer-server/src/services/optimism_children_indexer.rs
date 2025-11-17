use crate::proto::{
    optimism_children_indexer_service_server::OptimismChildrenIndexerService as OptimismChildrenIndexer,
    *,
};
use optimism_children_indexer_logic::repository;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct OptimismChildrenIndexerService {
    db: Arc<DatabaseConnection>,
}

impl OptimismChildrenIndexerService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl OptimismChildrenIndexer for OptimismChildrenIndexerService {
    async fn get_deposits(
        &self,
        request: Request<PaginationRequest>,
    ) -> Result<Response<DepositsResponse>, Status> {
        let inner: PaginationRequest = request.into_inner();
        let pagination = inner.try_into().map_err(|err| {
            tracing::error!(?err, "Invalid pagination params");
            Status::invalid_argument("Invalid pagination params")
        })?;
        let (deposits, pagination) = repository::deposits::list_deposits(&*self.db, pagination)
            .await
            .map_err(|err| {
                tracing::error!(?err, "failed to query deposits");
                Status::internal("failed to query deposits")
            })?;

        let items = deposits.into_iter().map(Into::into).collect();
        let pagination = pagination.into();
        Ok(Response::new(DepositsResponse {
            items,
            next_page_params: Some(pagination),
        }))
    }
}
