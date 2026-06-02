//! Relay (internet) transport: WebSocket-over-TLS tunnels brokered by a
//! [`chess-relay`] server.
//!
//! The relay server only pairs two clients by **room number** and forwards
//! opaque bytes between them; it never learns the room password or sees any
//! game data (the inner [`crate::crypto`] AEAD layer is end-to-end). The only
//! cleartext exchanged with the server is the control protocol below plus the
//! per-room `salt` (which is not secret).
//!
//! Control flow:
//! 1. Host connects, sends [`ControlMsg::Create`] with a random salt; the
//!    server replies [`ControlMsg::Created`] with a fresh room number.
//! 2. Guest connects, sends [`ControlMsg::Join`]; the server replies
//!    [`ControlMsg::Joined`] (echoing the host's salt) and pairs them.
//! 3. The server sends the host [`ControlMsg::PeerJoined`]; from then on it
//!    relays every binary frame verbatim. The normal end-to-end handshake
//!    (see [`crate::session`]) then proceeds over the tunnel.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::connection::{Connection, WsClient};
use crate::crypto::Cipher;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// Compiled-in default relay host. Overridable via config file or environment.
pub const DEFAULT_RELAY_HOST: &str = "relay.xiangqi.example.com";
/// Compiled-in default relay port (shared with HTTPS-style TLS).
pub const DEFAULT_RELAY_PORT: u16 = 9443;

/// Control messages between a client and the relay server (cleartext over TLS).
///
/// Shared by the client and the server crate. Carries no password or game data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMsg {
    /// Host -> server: create a room, supplying a base64 per-room salt.
    Create { salt: String },
    /// Server -> host: room created with this number.
    Created { room: String },
    /// Guest -> server: join an existing room by number.
    Join { room: String },
    /// Server -> guest: joined; echoes the host's base64 salt.
    Joined { salt: String },
    /// Server -> host: the guest has connected; relaying begins.
    PeerJoined,
    /// Server -> host: the previously connected guest has disconnected.
    /// The host should remain on this socket and wait for the next guest.
    PeerLeft,
    /// Server -> client: a fatal error (e.g. unknown room).
    Error { msg: String },
}

#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("tls error: {0}")]
    Tls(String),
    #[error("websocket error: {0}")]
    Ws(String),
    #[error("relay protocol error: {0}")]
    Protocol(String),
    #[error("relay server: {0}")]
    Server(String),
}

/// Client-side relay configuration, resolved with precedence
/// **config file > environment > compiled default**.
#[derive(Debug, Clone)]
pub struct RelayClientConfig {
    /// Server hostname (must match the TLS certificate in production).
    pub host: String,
    /// Server TLS port.
    pub port: u16,
    /// Optional PEM file of extra trust anchors (e.g. a dev self-signed cert).
    pub ca_path: Option<PathBuf>,
    /// Skip certificate verification entirely (development only!).
    pub insecure: bool,
}

