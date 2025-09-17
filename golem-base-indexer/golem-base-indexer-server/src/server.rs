use crate::{
    proto::{
        golem_base_indexer_service_actix::route_golem_base_indexer_service,
        health_actix::route_health, health_server::HealthServer,
    },
    services::{GolemBaseIndexerService, HealthService, UpdateTimeseriesService},
    settings::Settings,
};
use blockscout_endpoint_swagger::route_swagger;
use blockscout_service_launcher::{launcher, launcher::LaunchSettings};
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use std::{path::PathBuf, sync::Arc};

const SERVICE_NAME: &str = "golem_base_indexer_server";

#[derive(Clone)]
struct Router {
    golem_base_indexer: Arc<GolemBaseIndexerService>,
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
            route_golem_base_indexer_service(config, self.golem_base_indexer.clone())
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

    // TODO: init services here

    let update_timeseries = UpdateTimeseriesService::new(Arc::clone(&db_connection));
    update_timeseries.spawn_periodic_task(1800); // 1800 seconds = 30 minutes

    let golem_base_indexer = Arc::new(GolemBaseIndexerService::new(db_connection));

    let router = Router {
        golem_base_indexer,
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
