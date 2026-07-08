//! codeio-ir — materializes the AST into the canonical content-addressed IR (proto/ir/ir.proto).
//! Every node carries full metadata: kind, children (by content-address), attrs, and provenance.

use codeio_lang::parser::{Expr, Stmt};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Literal, Ref, Call, Fn, Match, Record, TableDef, Query, AiCall, Quote, Splice, Effect,
}
impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Literal => "LITERAL", NodeKind::Ref => "REF", NodeKind::Call => "CALL",
            NodeKind::Fn => "FN", NodeKind::Match => "MATCH", NodeKind::Record => "RECORD",
            NodeKind::TableDef => "TABLE_DEF", NodeKind::Query => "QUERY", NodeKind::AiCall => "AI_CALL",
            NodeKind::Quote => "QUOTE", NodeKind::Splice => "SPLICE", NodeKind::Effect => "EFFECT",
        }
    }
}

/// Provenance — the P9 developer-as-author record mapped onto every node.
#[derive(Debug, Clone)]
pub struct Provenance {
    pub author_kind: String,   // "human" | "ai" | "lifted"
    pub author: String,
    pub authority_ref: String, // conformance reference
    pub source_lang: String,
}
impl Default for Provenance {
    fn default() -> Self {
        Provenance {
            author_kind: "human".into(),
            author: "developer".into(),
            authority_ref: "docs/language-spec.md".into(),
            source_lang: "codeio".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,               // sha256 of canonical encoding
    pub kind: NodeKind,
    pub children: Vec<String>,    // child ids — structure by reference
    pub attrs: BTreeMap<String, String>,
    pub provenance: Provenance,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub nodes: BTreeMap<String, Node>,  // id -> node (dedup by content address)
    pub roots: Vec<String>,
    pub meta: BTreeMap<String, String>,
}

impl Graph {
    pub fn len(&self) -> usize { self.nodes.len() }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }
    pub fn get(&self, id: &str) -> Option<&Node> { self.nodes.get(id) }

    /// Insert a node, computing its content address. Deduplicates: identical content = same id.
    fn intern(&mut self, kind: NodeKind, children: Vec<String>, attrs: BTreeMap<String, String>, prov: Provenance) -> String {
        let mut hasher = Sha256::new();
        hasher.update(kind.as_str().as_bytes());
        for c in &children { hasher.update(b"|"); hasher.update(c.as_bytes()); }
        for (k, v) in &attrs { hasher.update(b"|"); hasher.update(k.as_bytes()); hasher.update(b"="); hasher.update(v.as_bytes()); }
        let id = format!("{:x}", hasher.finalize());
        let short = id[..16].to_string();
        self.nodes.entry(short.clone()).or_insert(Node {
            id: short.clone(), kind, children, attrs, provenance: prov,
        });
        short
    }

    /// Query: all node ids of a given kind.
    pub fn by_kind(&self, kind: &NodeKind) -> Vec<&Node> {
        self.nodes.values().filter(|n| &n.kind == kind).collect()
    }

    /// Query: count nodes by kind (the self-analysis surface, P2).
    pub fn kind_histogram(&self) -> BTreeMap<String, usize> {
        let mut h = BTreeMap::new();
        for n in self.nodes.values() {
            *h.entry(n.kind.as_str().to_string()).or_insert(0) += 1;
        }
        h
    }
}

/// Lower a parsed program into the materialized IR graph.
pub fn lower(stmts: &[Stmt]) -> Graph {
    let mut g = Graph::default();
    let prov = Provenance::default();
    for s in stmts {
        let id = lower_stmt(&mut g, s, &prov);
        g.roots.push(id);
    }
    g.meta.insert("node_count".into(), g.len().to_string());
    g
}

fn attr(k: &str, v: impl Into<String>) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert(k.to_string(), v.into());
    m
}

