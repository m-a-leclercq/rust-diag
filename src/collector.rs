use crate::api::ApiConfig;
use elasticsearch::Elasticsearch;
use elasticsearch::http::Method;
use elasticsearch::http::headers::{HeaderMap, HeaderValue, AUTHORIZATION};
use semver::{Version, VersionReq};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct ResolvedEndpoint {
    pub name: String,
    pub url_path: String,
    pub extension: String,
    pub subdir: Option<String>,
    pub retry: bool,
    pub show_errors: bool,
}

pub struct CollectionResult {
    pub endpoint: ResolvedEndpoint,
    pub body: String,
    pub success: bool,
}

/// Cluster info from GET /
pub struct ClusterInfo {
    pub version: Version,
    pub cluster_name: String,
}

/// Build a HeaderMap, injecting `Authorization: ApiKey <token>` when provided.
fn build_headers(auth_header: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(auth) = auth_header {
        if let Ok(val) = HeaderValue::from_str(auth) {
            headers.insert(AUTHORIZATION, val);
        }
    }
    headers
}

pub async fn get_cluster_info(
    client: &Elasticsearch,
    auth_header: Option<&str>,
) -> Result<ClusterInfo> {
    let response = client
        .transport()
        .send(
            Method::Get,
            "/",
            build_headers(auth_header),
            None::<&()>,
            None::<&str>,
            Some(Duration::from_secs(30)),
        )
        .await?;

    let status = response.status_code();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GET / returned HTTP {}: {}", status.as_u16(), body).into());
    }

    let body: serde_json::Value = response.json().await?;

    let raw_ver = body["version"]["number"]
        .as_str()
        .ok_or("missing version.number in GET / response")?;

    let cluster_name = body["cluster_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    // Strip pre-release suffix (e.g. "7.17.0-SNAPSHOT" -> "7.17.0")
    let ver_str = raw_ver.split('-').next().unwrap_or(raw_ver);
    let version = Version::parse(ver_str)?;

    Ok(ClusterInfo {
        version,
        cluster_name,
    })
}

/// Normalize a version requirement string so that multiple comparators separated
/// by spaces are joined by commas (both forms are accepted by the semver crate,
/// but this makes the intent explicit).
fn normalize_req(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' ' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if next == '<' || next == '>' || next == '=' || next == '~' || next == '^' {
                result.push(',');
                result.push(' ');
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// For each API entry in the config, pick the first version rule that matches.
pub fn resolve_endpoints(config: &ApiConfig, version: &Version) -> Vec<ResolvedEndpoint> {
    let mut endpoints = Vec::new();
    for (name, endpoint) in config {
        let mut matched_path: Option<&str> = None;
        for (rule, path) in &endpoint.versions {
            let normalized = normalize_req(rule);
            match VersionReq::parse(&normalized) {
                Ok(req) if req.matches(version) => {
                    matched_path = Some(path.as_str());
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Could not parse version requirement '{}': {}", rule, e);
                }
            }
        }
        if let Some(path) = matched_path {
            endpoints.push(ResolvedEndpoint {
                name: name.clone(),
                url_path: path.to_string(),
                extension: endpoint
                    .extension
                    .clone()
                    .unwrap_or_else(|| ".json".to_string()),
                subdir: endpoint.subdir.clone(),
                retry: endpoint.retry.unwrap_or(false),
                show_errors: endpoint.show_errors.unwrap_or(true),
            });
        } else {
            debug!(
                "No matching version rule for '{}' at version {}",
                name, version
            );
        }
    }
    endpoints
}

pub async fn fetch_endpoint(
    client: &Elasticsearch,
    ep: &ResolvedEndpoint,
    auth_header: Option<&str>,
) -> CollectionResult {
    let max_attempts: u32 = if ep.retry { 4 } else { 1 };
    let mut last_result = CollectionResult {
        endpoint: ep.clone(),
        body: String::new(),
        success: false,
    };

    for attempt in 0..max_attempts {
        if attempt > 0 {
            sleep(Duration::from_secs(1u64 << (attempt - 1))).await;
        }

        let send_result = client
            .transport()
            .send(
                Method::Get,
                &ep.url_path,
                build_headers(auth_header),
                None::<&()>,
                None::<&str>,
                Some(Duration::from_secs(30)),
            )
            .await;

        match send_result {
            Ok(resp) => {
                let status = resp.status_code();
                let success = status.is_success();
                let body = resp.text().await.unwrap_or_else(|e| {
                    warn!("Failed to read body for '{}': {}", ep.name, e);
                    String::new()
                });
                last_result = CollectionResult {
                    endpoint: ep.clone(),
                    body,
                    success,
                };
                if !success {
                    if ep.show_errors {
                        warn!(
                            "Non-2xx response for '{}': HTTP {} (attempt {})",
                            ep.name,
                            status.as_u16(),
                            attempt + 1
                        );
                    } else {
                        debug!(
                            "Non-2xx response for '{}': HTTP {} (attempt {}) [errors suppressed]",
                            ep.name,
                            status.as_u16(),
                            attempt + 1
                        );
                    }
                }
                if success || !ep.retry {
                    return last_result;
                }
            }
            Err(e) => {
                warn!(
                    "Transport error for '{}' (attempt {}): {}",
                    ep.name,
                    attempt + 1,
                    e
                );
                last_result = CollectionResult {
                    endpoint: ep.clone(),
                    body: String::new(),
                    success: false,
                };
            }
        }
    }

    last_result
}

pub async fn collect_all(
    client: &Elasticsearch,
    endpoints: Vec<ResolvedEndpoint>,
    auth_header: Option<&str>,
) -> Vec<CollectionResult> {
    let futures: Vec<_> = endpoints
        .iter()
        .map(|ep| fetch_endpoint(client, ep, auth_header))
        .collect();
    futures::future::join_all(futures).await
}
