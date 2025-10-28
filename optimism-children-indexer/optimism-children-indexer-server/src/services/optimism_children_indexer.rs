use crate::proto::{
    optimism_children_indexer_service_server::OptimismChildrenIndexerService as OptimismChildrenIndexer,
    *,
};
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
    async fn placeholder(&self, _req: Request<Empty>) -> Result<Response<Empty>, Status> {
        let _db = self.db.clone();
        todo!()
    }
}
