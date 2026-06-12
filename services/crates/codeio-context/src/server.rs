use std::sync::Arc;
use std::path::PathBuf;

use prost::Message;
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};
use tokio::sync::RwLock;

use codeio_common::proto::ai::{
    context_window_service_server::ContextWindowService,
    AssembleWindowRequest, AssembleWindowResponse,
    GetWindowRequest, ContextWindow,
};
use codeio_common::proto::index::IndexEntry;

use crate::assembler::assemble;

pub struct ContextServer {
    entries: Arc<RwLock<Vec<IndexEntry>>>,
    mmap_dir: PathBuf,
}

impl ContextServer {
    pub fn new(entries: Arc<RwLock<Vec<IndexEntry>>>, mmap_dir: PathBuf) -> Self {
        Self { entries, mmap_dir }
    }
}

#[tonic::async_trait]
impl ContextWindowService for ContextServer {
    async fn assemble_window(
        &self,
        req: Request<AssembleWindowRequest>,
    ) -> Result<Response<AssembleWindowResponse>, Status> {
        let r = req.into_inner();
        let focus = r.focus.unwrap_or_default();
        let budget = r.budget.unwrap_or_default();

        let candidates = self.entries.read().await.clone();
        let request_id = uuid_v4();

        let window = assemble(&r.workspace_id, focus, &budget, candidates, request_id.clone());

        // Serialize to a temp mmap file for large payloads
        let bytes = window.encode_to_vec();
        let path = self.mmap_dir.join(format!("ctx_{}.bin", request_id));
        tokio::fs::write(&path, &bytes).await
            .map_err(|e| Status::internal(e.to_string()))?;

        let checksum = hex::encode(Sha256::digest(&bytes));

        Ok(Response::new(AssembleWindowResponse {
            mmap_path: path.to_string_lossy().into_owned(),
            byte_size: bytes.len() as i64,
            checksum,
            total_tokens: window.total_tokens,
        }))
    }

    async fn get_window(
        &self,
        req: Request<GetWindowRequest>,
    ) -> Result<Response<ContextWindow>, Status> {
        let r = req.into_inner();
        let bytes = tokio::fs::read(&r.mmap_path).await
            .map_err(|e| Status::not_found(e.to_string()))?;
        let window = ContextWindow::decode(bytes.as_slice())
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(window))
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{:032x}", t)
}
