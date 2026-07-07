//! Parser for CodeIO v0 — Pratt expression parsing, statement-list blocks.

use crate::lexer::Tok;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Ident(String),
    Unary(Tok, Box<Expr>),
    Binary(Tok, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    If(Box<Expr>, Vec<Stmt>, Option<Vec<Stmt>>),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Expr),
    Var(String, Expr),
    Assign(String, Expr),
    Fn(String, Vec<String>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Return(Option<Expr>),
    Expr(Expr),
}

pub struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<(Tok, usize)>) -> Self {
        Parser { toks, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }
    fn line(&self) -> usize {
        self.toks[self.pos].1
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, t: Tok) -> Result<(), String> {
        if *self.peek() == t {
            self.bump();
            Ok(())
        } else {
            Err(format!("line {}: expected {:?}, found {:?}", self.line(), t, self.peek()))
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while *self.peek() != Tok::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Tok::LBrace)?;
        let mut stmts = Vec::new();
        while *self.peek() != Tok::RBrace && *self.peek() != Tok::Eof {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Tok::Let | Tok::Var => {
                let is_let = *self.peek() == Tok::Let;
                self.bump();
                let name = self.ident()?;
                // optional type annotation `: Ident` — parsed and (v0) ignored
                if *self.peek() == Tok::Colon {
                    self.bump();
                    self.ident()?;
                }
                self.expect(Tok::Eq)?;
                let e = self.parse_expr(0)?;
                Ok(if is_let { Stmt::Let(name, e) } else { Stmt::Var(name, e) })
            }
            Tok::Fn => {
                self.bump();
                let name = self.ident()?;
                self.expect(Tok::LParen)?;
                let mut params = Vec::new();
                while *self.peek() != Tok::RParen {
                    params.push(self.ident()?);
                    if *self.peek() == Tok::Colon {
                        self.bump();
                        self.ident()?; // param type, ignored in v0
                    }
                    if *self.peek() == Tok::Comma {
                        self.bump();
                    }
                }
                self.expect(Tok::RParen)?;
                if *self.peek() == Tok::Arrow {
                    self.bump();
                    self.ident()?; // return type, ignored in v0
                }
                let body = self.parse_block()?;
                Ok(Stmt::Fn(name, params, body))
            }
            Tok::While => {
                self.bump();
                let cond = self.parse_expr(0)?;
                let body = self.parse_block()?;
                Ok(Stmt::While(cond, body))
            }
            Tok::Return => {
                self.bump();
                if matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr(0)?)))
                }
            }
            Tok::Ident(name) => {
                // assignment or expression
                if self.toks.get(self.pos + 1).map(|t| &t.0) == Some(&Tok::Eq) {
                    self.bump();
                    self.bump();
                    let e = self.parse_expr(0)?;
                    Ok(Stmt::Assign(name, e))
                } else {
                    Ok(Stmt::Expr(self.parse_expr(0)?))
                }
            }
            _ => Ok(Stmt::Expr(self.parse_expr(0)?)),
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        if let Tok::Ident(s) = self.peek().clone() {
            self.bump();
            Ok(s)
        } else {
            Err(format!("line {}: expected identifier, found {:?}", self.line(), self.peek()))
        }
    }

    fn prefix(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Tok::Int(n) => Ok(Expr::Int(n)),
            Tok::Float(f) => Ok(Expr::Float(f)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::True => Ok(Expr::Bool(true)),
            Tok::False => Ok(Expr::Bool(false)),
            Tok::Nil => Ok(Expr::Nil),
            Tok::Ident(s) => Ok(Expr::Ident(s)),
            Tok::Minus => Ok(Expr::Unary(Tok::Minus, Box::new(self.parse_expr(8)?))),
            Tok::Bang => Ok(Expr::Unary(Tok::Bang, Box::new(self.parse_expr(8)?))),
            Tok::LParen => {
                let e = self.parse_expr(0)?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Tok::If => {
                let cond = self.parse_expr(0)?;
                let then = self.parse_block()?;
                let els = if *self.peek() == Tok::Else {
                    self.bump();
                    if *self.peek() == Tok::If {
                        // else if — wrap as single-stmt block
                        let e = self.prefix()?; // parses the nested If expression
                        Some(vec![Stmt::Expr(e)])
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                Ok(Expr::If(Box::new(cond), then, els))
            }
            Tok::LBrace => {
                // block expression: rewind one and use parse_block
                self.pos -= 1;
                Ok(Expr::Block(self.parse_block()?))
            }
            t => Err(format!("line {}: unexpected token {:?}", self.line(), t)),
        }
    }

    fn infix_bp(t: &Tok) -> Option<(u8, u8)> {
        Some(match t {
            Tok::OrOr => (1, 2),
            Tok::AndAnd => (2, 3),
            Tok::EqEq | Tok::NotEq => (3, 4),
            Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge => (4, 5),
            Tok::Pipe => (5, 6),
            Tok::Plus | Tok::Minus => (6, 7),
            Tok::Star | Tok::Slash | Tok::Percent => (7, 8),
            _ => return None,
        })
    }

    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.prefix()?;
        loop {
            // call postfix
            while *self.peek() == Tok::LParen {
                self.bump();
                let mut args = Vec::new();
                while *self.peek() != Tok::RParen {
                    args.push(self.parse_expr(0)?);
                    if *self.peek() == Tok::Comma {
                        self.bump();
                    }
                }
                self.expect(Tok::RParen)?;
                lhs = Expr::Call(Box::new(lhs), args);
            }
            let op = self.peek().clone();
            let Some((lbp, rbp)) = Self::infix_bp(&op) else { break };
            if lbp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr(rbp)?;
            if op == Tok::Pipe {
                // a |> f(b, c)  =>  f(a, b, c);  a |> f  =>  f(a)
                lhs = match rhs {
                    Expr::Call(f, mut args) => {
                        args.insert(0, lhs);
                        Expr::Call(f, args)
                    }
                    f => Expr::Call(Box::new(f), vec![lhs]),
                };
            } else {
                lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
            }
        }
        Ok(lhs)
    }
}
