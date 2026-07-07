//! Tree-walking evaluator for CodeIO v0.

use crate::lexer::Tok;
use crate::parser::{Expr, Stmt};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Fn(Rc<FnDef>),
    Builtin(&'static str, fn(&[Value]) -> Result<Value, String>),
}

pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub env: EnvRef,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => write!(f, "nil"),
            Value::Fn(d) => write!(f, "<fn {}>", d.name),
            Value::Builtin(n, _) => write!(f, "<builtin {n}>"),
        }
    }
}

pub type EnvRef = Rc<RefCell<Env>>;

pub struct Env {
    vars: HashMap<String, (Value, bool)>, // (value, mutable)
    parent: Option<EnvRef>,
}

impl Env {
    pub fn root() -> EnvRef {
        let mut vars = HashMap::new();
        let b = |n: &'static str, f: fn(&[Value]) -> Result<Value, String>| (Value::Builtin(n, f), false);
        vars.insert("print".into(), b("print", builtin_print));
        vars.insert("len".into(), b("len", builtin_len));
        vars.insert("str".into(), b("str", builtin_str));
        vars.insert("abs".into(), b("abs", builtin_abs));
        vars.insert("min".into(), b("min", builtin_min));
        vars.insert("max".into(), b("max", builtin_max));
        Rc::new(RefCell::new(Env { vars, parent: None }))
    }
    pub fn child(parent: &EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Env { vars: HashMap::new(), parent: Some(parent.clone()) }))
    }
    fn get(&self, name: &str) -> Option<Value> {
        if let Some((v, _)) = self.vars.get(name) {
            Some(v.clone())
        } else {
            self.parent.as_ref().and_then(|p| p.borrow().get(name))
        }
    }
    fn define(&mut self, name: String, v: Value, mutable: bool) {
        self.vars.insert(name, (v, mutable));
    }
    fn assign(&mut self, name: &str, v: Value) -> Result<(), String> {
        if let Some((slot, mutable)) = self.vars.get_mut(name) {
            if !*mutable {
                return Err(format!("cannot assign to immutable binding '{name}' (use var)"));
            }
            *slot = v;
            Ok(())
        } else if let Some(p) = &self.parent {
            p.borrow_mut().assign(name, v)
        } else {
            Err(format!("undefined variable '{name}'"))
        }
    }
}

fn builtin_print(args: &[Value]) -> Result<Value, String> {
    let out: Vec<String> = args.iter().map(|v| v.to_string()).collect();
    println!("{}", out.join(" "));
    Ok(Value::Nil)
}
fn builtin_len(args: &[Value]) -> Result<Value, String> {
    match args {
        [Value::Str(s)] => Ok(Value::Int(s.chars().count() as i64)),
        _ => Err("len expects one string".into()),
    }
}
fn builtin_str(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("")))
}
fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    match args {
        [Value::Int(n)] => Ok(Value::Int(n.abs())),
        [Value::Float(x)] => Ok(Value::Float(x.abs())),
        _ => Err("abs expects one number".into()),
    }
}
fn num2(args: &[Value]) -> Result<(f64, f64, bool), String> {
    match args {
        [Value::Int(a), Value::Int(b)] => Ok((*a as f64, *b as f64, true)),
        [a, b] => Ok((as_f64(a)?, as_f64(b)?, false)),
        _ => Err("expected two numbers".into()),
    }
}
fn builtin_min(args: &[Value]) -> Result<Value, String> {
    let (a, b, int) = num2(args)?;
    let m = a.min(b);
    Ok(if int { Value::Int(m as i64) } else { Value::Float(m) })
}
fn builtin_max(args: &[Value]) -> Result<Value, String> {
    let (a, b, int) = num2(args)?;
    let m = a.max(b);
    Ok(if int { Value::Int(m as i64) } else { Value::Float(m) })
}
fn as_f64(v: &Value) -> Result<f64, String> {
    match v {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(x) => Ok(*x),
        other => Err(format!("expected number, found {other}")),
    }
}

enum Flow {
    Normal(Value),
    Return(Value),
}

pub struct Interp;