fn lower_stmt(g: &mut Graph, s: &Stmt, p: &Provenance) -> String {
    match s {
        Stmt::Let(name, e) | Stmt::Var(name, e) => {
            let child = lower_expr(g, e, p);
            let mut a = attr("binding", name.clone());
            a.insert("mutable".into(), matches!(s, Stmt::Var(..)).to_string());
            g.intern(NodeKind::Effect, vec![child], a, p.clone())
        }
        Stmt::Assign(name, e) => {
            let child = lower_expr(g, e, p);
            g.intern(NodeKind::Effect, vec![child], attr("assign", name.clone()), p.clone())
        }
        Stmt::Fn(name, params, body) => {
            let body_ids: Vec<String> = body.iter().map(|b| lower_stmt(g, b, p)).collect();
            let mut a = attr("name", name.clone());
            a.insert("params".into(), params.join(","));
            g.intern(NodeKind::Fn, body_ids, a, p.clone())
        }
        Stmt::While(cond, body) => {
            let c = lower_expr(g, cond, p);
            let mut children = vec![c];
            children.extend(body.iter().map(|b| lower_stmt(g, b, p)));
            g.intern(NodeKind::Match, children, attr("form", "while"), p.clone())
        }
        Stmt::For(var, iter, body) => {
            let it = lower_expr(g, iter, p);
            let mut children = vec![it];
            children.extend(body.iter().map(|b| lower_stmt(g, b, p)));
            g.intern(NodeKind::Match, children, {
                let mut a = attr("form", "for"); a.insert("var".into(), var.clone()); a
            }, p.clone())
        }
        Stmt::Return(e) => {
            let children = e.as_ref().map(|e| vec![lower_expr(g, e, p)]).unwrap_or_default();
            g.intern(NodeKind::Effect, children, attr("effect", "return"), p.clone())
        }
        Stmt::Table(name, cols) => {
            let schema: Vec<String> = cols.iter().map(|(c, t)| format!("{c}:{t}")).collect();
            let mut a = attr("name", name.clone());
            a.insert("schema".into(), schema.join(","));
            g.intern(NodeKind::TableDef, vec![], a, p.clone())
        }
        Stmt::Insert(name, rec) => {
            let r = lower_expr(g, rec, p);
            g.intern(NodeKind::Effect, vec![r], attr("insert", name.clone()), p.clone())
        }
        Stmt::Expr(e) => lower_expr(g, e, p),
    }
}

