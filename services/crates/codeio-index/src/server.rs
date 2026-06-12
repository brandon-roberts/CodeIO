use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use codeio_common::proto::index::{
    workspace_scan_service_server::WorkspaceScanService,
    context_index_service_server::ContextIndexService,
    ScanRequest, ScanResult,
    IndexRequest, IndexResponse,
    GetEntriesRequest, GetEntriesResponse,
    GetEntryRequest, IndexEntry,
};

use crate::scanner::WorkspaceScanner;
use crate::symbol_index::SymbolIndexer;
use crate::store::IndexStore;

pub struct ScanServer {
    scanner: WorkspaceScanner,
}

impl ScanServer {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { scanner: WorkspaceScanner::new(root) }
    }
}

#[tonic::async_trait]
impl WorkspaceScanService for ScanServer {
    async fn scan(&self, req: Request<ScanRequest>) -> Result<Response<ScanResult>, Status> {
        let r = req.into_inner();
        let result = self.scanner.scan(&r.workspace_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(result))
    }

    type WatchWorkspaceStream = tokio_stream::wrappers::ReceiverStream<Result<ScanResult, Status>>;

    async fn watch_workspace(
        &self,
        req: Request<ScanRequest>,
    ) -> Result<Response<Self::WatchWorkspaceStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let r = req.into_inner();
        let scanner = WorkspaceScanner::new(&r.root_path);

        tokio::spawn(async move {
            // Initial scan
            if let Ok(result) = scanner.scan(&r.workspace_id) {
                let _ = tx.send(Ok(result)).await;
            }
            // File watching handled by codeio-index's file_watcher module
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

pub struct IndexServer {
    store: Arc<RwLock<IndexStore>>,
    indexer: SymbolIndexer,
}

impl IndexServer {
    pub fn new(store: Arc<RwLock<IndexStore>>) -> Self {
        Self { store, indexer: SymbolIndexer }
    }
}

#[tonic::async_trait]
impl ContextIndexService for IndexServer {
    async fn index_files(
        &self,
        req: Request<IndexRequest>,
    ) -> Result<Response<IndexResponse>, Status> {
        let r = req.into_inner();
        let mut indexed = 0i32;
        let mut updated = 0i32;

        for file_ref in &r.files {
            let path = std::path::Path::new(&file_ref.path);
            let source = tokio::fs::read_to_string(path).await
                .map_err(|e| Status::internal(e.to_string()))?;

            let entries = self.indexer.index_file(file_ref, &source, &r.workspace_id);
            let count = entries.len() as i32;

            let mut store = self.store.write().await;
            let prev = store.replace_file_entries(&file_ref.path, entries);
            if prev > 0 { updated += count; } else { indexed += count; }
        }

        Ok(Response::new(IndexResponse {
            workspace_id: r.workspace_id,
            entries_indexed: indexed,
            entries_updated: updated,
            entries_deleted: 0,
        }))
    }

    async fn get_entries(
        &self,
        req: Request<GetEntriesRequest>,
    ) -> Result<Response<GetEntriesResponse>, Status> {
        let r = req.into_inner();
        let file_path = r.file_ref.map(|f| f.path).unwrap_or_default();
        let store = self.store.read().await;
        let entries = store.get_file_entries(&file_path);
        Ok(Response::new(GetEntriesResponse { entries }))
    }

    async fn get_entry(
        &self,
        req: Request<GetEntryRequest>,
    ) -> Result<Response<IndexEntry>, Status> {
        let r = req.into_inner();
        let store = self.store.read().await;
        store.get_entry(&r.entry_id)
            .map(|e| Response::new(e))
            .ok_or_else(|| Status::not_found(format!("entry {} not found", r.entry_id)))
    }
}
