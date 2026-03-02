use crate::api::Result;
use crate::config::Config;
use elasticsearch::Elasticsearch;
use elasticsearch::auth::Credentials;
use elasticsearch::cert::{Certificate, CertificateValidation};
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use url::Url;

pub fn build(cfg: &Config) -> Result<Elasticsearch> {
    let url = Url::parse(&cfg.url)?;
    let pool = SingleNodeConnectionPool::new(url);
    let mut builder = TransportBuilder::new(pool);

    // Authentication — API key is injected per-request as a header (see collector.rs);
    // only Basic auth is set at the transport level here.
    if let (Some(user), Some(password)) = (&cfg.user, &cfg.password) {
        builder = builder.auth(Credentials::Basic(user.clone(), password.clone()));
    }

    // TLS
    if cfg.insecure {
        builder = builder.cert_validation(CertificateValidation::None);
    } else if let Some(cacert_path) = &cfg.cacert {
        let pem = std::fs::read(cacert_path)?;
        let cert = Certificate::from_pem(&pem)?;
        builder = builder.cert_validation(CertificateValidation::Certificate(cert));
    }

    let transport = builder.build()?;
    Ok(Elasticsearch::new(transport))
}
