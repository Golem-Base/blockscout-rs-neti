use std::path::PathBuf;

use blockscout_service_launcher::{
    database::{DatabaseConnectSettings, DatabaseSettings},
    launcher::{ConfigSettings, MetricsSettings, ServerSettings},
};
use golem_base_indexer_logic::IndexerSettings;
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
    #[serde(default)]
    pub external_services: ExternalServicesSettings,
}

fn default_swagger_path() -> PathBuf {
    blockscout_endpoint_swagger::default_swagger_path_from_service_name("golem-base-indexer")
}

impl ConfigSettings for Settings {
    const SERVICE_NAME: &'static str = "GOLEM_BASE_INDEXER";
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExternalServicesSettings {
    #[serde(default)]
    pub l3_rpc_url: String,
    #[serde(default)]
    pub l2_blockscout_url: String,
    #[serde(default)]
    pub l2_batcher_address: String,
    #[serde(default)]
    pub l2_batch_inbox_address: String,
}

impl Default for ExternalServicesSettings {
    fn default() -> Self {
        Self {
            l3_rpc_url: "http://127.0.0.1:8545".to_string(),
            l2_blockscout_url: "http://127.0.0.1:4000".to_string(),
            l2_batcher_address: "0x000000000000000000000000000000000000dEaD".to_string(),
            l2_batch_inbox_address: "0x000000000000000000000000000000000000dEaD".to_string(),
        }
    }
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
            external_services: Default::default(),
        }
    }
}
