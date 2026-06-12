use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

use codeio_common::proto::index::{ChunkKind, IndexEntry, SymbolKind, SymbolRecord, Visibility};
use codeio_common::proto::core::{FileRef, Span, Position};

/// Builds IndexEntry records from raw source text.
/// Uses line-based heuristics so it works across all languages without
/// requiring a language-specific AST from the Haskell parser.
pub struct SymbolIndexer;

impl SymbolIndexer {
    pub fn index_file(&self, file_ref: &FileRef, source: &str, workspace_id: &str) -> Vec<IndexEntry> {
        let mut entries = Vec::new();

        // Always index the file header (first 20 lines)
        let header_end = source.lines().take(20).count();
        if header_end > 0 {
            entries.push(self.make_entry(
                workspace_id,
                file_ref,
                source,
                0,
                header_end.saturating_sub(1),
                ChunkKind::FileHeader,
                None,
            ));
        }

        // Language-specific symbol extraction
        let symbols = extract_symbols(source, file_ref);
        for sym in symbols {
            entries.push(self.make_entry(
                workspace_id,
                file_ref,
                source,
                sym.start_line,
                sym.end_line,
                sym.chunk_kind,
                Some(sym.record),
            ));
        }

        entries
    }

    fn make_entry(
        &self,
        workspace_id: &str,
        file_ref: &FileRef,
        source: &str,
        start_line: usize,
        end_line: usize,
        chunk_kind: ChunkKind,
        symbol: Option<SymbolRecord>,
    ) -> IndexEntry {
        let raw_content: String = source
            .lines()
            .skip(start_line)
            .take(end_line - start_line + 1)
            .collect::<Vec<_>>()
            .join("\n");

        let mut hasher = Sha256::new();
        hasher.update(raw_content.as_bytes());
        let content_hash = hex::encode(hasher.finalize());

        let id = {
            let mut h = Sha256::new();
            h.update(workspace_id);
            h.update(&file_ref.path);
            h.update(start_line.to_string().as_bytes());
            hex::encode(h.finalize())
        };

        let tokens = estimate_tokens(&raw_content);

        IndexEntry {
            id,
            workspace_id: workspace_id.to_string(),
            file_ref: Some(file_ref.clone()),
            span: Some(Span {
                start: Some(Position { line: start_line as i32, column: 0, byte_offset: 0 }),
                end: Some(Position { line: end_line as i32, column: 0, byte_offset: 0 }),
            }),
            chunk_kind: chunk_kind as i32,
            symbol_record: symbol,
            content_hash,
            embedding_id: String::new(),
            tokens,
            importance_score: compute_importance(&chunk_kind, &symbol),
            last_indexed: None,
            raw_content,
        }
    }
}

struct SymbolHit {
    start_line: usize,
    end_line: usize,
    chunk_kind: ChunkKind,
    record: SymbolRecord,
}

fn extract_symbols(source: &str, file_ref: &FileRef) -> Vec<SymbolHit> {
    let lines: Vec<&str> = source.lines().collect();
    let mut hits = Vec::new();

    // Universal patterns that work across most languages
    let fn_patterns = [
        "fn ", "def ", "function ", "func ", "fun ", "sub ", "method ",
        "defn ", "defun ", "lambda ", "proc ",
    ];
    let type_patterns = [
        "class ", "struct ", "interface ", "trait ", "enum ", "type ",
        "data ", "newtype ", "typedef ", "impl ",
    ];

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        let (kind, sym_kind) = if fn_patterns.iter().any(|p| trimmed.starts_with(p)) {
            (ChunkKind::Function, SymbolKind::Function)
        } else if type_patterns.iter().any(|p| trimmed.starts_with(p)) {
            (ChunkKind::TypeDef, SymbolKind::Type)
        } else {
            continue;
        };

        let name = extract_name(trimmed);
        let end = find_block_end(&lines, i);

        hits.push(SymbolHit {
            start_line: i,
            end_line: end,
            chunk_kind: kind,
            record: SymbolRecord {
                name: name.clone(),
                qualified_name: format!("{}::{}", Path::new(&file_ref.path).file_stem()
                    .and_then(|s| s.to_str()).unwrap_or(""), name),
                kind: sym_kind as i32,
                visibility: Visibility::Public as i32,
                signature: trimmed.lines().next().unwrap_or("").to_string(),
                doc_comment: extract_doc_comment(&lines, i),
                defined_at: Some(file_ref.clone()),
                references: vec![],
            },
        });
    }

    hits
}

fn extract_name(line: &str) -> String {
    line.split_whitespace()
        .nth(1)
        .unwrap_or("")
        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
        .to_string()
}

fn find_block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' | '(' => depth += 1,
                '}' | ')' => depth -= 1,
                _ => {}
            }
        }
        if depth <= 0 && i > start {
            return i;
        }
    }
    (start + 30).min(lines.len().saturating_sub(1))
}

fn extract_doc_comment(lines: &[&str], fn_line: usize) -> String {
    let mut doc = Vec::new();
    let mut i = fn_line.saturating_sub(1);
    while i > 0 {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("--") || trimmed.starts_with(";;") {
            doc.push(trimmed.trim_start_matches(|c| c == '/' || c == '#' || c == '-' || c == ';').trim().to_string());
            i -= 1;
        } else {
            break;
        }
    }
    doc.reverse();
    doc.join("\n")
}

fn estimate_tokens(text: &str) -> i32 {
    // ~4 chars per token is a reasonable average across code and prose
    (text.len() / 4) as i32
}

fn compute_importance(chunk_kind: &ChunkKind, symbol: &Option<SymbolRecord>) -> f32 {
    let base = match chunk_kind {
        ChunkKind::Function => 0.7,
        ChunkKind::TypeDef  => 0.8,
        ChunkKind::Module   => 0.9,
        ChunkKind::FileHeader => 0.5,
        ChunkKind::ImportBlock => 0.4,
        ChunkKind::Test     => 0.6,
        ChunkKind::MacroDef => 0.75,
        _                   => 0.3,
    };

    if let Some(sym) = symbol {
        if sym.visibility == Visibility::Public as i32 {
            return (base + 0.1).min(1.0);
        }
    }
    base
}
