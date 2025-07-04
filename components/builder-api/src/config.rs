//! Configuration for a Biome Builder-API service

use crate::{bldr_core::{self,
                        config::ConfigFile},
            db::config::DataStoreCfg};
use artifactory_client::config::ArtifactoryCfg;
use github_api_client::config::GitHubCfg;

use biome_core::{crypto::keys::KeyCache,
                   package::target::{self,
                                     PackageTarget}};
use oauth_client::config::OAuth2Cfg;
use std::{env,
          error,
          fmt::{self,
                Write as _},
          io,
          net::{IpAddr,
                Ipv4Addr,
                SocketAddr,
                ToSocketAddrs},
          option::IntoIter,
          path::PathBuf};

pub trait GatewayCfg {
    /// Default number of worker threads to simultaneously handle HTTP requests.
    fn default_handler_count() -> usize { num_cpus::get() * 8 }

    /// Number of worker threads to simultaneously handle HTTP requests.
    fn handler_count(&self) -> usize { Self::default_handler_count() }

    fn listen_addr(&self) -> &IpAddr;

    fn listen_port(&self) -> u16;
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api:         ApiCfg,
    pub artifactory: ArtifactoryCfg,
    pub github:      GitHubCfg,
    pub http:        HttpCfg,
    pub oauth:       OAuth2Cfg,
    pub s3:          S3Cfg,
    pub ui:          UiCfg,
    pub memcache:    MemcacheCfg,
    pub datastore:   DataStoreCfg,
    pub provision:   ProvisionCfg,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

impl error::Error for ConfigError {}

impl ConfigFile for Config {
    type Error = ConfigError;
}

impl From<bldr_core::Error> for ConfigError {
    fn from(err: bldr_core::Error) -> ConfigError { ConfigError(format!("{:?}", err)) }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum S3Backend {
    Aws,
    Minio,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct S3Cfg {
    // These are for using S3 as the artifact storage
    pub key_id:      String,
    pub secret_key:  String,
    pub bucket_name: String,
    pub backend:     S3Backend,
    pub endpoint:    String,
}

impl Default for S3Cfg {
    fn default() -> Self {
        let endpoint =
            env::var("MINIO_ENDPOINT").unwrap_or_else(|_| String::from("http://localhost:9000"));
        let key_id = env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| String::from("depot"));
        let secret_key = env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| String::from("password"));
        let bucket_name = env::var("MINIO_BUCKET_NAME").unwrap_or_else(|_| {
                              String::from("biome-builder-artifact-store.default")
                          });

        S3Cfg { key_id,
                secret_key,
                bucket_name,
                backend: S3Backend::Minio,
                endpoint }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ApiCfg {
    pub data_path: PathBuf,
    pub log_path: PathBuf,
    /// Location of Builder encryption keys
    pub key_path: KeyCache,
    pub targets: Vec<PackageTarget>,
    #[serde(with = "deserialize_into_vec")]
    pub features_enabled: Vec<String>,
    pub private_max_age: usize,
    pub saas_bldr_url: String,
    pub suppress_autobuild_origins: Vec<String>,
    pub allowed_users_for_origin_create: Vec<String>,
    pub license_server_url: String,
    pub unrestricted_channels: Vec<String>,
    pub partially_unrestricted_channels: Vec<String>,
    pub restricted_if_present: Vec<String>,
}

mod deserialize_into_vec {
    use serde::{self,
                Deserialize,
                Deserializer};
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
        where D: Deserializer<'de>
    {
        let list = String::deserialize(deserializer)?;
        let features = list.split(',').map(|f| f.trim().to_uppercase()).collect();
        Ok(features)
    }
}

impl Default for ApiCfg {
    fn default() -> Self {
        let data_path = env::var("BLDR_DATA_DIR").map(PathBuf::from)
                                                 .unwrap_or_else(|_| PathBuf::from("files"));
        let key_path = env::var("BLDR_HAB_KEY_DIR").map(PathBuf::from)
                                                   .unwrap_or_else(|_| PathBuf::from("files"));

        ApiCfg { data_path,
                 log_path: env::temp_dir(),
                 key_path: KeyCache::new(key_path),
                 targets: vec![target::X86_64_LINUX,
                               target::X86_64_LINUX_KERNEL2,
                               target::X86_64_WINDOWS,],
                 features_enabled: vec!["jobsrv".to_string()],
                 private_max_age: 300,
                 saas_bldr_url: "https://bldr.habitat.sh".to_string(),
                 license_server_url: "http://licensing-acceptance.chef.co".to_string(),
                 suppress_autobuild_origins: vec![],
                 allowed_users_for_origin_create: vec![],
                 unrestricted_channels: vec![],
                 partially_unrestricted_channels: vec![],
                 restricted_if_present: vec![] }
    }
}

impl GatewayCfg for Config {
    fn handler_count(&self) -> usize { self.http.handler_count }

    fn listen_addr(&self) -> &IpAddr { &self.http.listen }

    fn listen_port(&self) -> u16 { self.http.port }
}

/// Public listening net address for HTTP requests
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HttpCfg {
    pub listen:        IpAddr,
    pub port:          u16,
    pub tls:           Option<TLSServerCfg>,
    pub handler_count: usize,
    pub keep_alive:    usize,
}

/// Optional TLS configuration
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TLSServerCfg {
    pub cert_path:    PathBuf,
    pub key_path:     PathBuf,
    pub ca_cert_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TLSClientCfg {
    pub cert_path:    Option<PathBuf>,
    pub key_path:     Option<PathBuf>,
    pub ca_cert_path: Option<PathBuf>,
    pub verify:       bool,
}

impl Default for TLSServerCfg {
    fn default() -> Self {
        TLSServerCfg { cert_path:    PathBuf::from("files/server.crt"),
                       key_path:     PathBuf::from("files/server.key"),
                       ca_cert_path: None, }
    }
}

impl Default for TLSClientCfg {
    fn default() -> Self {
        TLSClientCfg { cert_path:    None,
                       key_path:     None,
                       ca_cert_path: None,
                       verify:       true, }
    }
}

/// Resolves a given address string to an `IpAddr`.
///
/// - If the input is a valid IP address, it is returned as-is.
/// - If the input is a hostname, it attempts to resolve it to an IP.
/// - Returns an error if the input is neither a valid IP nor a resolvable hostname.
fn resolve_addr(addr: &str) -> Result<IpAddr, String> {
    addr.parse()
        .map_err(|_| format!("Invalid IP address or hostname: {}", addr))
        .or_else(|_| {
            (addr, 0) // Use port 0 since we only need the IP
                     .to_socket_addrs()
                     .map_err(|_| format!("Failed to resolve hostname: {}", addr))?
                     .next()
                     .map(|socket_addr| socket_addr.ip())
                     .ok_or_else(|| format!("No IP addresses found for hostname: {}", addr))
        })
}

impl Default for HttpCfg {
    fn default() -> Self {
        let listen = match env::var("BLDR_LISTEN") {
            Ok(addr) => resolve_addr(&addr).expect("Failed to resolve BLDR_LISTEN"),
            Err(_) => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), /* Use default if BLDR_LISTEN is not
                                                              * set */
        };
        let port = env::var("BLDR_PORT").ok()
                                        .and_then(|val| val.parse::<u16>().ok())
                                        .unwrap_or(9636);

        HttpCfg { listen,
                  port,
                  tls: None,
                  handler_count: Config::default_handler_count(),
                  keep_alive: 60 }
    }
}

impl ToSocketAddrs for HttpCfg {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<IntoIter<SocketAddr>> {
        match self.listen {
            IpAddr::V4(ref a) => (*a, self.port).to_socket_addrs(),
            IpAddr::V6(ref a) => (*a, self.port).to_socket_addrs(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    /// Path to UI files to host over HTTP. If not set the UI will be disabled.
    pub root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MemcacheCfgHosts {
    pub host: String,
    pub port: u16,
    pub tls:  Option<TLSClientCfg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MemcacheCfg {
    pub ttl:   u32,
    pub hosts: Vec<MemcacheCfgHosts>,
}

impl Default for MemcacheCfgHosts {
    fn default() -> Self {
        let host = env::var("MEMCACHED_HOST").unwrap_or_else(|_| String::from("localhost"));
        let port = env::var("MEMCACHED_PORT").ok()
                                             .and_then(|val| val.parse::<u16>().ok())
                                             .unwrap_or(11211);

        MemcacheCfgHosts { host,
                           port,
                           tls: None }
    }
}

impl Default for MemcacheCfg {
    fn default() -> Self {
        MemcacheCfg { hosts: vec![MemcacheCfgHosts::default()],
                      ttl:   15, }
    }
}

impl MemcacheCfg {
    pub fn memcache_hosts(&self) -> Vec<String> {
        self.hosts
            .iter()
            .map(|h| h.to_string_with_params())
            .collect()
    }
}

impl MemcacheCfgHosts {
    pub fn to_string_with_params(&self) -> String {
        let mut url = format!("{}?tcp_nodelay=true", self); // tcp_nodelay is a significant perf gain
        if let Some(tls_config) = &self.tls {
            if tls_config.ca_cert_path.is_some() {
                let _ = write!(url,
                               "&ca_path={}",
                               tls_config.ca_cert_path.as_ref().unwrap().to_string_lossy());
            }

            if tls_config.key_path.is_some() {
                let _ = write!(url,
                               "&key_path={}",
                               tls_config.key_path.as_ref().unwrap().to_string_lossy());
            }

            if tls_config.cert_path.is_some() {
                let _ = write!(url,
                               "&cert_path={}",
                               tls_config.cert_path.as_ref().unwrap().to_string_lossy());
            }

            if tls_config.verify {
                url.push_str("&verify_mode=peer");
            } else {
                url.push_str("&verify_mode=none");
            }
        }
        url
    }
}

impl fmt::Display for MemcacheCfgHosts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.tls {
            Some(_) => write!(f, "memcache+tls://{}:{}", self.host, self.port),
            None => write!(f, "memcache://{}:{}", self.host, self.port),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JobsrvCfg {
    pub host: String,
    pub port: u16,
}

impl Default for JobsrvCfg {
    fn default() -> Self {
        JobsrvCfg { host: String::from("localhost"),
                    port: 5580, }
    }
}

impl fmt::Display for JobsrvCfg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http://{}:{}", self.host, self.port)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ProvisionCfg {
    pub auto_provision_account: bool,
    pub username:               String,
    pub email:                  String,
    pub token_path:             PathBuf,
    pub origins:                Vec<String>,
    pub channels:               Vec<String>,
}

impl Default for ProvisionCfg {
    fn default() -> Self {
        let token_path = env::var("BLDR_TOKEN_DIR").map(PathBuf::from)
                                                   .unwrap_or_else(|_| env::temp_dir());

        ProvisionCfg { auto_provision_account: false,
                       username: "chef-platform".to_string(),
                       email: "chef-platform@progress.com".to_string(),
                       token_path,
                       origins: vec!["core".to_string()],
                       channels: vec!["stable".to_string()] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn config_from_file() {
        let content = r#"
        [api]
        data_path = "/hab/svc/bio-depot/data"
        log_path = "/hab/svc/bio-depot/var/log"
        key_path = "/hab/svc/bio-depot/files"
        targets = ["x86_64-linux", "x86_64-linux-kernel2", "x86_64-windows"]
        features_enabled = "foo, bar"
        private_max_age = 400
        suppress_autobuild_origins = ["origin1", "origin2"]
        allowed_users_for_origin_create = ["super1", "super2"]

        [http]
        listen = "0:0:0:0:0:0:0:1"
        port = 9636
        handler_count = 128
        keep_alive = 30

        [memcache]
        ttl = 11
        [[memcache.hosts]]
        host = "192.168.0.1"
        port = 12345

        [ui]
        root = "/some/path"

        [oauth]
        client_id = "0c2f738a7d0bd300de10"
        client_secret = "438223113eeb6e7edf2d2f91a232b72de72b9bdf"

        [s3]
        backend = "minio"
        key_id = "AWSKEYIDORSOMETHING"
        secret_key = "aW5S3c437Key7hIn817s7o7a11yN457y70Wr173L1k37h15"
        endpoint = "http://localhost:9000"
        bucket_name = "hibbity-bibbity-poopity-scoopity"

        [artifactory]
        api_url = "http://abcde"
        api_key = "secret"
        repo = "abracadabra"

        [github]
        api_url = "https://api.github.com"

        [jobsrv]
        host = "1.2.3.4"
        port = 1234

        [datastore]
        host = "1.1.1.1"
        port = 9000
        user = "test"
        database = "test"
        connection_retry_ms = 500
        connection_timeout_sec = 4800
        connection_test = true
        pool_size = 1
        ssl_mode = "verify_ca"
        ssl_root_cert = "/root_ca.crt"
        ssl_key = "/ssl.key"
        ssl_cert = "/ssl.crt"

        [eventbus]
        provider = "kafka"
        bootstrap_nodes = ["myhost:9092"]
        client_id = "http://myhost"
        "#;

        let config = Config::from_raw(content).unwrap();
        assert_eq!(config.api.data_path,
                   PathBuf::from("/hab/svc/bio-depot/data"));
        assert_eq!(config.api.log_path,
                   PathBuf::from("/hab/svc/bio-depot/var/log"));
        assert_eq!(config.api.key_path,
                   KeyCache::new("/hab/svc/bio-depot/files"));

        assert_eq!(config.api.targets.len(), 3);
        assert_eq!(config.api.targets[0], target::X86_64_LINUX);
        assert_eq!(config.api.targets[1], target::X86_64_LINUX_KERNEL2);
        assert_eq!(config.api.targets[2], target::X86_64_WINDOWS);

        assert_eq!(&config.api.allowed_users_for_origin_create,
                   &["super1".to_string(), "super2".to_string()]);

        assert_eq!(&config.api.features_enabled,
                   &["FOO".to_string(), "BAR".to_string()]);
        assert_eq!(config.api.private_max_age, 400);

        assert_eq!(&format!("{}", config.http.listen), "::1");

        assert_eq!(config.memcache.ttl, 11);
        assert_eq!(&format!("{}", config.memcache.hosts[0]),
                   "memcache://192.168.0.1:12345");

        assert_eq!(config.http.port, 9636);
        assert_eq!(config.http.handler_count, 128);
        assert_eq!(config.http.keep_alive, 30);

        assert_eq!(config.oauth.client_id, "0c2f738a7d0bd300de10");
        assert_eq!(config.oauth.client_secret,
                   "438223113eeb6e7edf2d2f91a232b72de72b9bdf");

        assert_eq!(config.github.api_url, "https://api.github.com");

        assert_eq!(config.ui.root, Some("/some/path".to_string()));

        assert_eq!(config.s3.backend, S3Backend::Minio);
        assert_eq!(config.s3.key_id, "AWSKEYIDORSOMETHING");
        assert_eq!(config.s3.secret_key,
                   "aW5S3c437Key7hIn817s7o7a11yN457y70Wr173L1k37h15");
        assert_eq!(config.s3.endpoint, "http://localhost:9000");
        assert_eq!(config.s3.bucket_name, "hibbity-bibbity-poopity-scoopity");

        assert_eq!(config.artifactory.api_url, "http://abcde");
        assert_eq!(config.artifactory.api_key, "secret");
        assert_eq!(config.artifactory.repo, "abracadabra");

        assert_eq!(config.datastore.port, 9000);
        assert_eq!(config.datastore.user, "test");
        assert_eq!(config.datastore.database, "test");
        assert_eq!(config.datastore.connection_retry_ms, 500);
        assert_eq!(config.datastore.connection_timeout_sec, 4800);
        assert!(config.datastore.connection_test);
        assert_eq!(config.datastore.pool_size, 1);
        assert_eq!(config.datastore.ssl_mode, Some("verify_ca".to_string()));
        assert_eq!(config.datastore.ssl_root_cert,
                   Some("/root_ca.crt".to_string()));
        assert_eq!(config.datastore.ssl_key, Some("/ssl.key".to_string()));
        assert_eq!(config.datastore.ssl_cert, Some("/ssl.crt".to_string()));
    }

    #[test]
    fn config_from_file_defaults() {
        let content = r#"
        [http]
        port = 9000
        "#;

        let config = Config::from_raw(content).unwrap();
        assert_eq!(config.http.port, 9000);
    }
}
