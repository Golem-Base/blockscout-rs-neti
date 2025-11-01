use std::path::PathBuf;

use blockscout_service_launcher::{
    database::{DatabaseConnectSettings, DatabaseSettings},
    launcher::{ConfigSettings, MetricsSettings, ServerSettings},
};
use optimism_children_indexer_logic::IndexerSettings;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub server: ServerSettings,
    #[serde(default)]
    pub metrics: MetricsSettings,
    #[serde(default)]
    pub indexer: IndexerSettings,
    pub database: DatabaseSettings,
    #[serde(default = "default_swagger_path")]
    pub swagger_path: PathBuf,
}

fn default_swagger_path() -> PathBuf {
    blockscout_endpoint_swagger::default_swagger_path_from_service_name("optimism-children-indexer")
}

impl ConfigSettings for Settings {
    const SERVICE_NAME: &'static str = "OPTIMISM_CHILDREN_INDEXER";
}

impl Settings {
    pub fn default(database_url: String) -> Self {
        Self {
            server: Default::default(),
            metrics: Default::default(),
            swagger_path: default_swagger_path(),
            database: DatabaseSettings {
                connect: DatabaseConnectSettings::Url(database_url),
                connect_options: Default::default(),
                create_database: Default::default(),
                run_migrations: Default::default(),
            },
            indexer: Default::default(),
        }
    }
}
