//! codeio-lang — lexer, parser, and tree-walking interpreter for CodeIO v0.
//! Spec: docs/language-spec.md. IR alignment lands in the next milestone.

pub mod interp;
pub mod lexer;
pub mod parser;

use interp::{Env, EnvRef, Interp, Value};

/// Parse and run a source string in a fresh environment. Returns the last value.
pub fn run_source(src: &str) -> Result<Value, String> {
    let env = Env::root();
    run_in(src, &env)
}

/// Parse and run a source string in an existing environment (REPL use).
pub fn run_in(src: &str, env: &EnvRef) -> Result<Value, String> {
    let toks = lexer::Lexer::new(src).tokenize()?;
    let stmts = parser::Parser::new(toks).parse_program()?;
    Interp::run(&stmts, env)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str) -> String {
        run_source(src).map(|v| v.to_string()).unwrap_or_else(|e| format!("ERR: {e}"))
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1 + 2 * 3"), "7");
        assert_eq!(eval("(1 + 2) * 3"), "9");
        assert_eq!(eval("10 % 3"), "1");
        assert_eq!(eval("7 / 2"), "3");
        assert_eq!(eval("7.0 / 2"), "3.5");
        assert_eq!(eval("-3 + 5"), "2");
    }

    #[test]
    fn bindings_and_mutability() {
        assert_eq!(eval("let x = 4 x * x"), "16");
        assert_eq!(eval("var x = 1 x = x + 1 x"), "2");
        assert!(eval("let x = 1 x = 2").starts_with("ERR"));
        assert!(eval("y + 1").starts_with("ERR"));
    }

    #[test]
    fn strings() {
        assert_eq!(eval(r#""hello " + "world""#), "hello world");
        assert_eq!(eval(r#"len("codeio")"#), "6");
        assert_eq!(eval(r#""n=" + 42"#), "n=42");
    }

    #[test]
    fn functions_and_recursion() {
        assert_eq!(eval("fn add(a, b) { a + b } add(2, 3)"), "5");
        assert_eq!(
            eval("fn fib(n) { if n < 2 { return n } fib(n-1) + fib(n-2) } fib(15)"),
            "610"
        );
    }

    #[test]
    fn closures() {
        assert_eq!(
            eval("fn make(n) { fn inner(x) { x + n } inner } let add5 = make(5) add5(10)"),
            "15"
        );
    }

    #[test]
    fn control_flow() {
        assert_eq!(eval("if 2 > 1 { \"yes\" } else { \"no\" }"), "yes");
        assert_eq!(eval("var i = 0 var s = 0 while i < 5 { s = s + i i = i + 1 } s"), "10");
        assert_eq!(eval("true && false"), "false");
        assert_eq!(eval("false || true"), "true");
    }

    #[test]
    fn pipes() {
        assert_eq!(eval("fn double(x) { x * 2 } 5 |> double"), "10");
        assert_eq!(eval("fn add(a, b) { a + b } 5 |> add(3)"), "8");
        assert_eq!(eval("fn d(x) { x * 2 } 2 |> d |> d |> d"), "16");
    }

    #[test]
    fn type_annotations_ignored_in_v0() {
        assert_eq!(eval("fn greet(who: Str) -> Str { \"hi \" + who } greet(\"cio\")"), "hi cio");
        assert_eq!(eval("let n: Int = 3 n + 1"), "4");
    }

    #[test]
    fn runtime_errors() {
        assert!(eval("1 / 0").starts_with("ERR"));
        assert!(eval("fn f(a) { a } f(1, 2)").starts_with("ERR"));
        assert!(eval("3(4)").starts_with("ERR"));
    }
}
