use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

use codeio_common::proto::ai::spotlight_service_server::SpotlightServiceServer;
use codeio_spotlight::server::SpotlightServer;
use codeio_spotlight::trigram::TrigramIndex;
use codeio_common::proto::index::IndexEntry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
    ).init();

    let addr: SocketAddr = std::env::var("CODEIO_SPOTLIGHT_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50053".into())
        .parse()?;

    tracing::info!("codeio-spotlight listening on {}", addr);

    let trigrams = Arc::new(RwLock::new(TrigramIndex::default()));
    let entries: Arc<RwLock<Vec<IndexEntry>>> = Arc::new(RwLock::new(vec![]));

    Server::builder()
        .add_service(SpotlightServiceServer::new(
            SpotlightServer::new(trigrams, entries)
        ))
        .serve(addr)
        .await?;

    Ok(())
}
