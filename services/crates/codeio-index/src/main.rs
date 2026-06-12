use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

use codeio_common::proto::index::{
    workspace_scan_service_server::WorkspaceScanServiceServer,
    context_index_service_server::ContextIndexServiceServer,
};
use codeio_index::{server::{ScanServer, IndexServer}, store::IndexStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let addr: SocketAddr = std::env::var("CODEIO_INDEX_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50052".into())
        .parse()?;

    let workspace_root = std::env::var("CODEIO_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    tracing::info!("codeio-index listening on {}", addr);

    let store = Arc::new(RwLock::new(IndexStore::default()));

    Server::builder()
        .add_service(WorkspaceScanServiceServer::new(ScanServer::new(&workspace_root)))
        .add_service(ContextIndexServiceServer::new(IndexServer::new(store)))
        .serve(addr)
        .await?;

    Ok(())
}
