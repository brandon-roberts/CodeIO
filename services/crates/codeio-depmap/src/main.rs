use std::net::SocketAddr;
use tonic::transport::Server;

use codeio_common::proto::index::dependency_map_service_server::DependencyMapServiceServer;
use codeio_depmap::server::DepMapServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
    ).init();

    let addr: SocketAddr = std::env::var("CODEIO_DEPMAP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50055".into())
        .parse()?;

    tracing::info!("codeio-depmap listening on {}", addr);

    Server::builder()
        .add_service(DependencyMapServiceServer::new(DepMapServer))
        .serve(addr)
        .await?;

    Ok(())
}
