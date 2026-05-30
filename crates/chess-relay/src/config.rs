//! Relay server configuration with precedence
//! **config file > environment > compiled default**.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Compiled-in defaults (overridable via env/file).
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 9443;
pub const DEFAULT_CERT: &str = "certs/relay.crt";
pub const DEFAULT_KEY: &str = "certs/relay.key";

/// Fully resolved server configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub cert: PathBuf,
    pub key: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            cert: PathBuf::from(DEFAULT_CERT),
            key: PathBuf::from(DEFAULT_KEY),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    host: Option<String>,
    port: Option<u16>,
    cert: Option<PathBuf>,
    key: Option<PathBuf>,
}

impl Config {
    /// Resolve config: compiled defaults, overridden by environment, then by
    /// the config file (highest precedence). `path` selects the file; when
    /// `None`, `relay.toml` in the working directory is used if present.
    pub fn load(path: Option<&Path>) -> anyhow::Result<Config> {
        let mut cfg = Config::default();

        // Environment overrides compiled defaults.
        if let Ok(v) = std::env::var("CHESS_RELAY_HOST") {
            if !v.is_empty() {
                cfg.host = v;
            }
        }
        if let Ok(v) = std::env::var("CHESS_RELAY_PORT") {
            if let Ok(p) = v.parse() {
                cfg.port = p;
            }
        }
        if let Ok(v) = std::env::var("CHESS_RELAY_CERT") {
            if !v.is_empty() {
                cfg.cert = PathBuf::from(v);
            }
        }
        if let Ok(v) = std::env::var("CHESS_RELAY_KEY") {
            if !v.is_empty() {
                cfg.key = PathBuf::from(v);
            }
        }

        // Config file overrides everything else.
        let file_path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("relay.toml"));
        let explicit = path.is_some();
        match std::fs::read_to_string(&file_path) {
            Ok(text) => {
                let fc: FileConfig = toml::from_str(&text)?;
                if let Some(v) = fc.host {
                    cfg.host = v;
                }
                if let Some(v) = fc.port {
                    cfg.port = v;
                }
                if let Some(v) = fc.cert {
                    cfg.cert = v;
                }
                if let Some(v) = fc.key {
                    cfg.key = v;
                }
            }
            Err(e) if explicit => {
                // A path was given explicitly but cannot be read: surface it.
                return Err(anyhow::anyhow!(
                    "cannot read config file {}: {e}",
                    file_path.display()
                ));
            }
            Err(_) => { /* default relay.toml absent: fine, use env/defaults */ }
        }

        Ok(cfg)
    }

    /// The `host:port` string to bind/listen on (resolved by the listener).
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
