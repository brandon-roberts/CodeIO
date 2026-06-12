use std::collections::HashMap;
use codeio_common::proto::index::IndexEntry;

/// In-memory index store. Replace with SQLite or RocksDB for persistence.
#[derive(Default)]
pub struct IndexStore {
    by_id:   HashMap<String, IndexEntry>,
    by_file: HashMap<String, Vec<String>>,  // file_path -> entry IDs
}

impl IndexStore {
    pub fn replace_file_entries(&mut self, file_path: &str, entries: Vec<IndexEntry>) -> usize {
        let old_ids = self.by_file.remove(file_path).unwrap_or_default();
        let prev_count = old_ids.len();
        for id in old_ids {
            self.by_id.remove(&id);
        }

        let new_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
        for entry in entries {
            self.by_id.insert(entry.id.clone(), entry);
        }
        self.by_file.insert(file_path.to_string(), new_ids);

        prev_count
    }

    pub fn get_file_entries(&self, file_path: &str) -> Vec<IndexEntry> {
        self.by_file.get(file_path)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_entry(&self, id: &str) -> Option<IndexEntry> {
        self.by_id.get(id).cloned()
    }

    pub fn all_entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.by_id.values()
    }
}
