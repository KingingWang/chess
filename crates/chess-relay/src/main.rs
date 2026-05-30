//! `chess-relay` binary entry point. See the crate library docs for details.
//!
//! Config precedence: **config file > environment > compiled default**.
//! Use `--config <path>` to select the file (defaults to `relay.toml`).

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chess_relay=info".into()),
        )
        .init();
    chess_relay::run().await
}
