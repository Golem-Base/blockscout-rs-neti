use crate::{
    proto::{
        health_actix::route_health, health_server::HealthServer,
        optimism_children_indexer_service_actix::route_optimism_children_indexer_service,
    },
    services::{HealthService, OptimismChildrenIndexerService},
    settings::Settings,
};
use anyhow::Result;
use blockscout_endpoint_swagger::route_swagger;
use blockscout_service_launcher::{launcher, launcher::LaunchSettings};
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use std::{path::PathBuf, sync::Arc};

const SERVICE_NAME: &str = "optimism_children_indexer_server";

#[derive(Clone)]
struct Router {
    optimism_children_indexer: Arc<OptimismChildrenIndexerService>,
    health: Arc<HealthService>,
    swagger_path: PathBuf,
}

impl Router {
    pub fn grpc_router(&self) -> tonic::transport::server::Router {
        tonic::transport::Server::builder().add_service(HealthServer::from_arc(self.health.clone()))
    }
}

#[derive(OpenApi)]
#[openapi()]
struct ApiDoc;

impl launcher::HttpRouter for Router {
    fn register_routes(&self, service_config: &mut actix_web::web::ServiceConfig) {
        service_config.configure(|config| route_health(config, self.health.clone()));
        service_config.configure(|config| {
            route_optimism_children_indexer_service(config, self.optimism_children_indexer.clone())
        });
        service_config.configure(|config| {
            route_swagger(
                config,
                self.swagger_path.clone(),
                "/api/v1/docs/swagger.yaml",
            )
        });
        service_config.configure(|config| {
            config.service(
                SwaggerUi::new("/docs/{_:.*}").url("/api/v1/docs/swagger.yaml", ApiDoc::openapi()),
            );
        });
    }
}

pub async fn run(
    db_connection: Arc<DatabaseConnection>,
    settings: Settings,
) -> Result<(), anyhow::Error> {
    let health = Arc::new(HealthService::default());

    let optimism_children_indexer = Arc::new(OptimismChildrenIndexerService::new(db_connection));

    let router = Router {
        optimism_children_indexer,
        health,
        swagger_path: settings.swagger_path,
    };

    let grpc_router = router.grpc_router();
    let http_router = router;

    let launch_settings = LaunchSettings {
        service_name: SERVICE_NAME.to_string(),
        server: settings.server,
        metrics: settings.metrics,
        graceful_shutdown: Default::default(),
    };

    launcher::launch(launch_settings, http_router, grpc_router).await
}
