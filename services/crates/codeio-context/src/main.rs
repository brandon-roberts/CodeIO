use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

use codeio_common::proto::ai::context_window_service_server::ContextWindowServiceServer;
use codeio_common::proto::index::IndexEntry;
use codeio_context::server::ContextServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
    ).init();

    let addr: SocketAddr = std::env::var("CODEIO_CONTEXT_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50054".into())
        .parse()?;

    let mmap_dir = PathBuf::from(
        std::env::var("CODEIO_MMAP_DIR").unwrap_or_else(|_| "/tmp/codeio".into())
    );
    tokio::fs::create_dir_all(&mmap_dir).await?;

    tracing::info!("codeio-context listening on {}", addr);

    let entries: Arc<RwLock<Vec<IndexEntry>>> = Arc::new(RwLock::new(vec![]));

    Server::builder()
        .add_service(ContextWindowServiceServer::new(
            ContextServer::new(entries, mmap_dir)
        ))
        .serve(addr)
        .await?;

    Ok(())
}