impl Default for RelayClientConfig {
    fn default() -> Self {
        RelayClientConfig {
            host: DEFAULT_RELAY_HOST.to_string(),
            port: DEFAULT_RELAY_PORT,
            ca_path: None,
            insecure: false,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileClientConfig {
    host: Option<String>,
    port: Option<u16>,
    ca_path: Option<PathBuf>,
    insecure: Option<bool>,
}

impl RelayClientConfig {
    /// Resolve configuration: start from compiled defaults, override with
    /// environment variables, then override with the config file (highest).
    ///
    /// `path` selects the config file; when `None`, `client.toml` in the
    /// working directory is used if present.
    pub fn load(path: Option<&Path>) -> RelayClientConfig {
        let mut cfg = RelayClientConfig::default();

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
        if let Ok(v) = std::env::var("CHESS_RELAY_CA") {
            if !v.is_empty() {
                cfg.ca_path = Some(PathBuf::from(v));
            }
        }
        if let Ok(v) = std::env::var("CHESS_RELAY_INSECURE") {
            cfg.insecure = matches!(v.as_str(), "1" | "true" | "yes" | "on");
        }

        // Config file overrides everything else.
        let file_path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("client.toml"));
        if let Ok(text) = std::fs::read_to_string(&file_path) {
            match toml::from_str::<FileClientConfig>(&text) {
                Ok(fc) => {
                    if let Some(v) = fc.host {
                        cfg.host = v;
                    }
                    if let Some(v) = fc.port {
                        cfg.port = v;
                    }
                    if fc.ca_path.is_some() {
                        cfg.ca_path = fc.ca_path;
                    }
                    if let Some(v) = fc.insecure {
                        cfg.insecure = v;
                    }
                }
                Err(e) => {
                    tracing::warn!(file = %file_path.display(), error = %e, "ignoring malformed relay config file");
                }
            }
        }

        cfg
    }

    /// Build a rustls client config using the **ring** provider (no OpenSSL).
    fn build_tls_config(&self) -> Result<rustls::ClientConfig, RelayError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let builder = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| RelayError::Tls(e.to_string()))?;

        if self.insecure {
            return Ok(builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(danger::NoVerify))
                .with_no_client_auth());
        }

        let mut roots = rustls::RootCertStore::empty();
        if let Some(ca) = &self.ca_path {
            let pem = std::fs::read(ca)?;
            for cert in rustls_pemfile::certs(&mut &pem[..]) {
                let cert = cert.map_err(|e| RelayError::Tls(e.to_string()))?;
                roots
                    .add(cert)
                    .map_err(|e| RelayError::Tls(e.to_string()))?;
            }
        } else {
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
        Ok(builder.with_root_certificates(roots).with_no_client_auth())
    }

    /// Open a TLS + WebSocket tunnel to the relay server.
    async fn ws_connect(&self) -> Result<WsClient, RelayError> {
        let tls_cfg = self.build_tls_config()?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(tls_cfg));
        let domain = rustls::pki_types::ServerName::try_from(self.host.clone())
            .map_err(|e| RelayError::Tls(format!("invalid server name: {e}")))?;

        let tcp = TcpStream::connect((self.host.as_str(), self.port)).await?;
        tcp.set_nodelay(true).ok();
        let tls = connector.connect(domain, tcp).await?;

        let url = format!("wss://{}:{}/", self.host, self.port);
        let (ws, _resp) = tokio_tungstenite::client_async(url, tls)
            .await
            .map_err(|e| RelayError::Ws(e.to_string()))?;
        Ok(ws)
    }
}

async fn send_control(ws: &mut WsClient, msg: &ControlMsg) -> Result<(), RelayError> {
    let txt = serde_json::to_string(msg).map_err(|e| RelayError::Protocol(e.to_string()))?;
    ws.send(WsMessage::Text(txt.into()))
        .await
        .map_err(|e| RelayError::Ws(e.to_string()))
}

async fn recv_control(ws: &mut WsClient) -> Result<ControlMsg, RelayError> {
    loop {
        match ws.next().await {
            Some(Ok(m)) if m.is_text() => {
                let txt = m.into_text().map_err(|e| RelayError::Ws(e.to_string()))?;
                return serde_json::from_str(txt.as_str())
                    .map_err(|e| RelayError::Protocol(e.to_string()));
            }
            Some(Ok(m)) if m.is_close() => {
                return Err(RelayError::Ws("server closed during handshake".into()))
            }
            // Ignore ping/pong/binary noise during the control handshake.
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(RelayError::Ws(e.to_string())),
            None => return Err(RelayError::Ws("server closed during handshake".into())),
        }
    }
}

/// A relayed room that has been created but is still waiting for a guest.
///
/// The [`room`](Self::room) number is available immediately so the host can
/// share it; call [`await_guest`](Self::await_guest) to block until a guest
/// joins and obtain the tunnelled [`Connection`].
pub struct PendingRelayHost {
    ws: WsClient,
    cipher: Cipher,
    /// The assigned room number to share with the guest.
    pub room: String,
}

