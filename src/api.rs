use indexmap::IndexMap;
use serde::Deserialize;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiEndpoint {
    pub extension: Option<String>,
    pub subdir: Option<String>,
    pub retry: Option<bool>,
    pub tags: Option<String>,
    pub show_errors: Option<bool>,
    pub versions: IndexMap<String, String>,
}

pub type ApiConfig = IndexMap<String, ApiEndpoint>;

pub fn load(yaml: &str) -> Result<ApiConfig> {
    let config: ApiConfig = serde_yaml::from_str(yaml)?;
    Ok(config)
}
