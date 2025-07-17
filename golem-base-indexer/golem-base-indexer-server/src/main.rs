use blockscout_service_launcher::launcher::ConfigSettings;
use golem_base_indexer_server::{run, Settings};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::build().expect("failed to read config");
    run(settings).await
}
