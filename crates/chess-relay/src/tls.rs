//! Server-side TLS using rustls with the **ring** provider (no OpenSSL).

use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;

/// Build a [`TlsAcceptor`] from PEM certificate-chain and private-key files.
pub fn build_acceptor(cert_path: &Path, key_path: &Path) -> anyhow::Result<TlsAcceptor> {
    let cert_pem = std::fs::read(cert_path)
        .with_context(|| format!("reading certificate {}", cert_path.display()))?;
    let key_pem = std::fs::read(key_path)
        .with_context(|| format!("reading private key {}", key_path.display()))?;

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut &cert_pem[..])
        .collect::<Result<_, _>>()
        .with_context(|| format!("parsing certificate {}", cert_path.display()))?;
    anyhow::ensure!(
        !certs.is_empty(),
        "no certificates found in {}",
        cert_path.display()
    );

    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut &key_pem[..])
        .with_context(|| format!("parsing private key {}", key_path.display()))?
        .with_context(|| format!("no private key found in {}", key_path.display()))?;

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("configuring TLS protocol versions")?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("installing server certificate")?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
