use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rust-diags", about = "Collect Elasticsearch diagnostics")]
pub struct Config {
    /// Elasticsearch base URL (e.g. https://localhost:9200)
    #[arg(long)]
    pub url: String,

    /// Username for Basic auth
    #[arg(long)]
    pub user: Option<String>,

    /// Password for Basic auth
    #[arg(long)]
    pub password: Option<String>,

    /// API key — base64-encoded `id:api_key` token as shown in Kibana / the Create API Key response
    #[arg(long, conflicts_with_all = ["user", "password"])]
    pub api_key: Option<String>,

    /// Path to CA certificate PEM file
    #[arg(long)]
    pub cacert: Option<PathBuf>,

    /// Skip TLS certificate verification
    #[arg(long, default_value_t = false)]
    pub insecure: bool,

    /// Directory for the output .tar.gz (default: current directory)
    #[arg(long, default_value = ".")]
    pub output: PathBuf,
}
