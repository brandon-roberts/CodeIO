//! L0 — Selector (mechanism). Pure, total, no policy. Matches IR nodes by pattern.
//! Base-form reusable: any module can match nodes without any routing/index concern.
//!
//! Selector syntax (CSS-like but match-only):
//!   kind(FN)              — nodes of a kind
//!   [attr]               — nodes having an attribute key
//!   [attr=value]         — attribute equals value
//!   [attr~=substr]       — attribute contains substring
//!   *                    — any node
//! Parts separated by whitespace mean containment: `kind(FN) kind(CALL)` = CALL inside FN.

use codeio_ir::{Graph, Node, NodeKind};

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorPart {
    Any,
    Kind(String),
    HasAttr(String),
    AttrEq(String, String),
    AttrContains(String, String),
}

impl SelectorPart {
    /// Total predicate: does this part match a single node? Never panics, never mutates.
    pub fn matches(&self, n: &Node) -> bool {
        match self {
            SelectorPart::Any => true,
            SelectorPart::Kind(k) => n.kind.as_str().eq_ignore_ascii_case(k),
            SelectorPart::HasAttr(a) => n.attrs.contains_key(a),
            SelectorPart::AttrEq(a, v) => n.attrs.get(a).map_or(false, |x| x == v),
            SelectorPart::AttrContains(a, v) => n.attrs.get(a).map_or(false, |x| x.contains(v)),
        }
    }
}

/// A selector is a containment chain: ancestor parts then the target part (last).
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub chain: Vec<SelectorPart>,
    pub raw: String,
}

impl Selector {
    /// Parse a selector string. Total: unknown input yields an Any-part, never an error,
    /// keeping the engine non-failing (routing decisions are always traceable, never crash).
    pub fn parse(s: &str) -> Selector {
        let chain = s
            .split_whitespace()
            .map(Self::parse_part)
            .collect::<Vec<_>>();
        Selector {
            chain: if chain.is_empty() { vec![SelectorPart::Any] } else { chain },
            raw: s.to_string(),
        }
    }

    fn parse_part(tok: &str) -> SelectorPart {
        if tok == "*" {
            return SelectorPart::Any;
        }
        if let Some(rest) = tok.strip_prefix("kind(").and_then(|r| r.strip_suffix(')')) {
            return SelectorPart::Kind(rest.to_string());
        }
        if let Some(inner) = tok.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            if let Some((a, v)) = inner.split_once("~=") {
                return SelectorPart::AttrContains(a.to_string(), v.to_string());
            }
            if let Some((a, v)) = inner.split_once('=') {
                return SelectorPart::AttrEq(a.to_string(), v.to_string());
            }
            return SelectorPart::HasAttr(inner.to_string());
        }
        // bareword = kind shorthand (fn, call, query, ...)
        SelectorPart::Kind(tok.to_string())
    }

    /// Specificity for cascade/precedence. Simple + deterministic (avoids CSS's quirks):
    /// AttrEq=3, AttrContains=2, Kind=2, HasAttr=1, Any=0; summed, +1 per chain depth.
    pub fn specificity(&self) -> u32 {
        let mut s = self.chain.len() as u32; // depth bonus
        for p in &self.chain {
            s += match p {
                SelectorPart::AttrEq(..) => 3,
                SelectorPart::Kind(_) | SelectorPart::AttrContains(..) => 2,
                SelectorPart::HasAttr(_) => 1,
                SelectorPart::Any => 0,
            };
        }
        s
    }
}

/// Match a selector against a graph. Returns ids of nodes matching the TARGET (last) part,
/// where all ancestor parts are satisfied along a containment path from some root.
/// Pure and total: builds a parent map, walks, never mutates the graph.
pub fn match_nodes<'a>(sel: &Selector, g: &'a Graph) -> Vec<&'a Node> {
    let target = sel.chain.last().cloned().unwrap_or(SelectorPart::Any);
    let ancestors = &sel.chain[..sel.chain.len().saturating_sub(1)];

    // parent index: child_id -> parent_id (first parent seen; IR is a DAG, good enough for containment)
    let mut parent: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for n in g.nodes.values() {
        for c in &n.children {
            parent.entry(c.as_str()).or_insert(n.id.as_str());
        }
    }

    g.nodes
        .values()
        .filter(|n| target.matches(n))
        .filter(|n| ancestors_satisfied(ancestors, n, &parent, g))
        .collect()
}

fn ancestors_satisfied(
    ancestors: &[SelectorPart],
    node: &Node,
    parent: &std::collections::HashMap<&str, &str>,
    g: &Graph,
) -> bool {
    if ancestors.is_empty() {
        return true;
    }
    // walk up from node; each ancestor part (right-to-left) must match some ancestor, in order.
    let mut remaining: Vec<&SelectorPart> = ancestors.iter().rev().collect();
    let mut cur = node.id.as_str();
    while let Some(&pid) = parent.get(cur) {
        if let Some(pnode) = g.get(pid) {
            if let Some(want) = remaining.first() {
                if want.matches(pnode) {
                    remaining.remove(0);
                    if remaining.is_empty() {
                        return true;
                    }
                }
            }
            cur = pid;
        } else {
            break;
        }
    }
    remaining.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use codeio_ir::lower;
    use codeio_lang::{lexer::Lexer, parser::Parser};

    fn g(src: &str) -> Graph {
        let toks = Lexer::new(src).tokenize().unwrap();
        lower(&Parser::new(toks).parse_program().unwrap())
    }

    #[test]
    fn matches_by_kind() {
        let graph = g("let x = 1 + 2");
        let m = match_nodes(&Selector::parse("kind(LITERAL)"), &graph);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn bareword_is_kind() {
        let graph = g("table T { n: Int }");
        assert_eq!(match_nodes(&Selector::parse("table_def"), &graph).len(), 1);
    }

    #[test]
    fn matches_by_attr() {
        let graph = g("table T { n: Int }");
        let m = match_nodes(&Selector::parse("[schema~=Int]"), &graph);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn containment_chain() {
        // CALL nodes that live inside a FN
        let graph = g("fn f(a) { a + a } let z = 9");
        let inside = match_nodes(&Selector::parse("kind(FN) kind(CALL)"), &graph);
        assert!(!inside.is_empty());
        // the top-level `9` literal is not a CALL, so no false hits from it
        for n in &inside {
            assert_eq!(n.kind, NodeKind::Call);
        }
    }

    #[test]
    fn specificity_orders() {
        assert!(Selector::parse("[k=v]").specificity() > Selector::parse("*").specificity());
        assert!(
            Selector::parse("kind(FN) kind(CALL)").specificity()
                > Selector::parse("kind(CALL)").specificity()
        );
    }

    #[test]
    fn parsing_is_total() {
        // garbage never panics; degrades to a kind match or any
        let _ = Selector::parse("");
        let _ = Selector::parse("@#$%");
        let _ = Selector::parse("[unclosed");
    }
}
