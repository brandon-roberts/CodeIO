//! L1 — Router (capability). Maps selector matches to destinations via relative/dynamic rules.
//! No writing: it decides WHERE a node goes, never rewrites it. Depends only on L0 selector + ir.

use crate::selector::{match_nodes, Selector};
use codeio_ir::Graph;

/// A destination for routed nodes: a language stack, adapter, and index bucket.
/// (Same shape serves UI styling: stack="render", bucket=a style class.)
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub stack: String,   // "rust" | "python" | "js" | "render" | ...
    pub adapter: String, // adapter/bucket id
    pub bucket: String,  // index bucket / style class
}

/// A routing rule: selector -> route, with specificity from the selector for cascade.
#[derive(Debug, Clone)]
pub struct RoutingRule {
    pub selector: Selector,
    pub route: Route,
}
impl RoutingRule {
    pub fn new(sel: &str, stack: &str, adapter: &str, bucket: &str) -> Self {
        RoutingRule {
            selector: Selector::parse(sel),
            route: Route { stack: stack.into(), adapter: adapter.into(), bucket: bucket.into() },
        }
    }
}

/// The router holds an ordered ruleset. Cascade: highest selector specificity wins;
/// ties broken by declaration order (later wins) — deterministic, so traces are stable.
#[derive(Default)]
pub struct Router {
    pub rules: Vec<RoutingRule>,
}

impl Router {
    pub fn new() -> Self { Router::default() }
    pub fn add(&mut self, rule: RoutingRule) -> &mut Self { self.rules.push(rule); self }

    /// Resolve the winning route for a single node id, plus the winning rule index (for tracing).
    pub fn resolve(&self, node_id: &str, g: &Graph) -> Option<(Route, usize)> {
        let mut best: Option<(u32, usize)> = None; // (specificity, rule_idx)
        for (i, rule) in self.rules.iter().enumerate() {
            let hits = match_nodes(&rule.selector, g);
            if hits.iter().any(|n| n.id == node_id) {
                let spec = rule.selector.specificity();
                match best {
                    Some((bspec, _)) if spec < bspec => {}
                    _ => best = Some((spec, i)),
                }
            }
        }
        best.map(|(_, i)| (self.rules[i].route.clone(), i))
    }
}
