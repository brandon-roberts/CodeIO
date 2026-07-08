//! L2 — Index (coherence). Bidirectional IR<->location map + replayable trace.
//! This is the round-trip spine: IR->codebase and codebase->IR both read this index.
//! Depends on L1 router + L0 selector + ir.

use crate::router::Router;
use codeio_ir::Graph;
use std::collections::HashMap;

/// One replayable routing decision: which rule matched which node to which destination.
#[derive(Debug, Clone)]
pub struct TraceStep {
    pub node_id: String,
    pub node_kind: String,
    pub selector: String,
    pub rule_index: usize,
    pub stack: String,
    pub bucket: String,
}

/// The bidirectional routing index over a graph.
#[derive(Default)]
pub struct RouteIndex {
    // forward: ir node id -> (stack, bucket)
    forward: HashMap<String, (String, String)>,
    // reverse: (stack, bucket) -> [ir node ids]  — codebase location back to IR
    reverse: HashMap<(String, String), Vec<String>>,
    pub trace: Vec<TraceStep>,
    pub unrouted: Vec<String>, // nodes no rule matched (honest: reported, not hidden)
}

impl RouteIndex {
    /// Build the index by routing every node in the graph. Pure over (router, graph).
    pub fn build(router: &Router, g: &Graph) -> RouteIndex {
        let mut idx = RouteIndex::default();
        let mut ids: Vec<&String> = g.nodes.keys().collect();
        ids.sort(); // deterministic order -> stable traces
        for id in ids {
            match router.resolve(id, g) {
                Some((route, rule_i)) => {
                    let node = g.get(id).unwrap();
                    idx.forward.insert(id.clone(), (route.stack.clone(), route.bucket.clone()));
                    idx.reverse.entry((route.stack.clone(), route.bucket.clone()))
                        .or_default().push(id.clone());
                    idx.trace.push(TraceStep {
                        node_id: id.clone(),
                        node_kind: node.kind.as_str().to_string(),
                        selector: router.rules[rule_i].selector.raw.clone(),
                        rule_index: rule_i,
                        stack: route.stack,
                        bucket: route.bucket,
                    });
                }
                None => idx.unrouted.push(id.clone()),
            }
        }
        idx
    }

    /// IR -> codebase location.
    pub fn location_of(&self, node_id: &str) -> Option<&(String, String)> {
        self.forward.get(node_id)
    }
    /// codebase location -> IR nodes (the reverse round-trip link).
    pub fn nodes_at(&self, stack: &str, bucket: &str) -> Vec<&String> {
        self.reverse.get(&(stack.to_string(), bucket.to_string()))
            .map(|v| v.iter().collect()).unwrap_or_default()
    }
    pub fn routed_count(&self) -> usize { self.forward.len() }

    /// Stack distribution: how many nodes routed to each stack (scaling view).
    pub fn stack_distribution(&self) -> HashMap<String, usize> {
        let mut d = HashMap::new();
        for (stack, _) in self.forward.values() {
            *d.entry(stack.clone()).or_insert(0) += 1;
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::{Router, RoutingRule};
    use codeio_ir::lower;
    use codeio_lang::{lexer::Lexer, parser::Parser};

    fn g(src: &str) -> Graph {
        let toks = Lexer::new(src).tokenize().unwrap();
        lower(&Parser::new(toks).parse_program().unwrap())
    }

    #[test]
    fn routes_and_indexes_bidirectionally() {
        let graph = g("table T { n: Int } from r in T select r.n let x = 1 + 2");
        let mut router = Router::new();
        router.add(RoutingRule::new("kind(TABLE_DEF)", "rust", "rust-tables", "schema"))
              .add(RoutingRule::new("kind(QUERY)", "rust", "rust-query", "queries"))
              .add(RoutingRule::new("kind(LITERAL)", "python", "py", "consts"));
        let idx = RouteIndex::build(&router, &graph);

        assert!(idx.routed_count() > 0);
        // forward: a table_def node routes to rust/schema
        let td = graph.by_kind(&codeio_ir::NodeKind::TableDef)[0];
        assert_eq!(idx.location_of(&td.id).unwrap().0, "rust");
        // reverse: python/consts contains the literal nodes
        assert!(!idx.nodes_at("python", "consts").is_empty());
        // every routed node has a trace step
        assert_eq!(idx.trace.len(), idx.routed_count());
    }

    #[test]
    fn cascade_specificity_wins() {
        let graph = g("table Users { n: Int }");
        let mut router = Router::new();
        // general rule then specific rule; specific must win
        router.add(RoutingRule::new("*", "default", "d", "all"))
              .add(RoutingRule::new("kind(TABLE_DEF)", "rust", "r", "schema"));
        let idx = RouteIndex::build(&router, &graph);
        let td = graph.by_kind(&codeio_ir::NodeKind::TableDef)[0];
        assert_eq!(idx.location_of(&td.id).unwrap().0, "rust"); // specific beat *
    }

    #[test]
    fn unrouted_reported_not_hidden() {
        let graph = g("let x = 1");
        let router = Router::new(); // no rules
        let idx = RouteIndex::build(&router, &graph);
        assert_eq!(idx.routed_count(), 0);
        assert!(!idx.unrouted.is_empty()); // honest: nothing routed, all reported
    }
}