impl Interp {
    pub fn run(stmts: &[Stmt], env: &EnvRef) -> Result<Value, String> {
        match Self::exec_block(stmts, env)? {
            Flow::Normal(v) | Flow::Return(v) => Ok(v),
        }
    }

    fn exec_block(stmts: &[Stmt], env: &EnvRef) -> Result<Flow, String> {
        let mut last = Value::Nil;
        for s in stmts {
            match Self::exec(s, env)? {
                Flow::Return(v) => return Ok(Flow::Return(v)),
                Flow::Normal(v) => last = v,
            }
        }
        Ok(Flow::Normal(last))
    }

    fn exec(stmt: &Stmt, env: &EnvRef) -> Result<Flow, String> {
        Ok(match stmt {
            Stmt::Let(name, e) => {
                let v = Self::eval(e, env)?;
                env.borrow_mut().define(name.clone(), v, false);
                Flow::Normal(Value::Nil)
            }
            Stmt::Var(name, e) => {
                let v = Self::eval(e, env)?;
                env.borrow_mut().define(name.clone(), v, true);
                Flow::Normal(Value::Nil)
            }
            Stmt::Assign(name, e) => {
                let v = Self::eval(e, env)?;
                env.borrow_mut().assign(name, v)?;
                Flow::Normal(Value::Nil)
            }
            Stmt::Fn(name, params, body) => {
                let def = FnDef {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    env: env.clone(),
                };
                env.borrow_mut().define(name.clone(), Value::Fn(Rc::new(def)), false);
                Flow::Normal(Value::Nil)
            }
            Stmt::While(cond, body) => {
                while truthy(&Self::eval(cond, env)?) {
                    let child = Env::child(env);
                    if let Flow::Return(v) = Self::exec_block(body, &child)? {
                        return Ok(Flow::Return(v));
                    }
                }
                Flow::Normal(Value::Nil)
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(e) => Self::eval(e, env)?,
                    None => Value::Nil,
                };
                Flow::Return(v)
            }
            Stmt::Expr(e) => Self::eval_flow(e, env)?,
        })
    }

    fn eval_flow(expr: &Expr, env: &EnvRef) -> Result<Flow, String> {
        Ok(match expr {
            Expr::If(cond, then, els) => {
                if truthy(&Self::eval(cond, env)?) {
                    Self::exec_block(then, &Env::child(env))?
                } else if let Some(els) = els {
                    Self::exec_block(els, &Env::child(env))?
                } else {
                    Flow::Normal(Value::Nil)
                }
            }
            Expr::Block(stmts) => Self::exec_block(stmts, &Env::child(env))?,
            other => Flow::Normal(Self::eval(other, env)?),
        })
    }

    pub fn eval(expr: &Expr, env: &EnvRef) -> Result<Value, String> {
        Ok(match expr {
            Expr::Int(n) => Value::Int(*n),
            Expr::Float(x) => Value::Float(*x),
            Expr::Str(s) => Value::Str(s.clone()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Nil => Value::Nil,
            Expr::Ident(name) => env
                .borrow()
                .get(name)
                .ok_or_else(|| format!("undefined variable '{name}'"))?,
            Expr::Unary(op, e) => {
                let v = Self::eval(e, env)?;
                match (op, v) {
                    (Tok::Minus, Value::Int(n)) => Value::Int(-n),
                    (Tok::Minus, Value::Float(x)) => Value::Float(-x),
                    (Tok::Bang, v) => Value::Bool(!truthy(&v)),
                    (op, v) => return Err(format!("bad unary {op:?} on {v}")),
                }
            }
            Expr::Binary(op, a, b) => {
                if *op == Tok::AndAnd {
                    let l = Self::eval(a, env)?;
                    return if !truthy(&l) { Ok(Value::Bool(false)) } else { Ok(Value::Bool(truthy(&Self::eval(b, env)?))) };
                }
                if *op == Tok::OrOr {
                    let l = Self::eval(a, env)?;
                    return if truthy(&l) { Ok(Value::Bool(true)) } else { Ok(Value::Bool(truthy(&Self::eval(b, env)?))) };
                }
                let l = Self::eval(a, env)?;
                let r = Self::eval(b, env)?;
                binop(op, l, r)?
            }
            Expr::Call(f, args) => {
                let callee = Self::eval(f, env)?;
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(Self::eval(a, env)?);
                }
                match callee {
                    Value::Builtin(_, f) => f(&vals)?,
                    Value::Fn(def) => {
                        if vals.len() != def.params.len() {
                            return Err(format!(
                                "fn {} expects {} args, got {}",
                                def.name,
                                def.params.len(),
                                vals.len()
                            ));
                        }
                        let child = Env::child(&def.env);
                        for (p, v) in def.params.iter().zip(vals) {
                            child.borrow_mut().define(p.clone(), v, false);
                        }
                        match Self::exec_block(&def.body, &child)? {
                            Flow::Normal(v) | Flow::Return(v) => v,
                        }
                    }
                    other => return Err(format!("{other} is not callable")),
                }
            }
            Expr::If(cond, then, els) => {
                if truthy(&Self::eval(cond, env)?) {
                    match Self::exec_block(then, &Env::child(env))? {
                        Flow::Return(_) => return Err("`return` not allowed in value position (v0)".into()),
                        Flow::Normal(v) => v,
                    }
                } else if let Some(els) = els {
                    match Self::exec_block(els, &Env::child(env))? {
                        Flow::Return(_) => return Err("`return` not allowed in value position (v0)".into()),
                        Flow::Normal(v) => v,
                    }
                } else {
                    Value::Nil
                }
            }
            Expr::Block(stmts) => match Self::exec_block(stmts, &Env::child(env))? {
                Flow::Normal(v) | Flow::Return(v) => v,
            },
        })
    }
}

