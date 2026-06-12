use codeio_common::proto::ai::{ContextBudget, ContextSlice, ContextWindow, FocusPoint};
use codeio_common::proto::index::IndexEntry;

use crate::ranker::rank;

/// Builds a ContextWindow from a flat list of IndexEntry candidates.
/// Greedy knapsack: add entries in score order until the token budget is full.
pub fn assemble(
    workspace_id: &str,
    focus: FocusPoint,
    budget: &ContextBudget,
    candidates: Vec<IndexEntry>,
    request_id: String,
) -> ContextWindow {
    let available = budget.max_tokens
        - budget.system_reserve
        - budget.history_reserve
        - budget.response_reserve;

    let mut scored: Vec<(IndexEntry, f32, codeio_common::proto::ai::IncludeReason)> = candidates
        .into_iter()
        .map(|e| {
            let (score, reason) = rank(&e, &focus);
            (e, score, reason)
        })
        .collect();

    // Sort descending by relevance score
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut slices = Vec::new();
    let mut used_tokens = 0i32;
    let mut truncated = false;
    let mut skipped = 0usize;

    for (rank_pos, (entry, score, reason)) in scored.into_iter().enumerate() {
        let cost = entry.tokens;
        if used_tokens + cost > available {
            skipped += 1;
            truncated = true;
            continue;
        }
        used_tokens += cost;
        slices.push(ContextSlice {
            entry: Some(entry),
            relevance_score: score,
            include_reason: reason as i32,
            rank: rank_pos as i32,
        });
    }

    let truncation_summary = if truncated {
        format!("omitted {} low-relevance entries (budget: {} tokens)", skipped, available)
    } else {
        String::new()
    };

    ContextWindow {
        request_id,
        workspace_id: workspace_id.to_string(),
        focus_point: Some(focus),
        slices,
        total_tokens: used_tokens,
        truncated,
        truncation_summary,
    }
}
