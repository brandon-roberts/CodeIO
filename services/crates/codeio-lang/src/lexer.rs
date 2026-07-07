//! Lexer for CodeIO v0 (spec: docs/language-spec.md).

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),
    // keywords
    Let, Var, Fn, If, Else, While, For, In, Return, True, False, Nil,
    Table, Insert, From, Where, Select,
    // symbols
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Comma, Colon, Arrow, Pipe,
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, NotEq, Lt, Le, Gt, Ge, Bang, AndAnd, OrOr, Dot,
    Eof,
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    pub line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0, line: 1 }
    }

    fn peek(&self) -> u8 {
        *self.src.get(self.pos).unwrap_or(&0)
    }
    fn peek2(&self) -> u8 {
        *self.src.get(self.pos + 1).unwrap_or(&0)
    }
    fn bump(&mut self) -> u8 {
        let c = self.peek();
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
        }
        c
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.peek() {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.bump();
                }
                b'/' if self.peek2() == b'/' => {
                    while self.peek() != b'\n' && self.peek() != 0 {
                        self.bump();
                    }
                }
                b'/' if self.peek2() == b'*' => {
                    self.bump();
                    self.bump();
                    let mut depth = 1;
                    while depth > 0 && self.peek() != 0 {
                        if self.peek() == b'/' && self.peek2() == b'*' {
                            depth += 1;
                            self.bump();
                        } else if self.peek() == b'*' && self.peek2() == b'/' {
                            depth -= 1;
                            self.bump();
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    pub fn next_tok(&mut self) -> Result<Tok, String> {
        self.skip_ws_and_comments();
        let line = self.line;
        let c = self.peek();
        Ok(match c {
            0 => Tok::Eof,
            b'(' => { self.bump(); Tok::LParen }
            b')' => { self.bump(); Tok::RParen }
            b'{' => { self.bump(); Tok::LBrace }
            b'}' => { self.bump(); Tok::RBrace }
            b'[' => { self.bump(); Tok::LBracket }
            b']' => { self.bump(); Tok::RBracket }
            b',' => { self.bump(); Tok::Comma }
            b'.' => { self.bump(); Tok::Dot }
            b':' => { self.bump(); Tok::Colon }
            b'+' => { self.bump(); Tok::Plus }
            b'*' => { self.bump(); Tok::Star }
            b'/' => { self.bump(); Tok::Slash }
            b'%' => { self.bump(); Tok::Percent }
            b'-' => {
                self.bump();
                if self.peek() == b'>' { self.bump(); Tok::Arrow } else { Tok::Minus }
            }
            b'|' => {
                self.bump();
                match self.peek() {
                    b'>' => { self.bump(); Tok::Pipe }
                    b'|' => { self.bump(); Tok::OrOr }
                    _ => return Err(format!("line {line}: unexpected '|'")),
                }
            }
            b'&' if self.peek2() == b'&' => { self.bump(); self.bump(); Tok::AndAnd }
            b'=' => {
                self.bump();
                if self.peek() == b'=' { self.bump(); Tok::EqEq } else { Tok::Eq }
            }
            b'!' => {
                self.bump();
                if self.peek() == b'=' { self.bump(); Tok::NotEq } else { Tok::Bang }
            }
            b'<' => {
                self.bump();
                if self.peek() == b'=' { self.bump(); Tok::Le } else { Tok::Lt }
            }
            b'>' => {
                self.bump();
                if self.peek() == b'=' { self.bump(); Tok::Ge } else { Tok::Gt }
            }
            b'"' => {
                self.bump();
                let mut s = String::new();
                loop {
                    match self.bump() {
                        0 => return Err(format!("line {line}: unterminated string")),
                        b'"' => break,
                        b'\\' => match self.bump() {
                            b'n' => s.push('\n'),
                            b't' => s.push('\t'),
                            b'"' => s.push('"'),
                            b'\\' => s.push('\\'),
                            other => return Err(format!("line {line}: bad escape \\{}", other as char)),
                        },
                        b => s.push(b as char),
                    }
                }
                Tok::Str(s)
            }
            b'0'..=b'9' => {
                let start = self.pos;
                while self.peek().is_ascii_digit() || self.peek() == b'_' {
                    self.bump();
                }
                let mut is_float = false;
                if self.peek() == b'.' && self.peek2().is_ascii_digit() {
                    is_float = true;
                    self.bump();
                    while self.peek().is_ascii_digit() {
                        self.bump();
                    }
                }
                let text: String = std::str::from_utf8(&self.src[start..self.pos])
                    .unwrap()
                    .replace('_', "");
                if is_float {
                    Tok::Float(text.parse().map_err(|e| format!("line {line}: {e}"))?)
                } else {
                    Tok::Int(text.parse().map_err(|e| format!("line {line}: {e}"))?)
                }
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.pos;
                while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' {
                    self.bump();
                }
                let word = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
                match word {
                    "let" => Tok::Let,
                    "var" => Tok::Var,
                    "fn" => Tok::Fn,
                    "if" => Tok::If,
                    "else" => Tok::Else,
                    "while" => Tok::While,
                    "for" => Tok::For,
                    "table" => Tok::Table,
                    "insert" => Tok::Insert,
                    "from" => Tok::From,
                    "where" => Tok::Where,
                    "select" => Tok::Select,
                    "in" => Tok::In,
                    "return" => Tok::Return,
                    "true" => Tok::True,
                    "false" => Tok::False,
                    "nil" => Tok::Nil,
                    _ => Tok::Ident(word.to_string()),
                }
            }
            other => return Err(format!("line {line}: unexpected character '{}'", other as char)),
        })
    }

    pub fn tokenize(mut self) -> Result<Vec<(Tok, usize)>, String> {
        let mut out = Vec::new();
        loop {
            let line = self.line;
            let t = self.next_tok()?;
            let done = t == Tok::Eof;
            out.push((t, line));
            if done {
                return Ok(out);
            }
        }
    }
}
