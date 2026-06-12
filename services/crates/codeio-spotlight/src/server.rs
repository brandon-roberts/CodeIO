use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use codeio_common::proto::ai::{
    spotlight_service_server::SpotlightService,
    SpotlightQuery, SpotlightResults, SpotlightHit,
    SearchMode, MatchKind,
};
use codeio_common::proto::index::IndexEntry;

use crate::trigram::TrigramIndex;

pub struct SpotlightServer {
    trigrams: Arc<RwLock<TrigramIndex>>,
    entries:  Arc<RwLock<Vec<IndexEntry>>>,
}

impl SpotlightServer {
    pub fn new(
        trigrams: Arc<RwLock<TrigramIndex>>,
        entries: Arc<RwLock<Vec<IndexEntry>>>,
    ) -> Self {
        Self { trigrams, entries }
    }
}

#[tonic::async_trait]
impl SpotlightService for SpotlightServer {
    async fn search(
        &self,
        req: Request<SpotlightQuery>,
    ) -> Result<Response<SpotlightResults>, Status> {
        let q = req.into_inner();
        let max = q.max_results.max(1) as usize;

        let mode = SearchMode::try_from(q.search_mode)
            .unwrap_or(SearchMode::Hybrid);

        let hits = match mode {
            SearchMode::Exact | SearchMode::Fuzzy | SearchMode::Hybrid => {
                let tri = self.trigrams.read().await;
                let ranked = tri.search(&q.query_text, max * 2);
                let entries = self.entries.read().await;
                let entry_map: std::collections::HashMap<_, _> = entries.iter().map(|e| (e.id.as_str(), e)).collect();

                ranked
                    .into_iter()
                    .filter_map(|(id, score)| {
                        let entry = entry_map.get(id.as_str())?;
                        Some(SpotlightHit {
                            file_ref: entry.file_ref.clone(),
                            span: entry.span.clone(),
                            score,
                            match_kind: MatchKind::Fuzzy as i32,
                            context_before: vec![],
                            context_after: vec![],
                            symbol_info: entry.symbol_record.clone(),
                        })
                    })
                    .take(max)
                    .collect::<Vec<_>>()
            }
            SearchMode::Semantic => {
                // Semantic search requires embeddings — return empty until
                // the embedding backend (sqlite-vec / Qdrant) is wired in.
                vec![]
            }
            _ => vec![],
        };

        let total = hits.len() as i32;
        Ok(Response::new(SpotlightResults { hits, total_matches: total, truncated: false }))
    }
}
