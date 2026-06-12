use tonic::{Request, Response, Status};

use codeio_common::proto::index::{
    dependency_map_service_server::DependencyMapService,
    DepMapRequest, DepMapResponse, dep_map_request::Target,
};

use crate::resolver::resolve;

#[derive(Default)]
pub struct DepMapServer;

#[tonic::async_trait]
impl DependencyMapService for DepMapServer {
    async fn get_dependency_map(
        &self,
        req: Request<DepMapRequest>,
    ) -> Result<Response<DepMapResponse>, Status> {
        let r = req.into_inner();

        let graph = match r.target {
            Some(Target::File(file_ref)) => {
                let path = file_ref.path.clone();
                let source = tokio::fs::read_to_string(&path).await
                    .map_err(|e| Status::not_found(e.to_string()))?;
                resolve(&file_ref, &source)
            }
            Some(Target::SymbolQualifiedName(_sym)) => {
                return Err(Status::unimplemented("symbol-level dep map not yet implemented"));
            }
            None => {
                return Err(Status::invalid_argument("target is required"));
            }
        };

        Ok(Response::new(DepMapResponse { graph: Some(graph) }))
    }
}