fn truthy(v: &Value) -> bool {
    !matches!(v, Value::Bool(false) | Value::Nil)
}

fn binop(op: &Tok, l: Value, r: Value) -> Result<Value, String> {
    use Value::*;
    Ok(match (op, &l, &r) {
        (Tok::Plus, Int(a), Int(b)) => Int(a + b),
        (Tok::Minus, Int(a), Int(b)) => Int(a - b),
        (Tok::Star, Int(a), Int(b)) => Int(a * b),
        (Tok::Percent, Int(a), Int(b)) => {
            if *b == 0 { return Err("modulo by zero".into()); }
            Int(a % b)
        }
        (Tok::Slash, Int(a), Int(b)) => {
            if *b == 0 { return Err("division by zero".into()); }
            Int(a / b)
        }
        (Tok::Plus, Str(a), b) => Str(format!("{a}{b}")),
        (Tok::Plus, a, Str(b)) => Str(format!("{a}{b}")),
        (Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash, _, _) => {
            let (a, b) = (as_f64(&l)?, as_f64(&r)?);
            match op {
                Tok::Plus => Float(a + b),
                Tok::Minus => Float(a - b),
                Tok::Star => Float(a * b),
                _ => {
                    if b == 0.0 { return Err("division by zero".into()); }
                    Float(a / b)
                }
            }
        }
        (Tok::EqEq, _, _) => Bool(val_eq(&l, &r)),
        (Tok::NotEq, _, _) => Bool(!val_eq(&l, &r)),
        (Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge, _, _) => {
            let (a, b) = match (&l, &r) {
                (Str(a), Str(b)) => return Ok(Bool(cmp_ord(op, a.cmp(b)))),
                _ => (as_f64(&l)?, as_f64(&r)?),
            };
            Bool(cmp_ord(op, a.partial_cmp(&b).ok_or("NaN comparison")?))
        }
        _ => return Err(format!("unsupported operation {op:?} on {l} and {r}")),
    })
}

fn cmp_ord(op: &Tok, o: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering::*;
    matches!(
        (op, o),
        (Tok::Lt, Less) | (Tok::Le, Less | Equal) | (Tok::Gt, Greater) | (Tok::Ge, Greater | Equal)
    )
}

fn val_eq(a: &Value, b: &Value) -> bool {
    use Value::*;
    match (a, b) {
        (Int(x), Int(y)) => x == y,
        (Float(x), Float(y)) => x == y,
        (Int(x), Float(y)) | (Float(y), Int(x)) => *x as f64 == *y,
        (Str(x), Str(y)) => x == y,
        (Bool(x), Bool(y)) => x == y,
        (Nil, Nil) => true,
        _ => false,
    }
}
