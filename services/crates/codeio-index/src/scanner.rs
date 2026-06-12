use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use ignore::WalkBuilder;
use prost_types::Timestamp;

use codeio_common::proto::core::{FileRef, Language};
use codeio_common::proto::index::{FileNode, ScanResult};

pub struct WorkspaceScanner {
    root: PathBuf,
}

impl WorkspaceScanner {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn scan(&self, workspace_id: &str) -> Result<ScanResult> {
        let root_node = self.build_node(&self.root)?;
        let (files, dirs) = count_nodes(&root_node);

        Ok(ScanResult {
            workspace_id: workspace_id.to_string(),
            root: Some(root_node),
            total_files: files as i32,
            total_directories: dirs as i32,
            scanned_at: Some(system_time_to_proto(SystemTime::now())),
            errors: vec![],
        })
    }

    fn build_node(&self, path: &Path) -> Result<FileNode> {
        if path.is_dir() {
            let mut children: Vec<FileNode> = WalkBuilder::new(path)
                .max_depth(Some(1))
                .hidden(false)
                .build()
                .skip(1) // skip the root entry itself
                .filter_map(|e| e.ok())
                .filter(|e| e.depth() == 1)
                .map(|e| self.build_node(e.path()))
                .collect::<Result<Vec<_>>>()?;

            children.sort_by(|a, b| {
                let a_dir = a.is_directory;
                let b_dir = b.is_directory;
                b_dir.cmp(&a_dir).then(node_path(a).cmp(&node_path(b)))
            });

            Ok(FileNode {
                file_ref: Some(file_ref_for(path, workspace_id_from_path(path).as_deref())),
                is_directory: true,
                children,
            })
        } else {
            Ok(FileNode {
                file_ref: Some(file_ref_for(path, None)),
                is_directory: false,
                children: vec![],
            })
        }
    }
}

fn file_ref_for(path: &Path, workspace_id: Option<&str>) -> FileRef {
    let meta = path.metadata().ok();
    let last_modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .map(system_time_to_proto);
    let size_bytes = meta.map(|m| m.len() as i64).unwrap_or(0);

    FileRef {
        path: path.to_string_lossy().into_owned(),
        language: detect_language(path) as i32,
        last_modified,
        size_bytes,
        workspace_id: workspace_id.unwrap_or("").to_string(),
    }
}

fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("cpp" | "cxx" | "cc" | "c" | "h" | "hpp") => Language::Cpp,
        Some("rs")                                        => Language::Rust,
        Some("hs" | "lhs")                               => Language::Haskell,
        Some("clj" | "cljs" | "cljc" | "edn")
        | Some("lisp" | "el" | "scm" | "rkt")            => Language::Lisp,
        Some("py" | "pyi")                               => Language::Python,
        Some("java" | "kt" | "kts")                      => Language::Java,
        Some("js" | "mjs" | "cjs")                       => Language::Javascript,
        Some("ts" | "tsx" | "d.ts")                      => Language::Typescript,
        Some("php")                                       => Language::Php,
        Some("cio")                                       => Language::Codeio,
        _                                                 => Language::Unknown,
    }
}

fn count_nodes(node: &FileNode) -> (usize, usize) {
    if node.is_directory {
        let (f, d) = node.children.iter().map(count_nodes).fold((0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        (f, d + 1)
    } else {
        (1, 0)
    }
}

fn node_path(node: &FileNode) -> String {
    node.file_ref.as_ref().map(|r| r.path.clone()).unwrap_or_default()
}

fn workspace_id_from_path(_path: &Path) -> Option<String> {
    None
}

fn system_time_to_proto(t: SystemTime) -> Timestamp {
    let dur = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    Timestamp {
        seconds: dur.as_secs() as i64,
        nanos: dur.subsec_nanos() as i32,
    }
}