impl PendingRelayHost {
    /// Block until a guest joins, returning the established [`Connection`]
    /// (the end-to-end handshake has not yet run).
    pub async fn await_guest(self) -> Result<Connection, RelayError> {
        let mut ws = self.ws;
        match recv_control(&mut ws).await? {
            ControlMsg::PeerJoined => {}
            ControlMsg::Error { msg } => return Err(RelayError::Server(msg)),
            other => {
                return Err(RelayError::Protocol(format!(
                    "expected peer_joined, got {other:?}"
                )))
            }
        }
        Ok(Connection::from_ws(ws, self.cipher))
    }
}

/// Create a relayed room. Returns immediately once the server assigns a room
/// number, *before* any guest joins, so the host can display/share the number.
pub async fn relay_create(
    cfg: &RelayClientConfig,
    password: &str,
) -> Result<PendingRelayHost, RelayError> {
    let mut ws = cfg.ws_connect().await?;
    let salt = crate::crypto::random_salt();

    send_control(
        &mut ws,
        &ControlMsg::Create {
            salt: B64.encode(salt),
        },
    )
    .await?;
    let room = match recv_control(&mut ws).await? {
        ControlMsg::Created { room } => room,
        ControlMsg::Error { msg } => return Err(RelayError::Server(msg)),
        other => {
            return Err(RelayError::Protocol(format!(
                "expected created, got {other:?}"
            )))
        }
    };

    let cipher = Cipher::from_password_salt(password, &salt);
    Ok(PendingRelayHost { ws, cipher, room })
}

/// Join a relayed game by room number. Returns the established [`Connection`]
/// (E2E handshake not yet run).
pub async fn relay_join(
    cfg: &RelayClientConfig,
    room: &str,
    password: &str,
) -> Result<Connection, RelayError> {
    let mut ws = cfg.ws_connect().await?;

    send_control(
        &mut ws,
        &ControlMsg::Join {
            room: room.to_string(),
        },
    )
    .await?;
    let salt_b64 = match recv_control(&mut ws).await? {
        ControlMsg::Joined { salt } => salt,
        ControlMsg::Error { msg } => return Err(RelayError::Server(msg)),
        other => {
            return Err(RelayError::Protocol(format!(
                "expected joined, got {other:?}"
            )))
        }
    };
    let salt = B64
        .decode(salt_b64.as_bytes())
        .map_err(|_| RelayError::Protocol("malformed salt from server".into()))?;

    let cipher = Cipher::from_password_salt(password, &salt);
    Ok(Connection::from_ws(ws, cipher))
}

mod danger {
    use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use rustls::{DigitallySignedStruct, SignatureScheme};

    /// Development-only verifier that accepts any server certificate.
    ///
    /// This disables server authentication and must only be used with
    /// `insecure = true` against a trusted local/dev relay. The inner AEAD
    /// layer still protects game data end-to-end.
    #[derive(Debug)]
    pub struct NoVerify;

    impl ServerCertVerifier for NoVerify {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            rustls::crypto::ring::default_provider()
                .signature_verification_algorithms
                .supported_schemes()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_msg_json_roundtrip() {
        let msgs = vec![
            ControlMsg::Create {
                salt: "c2FsdA==".into(),
            },
            ControlMsg::Created {
                room: "12345678".into(),
            },
            ControlMsg::Join {
                room: "12345678".into(),
            },
            ControlMsg::Joined {
                salt: "c2FsdA==".into(),
            },
            ControlMsg::PeerJoined,
            ControlMsg::Error {
                msg: "no such room".into(),
            },
        ];
        for m in msgs {
            let s = serde_json::to_string(&m).unwrap();
            assert_eq!(serde_json::from_str::<ControlMsg>(&s).unwrap(), m);
        }
    }

    #[test]
    fn default_config_uses_compiled_constants() {
        let cfg = RelayClientConfig::default();
        assert_eq!(cfg.host, DEFAULT_RELAY_HOST);
        assert_eq!(cfg.port, DEFAULT_RELAY_PORT);
        assert!(!cfg.insecure);
    }
}
