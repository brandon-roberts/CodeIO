use std::collections::{HashMap, HashSet};

/// Trigram inverted index for sub-millisecond fuzzy symbol search.
/// Maps each 3-character trigram to the set of entry IDs containing it.
#[derive(Default)]
pub struct TrigramIndex {
    index: HashMap<[u8; 3], Vec<String>>,
}

impl TrigramIndex {
    pub fn insert(&mut self, text: &str, entry_id: &str) {
        for tri in trigrams(text) {
            self.index
                .entry(tri)
                .or_default()
                .push(entry_id.to_string());
        }
    }

    pub fn remove(&mut self, text: &str, entry_id: &str) {
        for tri in trigrams(text) {
            if let Some(ids) = self.index.get_mut(&tri) {
                ids.retain(|id| id != entry_id);
            }
        }
    }

    /// Returns candidate entry IDs ranked by trigram overlap with `query`.
    /// Score = matched_trigrams / total_trigrams(query) — Jaccard-like.
    pub fn search(&self, query: &str, max_results: usize) -> Vec<(String, f32)> {
        let query_tris: Vec<[u8; 3]> = trigrams(query).collect();
        if query_tris.is_empty() {
            return vec![];
        }

        let mut scores: HashMap<&str, usize> = HashMap::new();
        for tri in &query_tris {
            if let Some(ids) = self.index.get(tri) {
                for id in ids {
                    *scores.entry(id.as_str()).or_insert(0) += 1;
                }
            }
        }

        let total = query_tris.len() as f32;
        let mut ranked: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(id, hits)| (id.to_string(), hits as f32 / total))
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        ranked.truncate(max_results);
        ranked
    }
}

fn trigrams(s: &str) -> impl Iterator<Item = [u8; 3]> + '_ {
    let padded = format!("  {}  ", s.to_lowercase());
    let bytes = padded.into_bytes();
    (0..bytes.len().saturating_sub(2)).map(move |i| [bytes[i], bytes[i + 1], bytes[i + 2]])
}
