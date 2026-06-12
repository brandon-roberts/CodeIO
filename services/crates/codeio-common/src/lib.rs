// Re-exports all generated protobuf/gRPC types for use across Rust crates.
// Other crates depend on this and import via `codeio_common::proto::*`.

pub mod proto {
    pub mod core {
        tonic::include_proto!("codeio.core");
    }
    pub mod index {
        tonic::include_proto!("codeio.index");
    }
    pub mod ai {
        tonic::include_proto!("codeio.ai");
    }
    pub mod frontend {
        tonic::include_proto!("codeio.frontend");
    }
    pub mod vm {
        tonic::include_proto!("codeio.vm");
    }
    pub mod meta {
        tonic::include_proto!("codeio.meta");
    }
}

pub use proto::core::{FileRef, Language, Position, Span, SeverityLevel};
pub use proto::index::{IndexEntry, SymbolRecord, ChunkKind, SymbolKind};
pub use proto::ai::{ContextWindow, FocusPoint, ContextBudget};
