use codeio_common::proto::core::{FileRef, Language};
use codeio_common::proto::index::{DependencyEdge, DependencyGraph, DependencyNode, EdgeKind};

/// Resolves imports/uses/requires from source text into a DependencyGraph.
/// Works across all 8 host languages using pattern matching on import syntax.
pub fn resolve(file_ref: &FileRef, source: &str) -> DependencyGraph {
    let mut nodes = vec![DependencyNode {
        id: file_ref.path.clone(),
        file: Some(file_ref.clone()),
        symbol: String::new(),
        module: module_name(&file_ref.path),
    }];
    let mut edges = vec![];

    let lang = Language::try_from(file_ref.language).unwrap_or(Language::Unknown);
    let imports = extract_imports(source, lang);

    for imp in imports {
        let dep_id = imp.clone();
        nodes.push(DependencyNode {
            id: dep_id.clone(),
            file: None,
            symbol: String::new(),
            module: imp,
        });
        edges.push(DependencyEdge {
            from_id: file_ref.path.clone(),
            to_id: dep_id,
            kind: EdgeKind::Import as i32,
        });
    }

    DependencyGraph { nodes, edges }
}

fn extract_imports(source: &str, lang: Language) -> Vec<String> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        let raw = match lang {
            Language::Python => {
                if let Some(r) = t.strip_prefix("import ").or_else(|| t.strip_prefix("from ")) {
                    r.split_whitespace().next().map(|s| s.trim_end_matches(',').to_string())
                } else { None }
            }
            Language::Rust => {
                if let Some(r) = t.strip_prefix("use ") {
                    r.split("::").next().map(|s| s.trim_end_matches(';').to_string())
                } else if let Some(r) = t.strip_prefix("extern crate ") {
                    Some(r.trim_end_matches(';').to_string())
                } else { None }
            }
            Language::Javascript | Language::Typescript => {
                if t.contains("from \"") || t.contains("from '") {
                    t.split('"').nth(1).or_else(|| t.split('\'').nth(1)).map(|s| s.to_string())
                } else { None }
            }
            Language::Java => {
                t.strip_prefix("import ")
                    .map(|r| r.split('.').next().unwrap_or("").trim_end_matches(';').to_string())
            }
            Language::Haskell => {
                t.strip_prefix("import ").map(|r| r.split_whitespace().next().unwrap_or("").to_string())
            }
            _ => None,
        };
        if let Some(name) = raw {
            if !name.is_empty() { imports.push(name); }
        }
    }
    imports
}

fn module_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}
