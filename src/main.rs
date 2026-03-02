mod api;
mod client;
mod collector;
mod config;
mod output;

use clap::Parser;
use tracing::info;

static APIS_YAML: &str = include_str!("../resources/apis.yaml");

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cfg = config::Config::parse();

    // Load API definitions (embedded at compile time)
    let api_config = api::load(APIS_YAML)?;
    info!("Loaded {} API endpoints", api_config.len());

    // Build Elasticsearch client
    let es_client = client::build(&cfg)?;
    // API key is injected as a per-request Authorization header
    let api_key_header = cfg.api_key.as_deref().map(|k| format!("ApiKey {}", k));
    let auth_header = api_key_header.as_deref();

    // Query cluster info
    info!("Connecting to {}", cfg.url);
    let cluster_info = collector::get_cluster_info(&es_client, auth_header).await?;
    info!(
        "Cluster '{}' running Elasticsearch {}",
        cluster_info.cluster_name, cluster_info.version
    );

    // Resolve which endpoints apply to this cluster version
    let endpoints = collector::resolve_endpoints(&api_config, &cluster_info.version);
    info!(
        "Resolved {} endpoints for version {}",
        endpoints.len(),
        cluster_info.version
    );

    // Collect all endpoints concurrently
    let results = collector::collect_all(&es_client, endpoints, auth_header).await;

    let ok_count    = results.iter().filter(|r| r.success).count();
    let error_count = results.iter().filter(|r| !r.success && r.endpoint.show_errors).count();
    let na_count    = results.iter().filter(|r| !r.success && !r.endpoint.show_errors).count();
    info!("Collected {} ok, {} failed, {} not applicable", ok_count, error_count, na_count);

    if error_count > 0 {
        let names: Vec<&str> = results
            .iter()
            .filter(|r| !r.success && r.endpoint.show_errors)
            .map(|r| r.endpoint.name.as_str())
            .collect();
        tracing::warn!("Failed endpoints: {}", names.join(", "));
    }
    if na_count > 0 {
        let names: Vec<&str> = results
            .iter()
            .filter(|r| !r.success && !r.endpoint.show_errors)
            .map(|r| r.endpoint.name.as_str())
            .collect();
        info!(
            "Not applicable (expected non-2xx, e.g. feature unused or cluster not in required state): {}",
            names.join(", ")
        );
    }

    // Write results to temp directory and archive
    let output_mgr = output::OutputManager::new()?;
    for result in &results {
        if let Err(e) = output_mgr.write_result(result) {
            tracing::warn!("Failed to write '{}': {}", result.endpoint.name, e);
        }
    }

    let archive_path =
        output_mgr.create_archive(&cluster_info.cluster_name, &cfg.output)?;

    println!("{}", archive_path.display());
    Ok(())
}