fn lower_expr(g: &mut Graph, e: &Expr, p: &Provenance) -> String {
    match e {
        Expr::Int(n) => g.intern(NodeKind::Literal, vec![], { let mut a=attr("value", n.to_string()); a.insert("type".into(),"Int".into()); a }, p.clone()),
        Expr::Float(x) => g.intern(NodeKind::Literal, vec![], { let mut a=attr("value", x.to_string()); a.insert("type".into(),"Float".into()); a }, p.clone()),
        Expr::Str(s) => g.intern(NodeKind::Literal, vec![], { let mut a=attr("value", s.clone()); a.insert("type".into(),"Str".into()); a }, p.clone()),
        Expr::Bool(b) => g.intern(NodeKind::Literal, vec![], { let mut a=attr("value", b.to_string()); a.insert("type".into(),"Bool".into()); a }, p.clone()),
        Expr::Nil => g.intern(NodeKind::Literal, vec![], attr("type", "Nil"), p.clone()),
        Expr::Ident(name) => g.intern(NodeKind::Ref, vec![], attr("name", name.clone()), p.clone()),
        Expr::Unary(op, e) => {
            let c = lower_expr(g, e, p);
            g.intern(NodeKind::Call, vec![c], attr("op", format!("{op:?}")), p.clone())
        }
        Expr::Binary(op, a, b) => {
            let l = lower_expr(g, a, p);
            let r = lower_expr(g, b, p);
            g.intern(NodeKind::Call, vec![l, r], attr("op", format!("{op:?}")), p.clone())
        }
        Expr::Call(f, args) => {
            let fid = lower_expr(g, f, p);
            let mut children = vec![fid];
            children.extend(args.iter().map(|a| lower_expr(g, a, p)));
            g.intern(NodeKind::Call, children, attr("form", "apply"), p.clone())
        }
        Expr::List(items) => {
            let children: Vec<String> = items.iter().map(|i| lower_expr(g, i, p)).collect();
            g.intern(NodeKind::Record, children, attr("form", "list"), p.clone())
        }
        Expr::Record(fields) => {
            let children: Vec<String> = fields.iter().map(|(_, v)| lower_expr(g, v, p)).collect();
            let keys: Vec<String> = fields.iter().map(|(k, _)| k.clone()).collect();
            g.intern(NodeKind::Record, children, attr("keys", keys.join(",")), p.clone())
        }
        Expr::Field(t, name) => {
            let c = lower_expr(g, t, p);
            g.intern(NodeKind::Ref, vec![c], attr("field", name.clone()), p.clone())
        }
        Expr::Index(t, i) => {
            let a = lower_expr(g, t, p);
            let b = lower_expr(g, i, p);
            g.intern(NodeKind::Call, vec![a, b], attr("op", "index"), p.clone())
        }
        Expr::Query { var, source, filter, select } => {
            let mut children = vec![];
            let mut a = attr("var", var.clone());
            a.insert("source".into(), source.clone());
            if let Some(f) = filter { children.push(lower_expr(g, f, p)); a.insert("has_filter".into(), "true".into()); }
            if let Some(s) = select { children.push(lower_expr(g, s, p)); a.insert("has_select".into(), "true".into()); }
            g.intern(NodeKind::Query, children, a, p.clone())
        }
        Expr::If(c, then, els) => {
            let cid = lower_expr(g, c, p);
            let mut children = vec![cid];
            children.extend(then.iter().map(|s| lower_stmt(g, s, p)));
            if let Some(e) = els { children.extend(e.iter().map(|s| lower_stmt(g, s, p))); }
            g.intern(NodeKind::Match, children, attr("form", "if"), p.clone())
        }
        Expr::Block(stmts) => {
            let children: Vec<String> = stmts.iter().map(|s| lower_stmt(g, s, p)).collect();
            g.intern(NodeKind::Effect, children, attr("form", "block"), p.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codeio_lang::{lexer::Lexer, parser::Parser};

    fn graph(src: &str) -> Graph {
        let toks = Lexer::new(src).tokenize().unwrap();
        let stmts = Parser::new(toks).parse_program().unwrap();
        lower(&stmts)
    }

    #[test]
    fn materializes_nodes() {
        let g = graph("let x = 1 + 2");
        assert!(!g.is_empty());
        assert_eq!(g.by_kind(&NodeKind::Literal).len(), 2); // 1 and 2
        assert_eq!(g.by_kind(&NodeKind::Call).len(), 1);    // the +
    }

    #[test]
    fn content_addressing_dedups() {
        // identical subexpressions share one node id
        let g = graph("let a = 5 + 5");
        let lits = g.by_kind(&NodeKind::Literal);
        assert_eq!(lits.len(), 1); // both 5s dedup to one content-addressed node
    }

    #[test]
    fn provenance_on_every_node() {
        let g = graph("fn f(x) { x }");
        for n in g.nodes.values() {
            assert_eq!(n.provenance.author_kind, "human");
            assert_eq!(n.provenance.source_lang, "codeio");
            assert!(!n.provenance.authority_ref.is_empty());
        }
    }

    #[test]
    fn tables_and_queries_lower() {
        let g = graph("table T { n: Int } from r in T where r.n > 0 select r.n");
        assert_eq!(g.by_kind(&NodeKind::TableDef).len(), 1);
        assert_eq!(g.by_kind(&NodeKind::Query).len(), 1);
        let td = g.by_kind(&NodeKind::TableDef)[0];
        assert_eq!(td.attrs.get("schema").unwrap(), "n:Int");
    }

    #[test]
    fn histogram_self_analysis() {
        let g = graph("fn sq(n) { n * n } let a = sq(3)");
        let h = g.kind_histogram();
        assert!(h.get("FN").copied().unwrap_or(0) >= 1);
        assert!(h.get("CALL").copied().unwrap_or(0) >= 1);
    }
}
