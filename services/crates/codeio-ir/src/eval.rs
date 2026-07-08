//! Execute directly FROM the materialized IR graph (not the AST).
//! This closes the IR<->execution loop: the graph is the live program, so anything that edits
//! the graph (GUI, blueprint, AI, adapters) changes what runs. v0 covers the arithmetic/literal/
//! ref/call core; full parity with the tree-walker grows as the IR carries more (bindings scope).

use crate::{Graph, Node, NodeKind};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum IrValue { Int(i64), Float(f64), Str(String), Bool(bool), Nil }

impl std::fmt::Display for IrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IrValue::Int(n)=>write!(f,"{n}"), IrValue::Float(x)=>write!(f,"{x}"),
            IrValue::Str(s)=>write!(f,"{s}"), IrValue::Bool(b)=>write!(f,"{b}"), IrValue::Nil=>write!(f,"nil"),
        }
    }
}

/// Evaluate a single node by id, resolving children from the graph. Pure over (graph, env).
pub fn eval_node(g: &Graph, id: &str, env: &HashMap<String, IrValue>) -> Result<IrValue, String> {
    let n = g.get(id).ok_or_else(|| format!("node {id} not found"))?;
    match n.kind {
        NodeKind::Literal => eval_literal(n),
        NodeKind::Ref => {
            let name = n.attrs.get("name").ok_or("ref without name")?;
            env.get(name).cloned().ok_or_else(|| format!("unbound ref '{name}'"))
        }
        NodeKind::Call => eval_call(g, n, env),
        _ => Err(format!("IR eval v0 does not yet execute {} nodes", n.kind.as_str())),
    }
}

fn eval_literal(n: &Node) -> Result<IrValue, String> {
    let ty = n.attrs.get("type").map(|s| s.as_str()).unwrap_or("Nil");
    let v = n.attrs.get("value").cloned().unwrap_or_default();
    Ok(match ty {
        "Int" => IrValue::Int(v.parse().map_err(|_| "bad int")?),
        "Float" => IrValue::Float(v.parse().map_err(|_| "bad float")?),
        "Str" => IrValue::Str(v),
        "Bool" => IrValue::Bool(v == "true"),
        _ => IrValue::Nil,
    })
}

fn eval_call(g: &Graph, n: &Node, env: &HashMap<String, IrValue>) -> Result<IrValue, String> {
    let op = n.attrs.get("op").map(|s| s.as_str()).unwrap_or("");
    // binary arithmetic/comparison over two children
    if n.children.len() == 2 {
        let a = eval_node(g, &n.children[0], env)?;
        let b = eval_node(g, &n.children[1], env)?;
        return apply_binary(op, a, b);
    }
    if n.children.len() == 1 && op == "Minus" {
        return match eval_node(g, &n.children[0], env)? {
            IrValue::Int(x) => Ok(IrValue::Int(-x)),
            IrValue::Float(x) => Ok(IrValue::Float(-x)),
            v => Err(format!("cannot negate {v}")),
        };
    }
    Err(format!("IR eval v0: unsupported call op '{op}' with {} children", n.children.len()))
}

fn apply_binary(op: &str, a: IrValue, b: IrValue) -> Result<IrValue, String> {
    use IrValue::*;
    Ok(match (op, &a, &b) {
        ("Plus", Int(x), Int(y)) => Int(x + y),
        ("Minus", Int(x), Int(y)) => Int(x - y),
        ("Star", Int(x), Int(y)) => Int(x * y),
        ("Slash", Int(x), Int(y)) if *y != 0 => Int(x / y),
        ("Plus", Str(x), Str(y)) => Str(format!("{x}{y}")),
        ("EqEq", _, _) => Bool(a == b),
        ("Lt", Int(x), Int(y)) => Bool(x < y),
        ("Gt", Int(x), Int(y)) => Bool(x > y),
        _ => return Err(format!("IR eval v0: unsupported {op} on {a},{b}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower;
    use codeio_lang::{lexer::Lexer, parser::Parser};

    fn last_root_val(src: &str) -> Result<IrValue, String> {
        let toks = Lexer::new(src).tokenize().unwrap();
        let g = lower(&Parser::new(toks).parse_program().unwrap());
        let root = g.roots.last().unwrap();
        eval_node(&g, root, &HashMap::new())
    }

    #[test]
    fn executes_from_graph() {
        assert_eq!(last_root_val("1 + 2 * 3").unwrap(), IrValue::Int(7));
        assert_eq!(last_root_val("10 - 4").unwrap(), IrValue::Int(6));
        assert_eq!(last_root_val("\"a\" + \"b\"").unwrap(), IrValue::Str("ab".into()));
        assert_eq!(last_root_val("5 > 3").unwrap(), IrValue::Bool(true));
    }

    #[test]
    fn refs_resolve_from_env() {
        let toks = Lexer::new("x + 1").tokenize().unwrap();
        let g = lower(&Parser::new(toks).parse_program().unwrap());
        let mut env = HashMap::new();
        env.insert("x".to_string(), IrValue::Int(41));
        assert_eq!(eval_node(&g, g.roots.last().unwrap(), &env).unwrap(), IrValue::Int(42));
    }
}
