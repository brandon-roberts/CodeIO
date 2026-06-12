use codeio_common::proto::ai::{FocusPoint, IncludeReason};
use codeio_common::proto::index::IndexEntry;

/// Scores an IndexEntry for relevance to a given FocusPoint.
/// Returns (score, reason) where score ∈ [0.0, 1.0].
pub fn rank(entry: &IndexEntry, focus: &FocusPoint) -> (f32, IncludeReason) {
    // Same file as focus → highest priority
    if let (Some(ef), Some(ff)) = (&entry.file_ref, &focus.file_ref) {
        if ef.path == ff.path {
            // Check if span contains the focus cursor position
            if let (Some(span), Some(pos)) = (&entry.span, &focus.position) {
                if let (Some(start), Some(end)) = (&span.start, &span.end) {
                    if pos.line >= start.line && pos.line <= end.line {
                        return (1.0, IncludeReason::FocusPoint);
                    }
                }
            }
            return (0.9, IncludeReason::SameFile);
        }
    }

    // Symbol name match against focus query
    if !focus.query.is_empty() {
        if let Some(sym) = &entry.symbol_record {
            let q = focus.query.to_lowercase();
            let name = sym.name.to_lowercase();
            if name == q {
                return (0.85, IncludeReason::SymbolRef);
            }
            if name.contains(&q) || q.contains(&name) {
                return (0.7, IncludeReason::SemanticSimilar);
            }
        }
    }

    // Fall back to the static importance score stored in the entry
    (entry.importance_score * 0.5, IncludeReason::SemanticSimilar)
}
