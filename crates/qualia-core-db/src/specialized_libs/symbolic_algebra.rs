//! Symbolic algebra — a small computer-algebra system (CAS).
//!
//! Expression trees over rationals/reals + variables, with `simplify`,
//! `differentiate`, numeric `eval`, and symbolic equation solving. This is the
//! ALGEBRA_MANIFOLD_PLAN.md Phase 3 module and is DELIBERATELY distinct from
//! `solvers/symbolic_logic` (which is SAT / defeasible LOGIC, not computer algebra).
//!
//! The CAS is an authoring / tooling path and may allocate (`Box`, `String`); it must
//! NOT be used on an NQuin/SlgArena hot path. Results can be bridged back into the
//! graph via [`expr_citation_hash`] for provenance.

use std::collections::HashMap;

/// A symbolic expression over real constants and named variables.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Const(f64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    /// Integer power `base^exp`.
    Pow(Box<Expr>, i32),
    Neg(Box<Expr>),
    /// Principal square root.
    Sqrt(Box<Expr>),
}

// ── ergonomic constructors ──────────────────────────────────────────────────────
pub fn c(v: f64) -> Expr { Expr::Const(v) }
pub fn var(name: &str) -> Expr { Expr::Var(name.to_string()) }
pub fn add(a: Expr, b: Expr) -> Expr { Expr::Add(Box::new(a), Box::new(b)) }
pub fn sub(a: Expr, b: Expr) -> Expr { Expr::Sub(Box::new(a), Box::new(b)) }
pub fn mul(a: Expr, b: Expr) -> Expr { Expr::Mul(Box::new(a), Box::new(b)) }
pub fn div(a: Expr, b: Expr) -> Expr { Expr::Div(Box::new(a), Box::new(b)) }
pub fn pow(a: Expr, e: i32) -> Expr { Expr::Pow(Box::new(a), e) }
pub fn neg(a: Expr) -> Expr { Expr::Neg(Box::new(a)) }
pub fn sqrt(a: Expr) -> Expr { Expr::Sqrt(Box::new(a)) }

impl Expr {
    /// Numerically evaluate, given variable bindings. Returns `None` if a variable is
    /// unbound or a non-finite value is produced (e.g. division by zero, √negative).
    pub fn eval(&self, env: &HashMap<String, f64>) -> Option<f64> {
        let v = match self {
            Expr::Const(k) => *k,
            Expr::Var(name) => *env.get(name)?,
            Expr::Add(a, b) => a.eval(env)? + b.eval(env)?,
            Expr::Sub(a, b) => a.eval(env)? - b.eval(env)?,
            Expr::Mul(a, b) => a.eval(env)? * b.eval(env)?,
            Expr::Div(a, b) => {
                let d = b.eval(env)?;
                if d == 0.0 { return None; }
                a.eval(env)? / d
            }
            Expr::Pow(a, e) => a.eval(env)?.powi(*e),
            Expr::Neg(a) => -a.eval(env)?,
            Expr::Sqrt(a) => {
                let x = a.eval(env)?;
                if x < 0.0 { return None; }
                x.sqrt()
            }
        };
        if v.is_finite() { Some(v) } else { None }
    }
}

/// Symbolic derivative of `expr` with respect to variable `wrt`. The result is NOT
/// auto-simplified — call [`simplify`] on it for a compact form.
pub fn differentiate(expr: &Expr, wrt: &str) -> Expr {
    match expr {
        Expr::Const(_) => c(0.0),
        Expr::Var(name) => if name == wrt { c(1.0) } else { c(0.0) },
        Expr::Add(a, b) => add(differentiate(a, wrt), differentiate(b, wrt)),
        Expr::Sub(a, b) => sub(differentiate(a, wrt), differentiate(b, wrt)),
        // (f·g)' = f'·g + f·g'
        Expr::Mul(a, b) => add(
            mul(differentiate(a, wrt), (**b).clone()),
            mul((**a).clone(), differentiate(b, wrt)),
        ),
        // (f/g)' = (f'·g − f·g') / g²
        Expr::Div(a, b) => div(
            sub(
                mul(differentiate(a, wrt), (**b).clone()),
                mul((**a).clone(), differentiate(b, wrt)),
            ),
            pow((**b).clone(), 2),
        ),
        // (fⁿ)' = n·fⁿ⁻¹·f'
        Expr::Pow(a, e) => mul(
            mul(c(*e as f64), pow((**a).clone(), e - 1)),
            differentiate(a, wrt),
        ),
        Expr::Neg(a) => neg(differentiate(a, wrt)),
        // (√f)' = f' / (2·√f)
        Expr::Sqrt(a) => div(
            differentiate(a, wrt),
            mul(c(2.0), sqrt((**a).clone())),
        ),
    }
}

/// Simplify an expression: constant folding, identity elimination (`x+0`, `x·1`,
/// `x·0`, `x⁰`, `x¹`, `−(−x)`, `x/1`, `x−x`, `x/x`) and collection of an identical
/// `x+x → 2·x`. Applied to a fixpoint (bounded).
pub fn simplify(expr: &Expr) -> Expr {
    let mut cur = expr.clone();
    for _ in 0..16 {
        let next = simplify_once(&cur);
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

fn simplify_once(expr: &Expr) -> Expr {
    match expr {
        Expr::Const(_) | Expr::Var(_) => expr.clone(),
        Expr::Add(a, b) => {
            let (a, b) = (simplify_once(a), simplify_once(b));
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => c(x + y),
                (Expr::Const(z), _) if *z == 0.0 => b,
                (_, Expr::Const(z)) if *z == 0.0 => a,
                _ if a == b => mul(c(2.0), a), // x + x → 2·x
                _ => add(a, b),
            }
        }
        Expr::Sub(a, b) => {
            let (a, b) = (simplify_once(a), simplify_once(b));
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => c(x - y),
                (_, Expr::Const(z)) if *z == 0.0 => a,
                _ if a == b => c(0.0), // x − x → 0
                _ => sub(a, b),
            }
        }
        Expr::Mul(a, b) => {
            let (a, b) = (simplify_once(a), simplify_once(b));
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => c(x * y),
                (Expr::Const(z), _) | (_, Expr::Const(z)) if *z == 0.0 => c(0.0),
                (Expr::Const(o), _) if *o == 1.0 => b,
                (_, Expr::Const(o)) if *o == 1.0 => a,
                _ => mul(a, b),
            }
        }
        Expr::Div(a, b) => {
            let (a, b) = (simplify_once(a), simplify_once(b));
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if *y != 0.0 => c(x / y),
                (Expr::Const(z), _) if *z == 0.0 => c(0.0),
                (_, Expr::Const(o)) if *o == 1.0 => a,
                _ if a == b => c(1.0), // x / x → 1 (assumes x ≠ 0)
                _ => div(a, b),
            }
        }
        Expr::Pow(a, e) => {
            let a = simplify_once(a);
            match (&a, e) {
                (_, 0) => c(1.0),
                (_, 1) => a,
                (Expr::Const(x), _) => c(x.powi(*e)),
                _ => pow(a, *e),
            }
        }
        Expr::Neg(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(x) => c(-x),
                Expr::Neg(inner) => (**inner).clone(), // −(−x) → x
                _ => neg(a),
            }
        }
        Expr::Sqrt(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(x) if *x >= 0.0 => {
                    let r = x.sqrt();
                    // fold only when exact (avoids hiding irrationality)
                    if r.fract() == 0.0 { c(r) } else { sqrt(a) }
                }
                _ => sqrt(a),
            }
        }
    }
}

/// Symbolic roots of `a·x² + b·x + c = 0` with real coefficients, as exact `Expr`s
/// `(-b ± √(b²−4ac)) / (2a)`. Returns the two root expressions (simplified). For
/// `a = 0` returns the single linear root `-c/b`.
pub fn solve_quadratic_symbolic(a: f64, b: f64, cc: f64) -> Vec<Expr> {
    if a == 0.0 {
        if b == 0.0 {
            return Vec::new();
        }
        return vec![simplify(&div(neg(c(cc)), c(b)))];
    }
    let disc = sub(pow(c(b), 2), mul(c(4.0), mul(c(a), c(cc)))); // b² − 4ac
    let root_plus = div(add(neg(c(b)), sqrt(disc.clone())), mul(c(2.0), c(a)));
    let root_minus = div(sub(neg(c(b)), sqrt(disc)), mul(c(2.0), c(a)));
    vec![simplify(&root_plus), simplify(&root_minus)]
}

/// A stable provenance hash of an expression's canonical form, for citing symbolic
/// results back into the graph (Phase 3.8 bridge). Two structurally-equal expressions
/// hash equally; this is `q_hash` over the canonical `Display` string.
pub fn expr_citation_hash(expr: &Expr) -> u64 {
    crate::q_hash(&format!("{expr}"))
}

// ── parser: text → Expr (recursive descent) ─────────────────────────────────────
// Grammar:  expr = term (('+'|'-') term)*
//           term = factor (('*'|'/') factor)*
//           factor = unary ('^' integer)?
//           unary = '-' unary | base
//           base = number | ident | 'sqrt' '(' expr ')' | '(' expr ')'

/// Parse a textual expression like `"x^3 - 2*x^2 + 5"` or `"sqrt(b^2 - 4*a*c)"` into an
/// [`Expr`]. Supports `+ - * / ^`, parentheses, `sqrt(...)`, numbers and identifiers.
pub fn parse(input: &str) -> Result<Expr, String> {
    let tokens = tokenize(input)?;
    let mut p = Parser { tokens, pos: 0 };
    let e = p.parse_expr()?;
    if p.pos != p.tokens.len() {
        return Err(format!("unexpected trailing tokens at {}", p.pos));
    }
    Ok(e)
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '+' => { out.push(Tok::Plus); i += 1; }
            '-' => { out.push(Tok::Minus); i += 1; }
            '*' => { out.push(Tok::Star); i += 1; }
            '/' => { out.push(Tok::Slash); i += 1; }
            '^' => { out.push(Tok::Caret); i += 1; }
            '(' => { out.push(Tok::LParen); i += 1; }
            ')' => { out.push(Tok::RParen); i += 1; }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num: String = chars[start..i].iter().collect();
                out.push(Tok::Num(num.parse().map_err(|_| format!("bad number '{num}'"))?));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            other => return Err(format!("unexpected character '{other}'")),
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> { self.tokens.get(self.pos) }
    fn next(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() { self.pos += 1; }
        t
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        while let Some(op) = self.peek() {
            match op {
                Tok::Plus => { self.next(); left = add(left, self.parse_term()?); }
                Tok::Minus => { self.next(); left = sub(left, self.parse_term()?); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;
        while let Some(op) = self.peek() {
            match op {
                Tok::Star => { self.next(); left = mul(left, self.parse_factor()?); }
                Tok::Slash => { self.next(); left = div(left, self.parse_factor()?); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        if let Some(Tok::Caret) = self.peek() {
            self.next();
            // exponent must be an integer literal (optionally negated)
            let neg_exp = matches!(self.peek(), Some(Tok::Minus));
            if neg_exp { self.next(); }
            match self.next() {
                Some(Tok::Num(n)) if n.fract() == 0.0 => {
                    let e = n as i32 * if neg_exp { -1 } else { 1 };
                    Ok(pow(base, e))
                }
                _ => Err("'^' requires an integer exponent".to_string()),
            }
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if let Some(Tok::Minus) = self.peek() {
            self.next();
            return Ok(neg(self.parse_unary()?));
        }
        self.parse_base()
    }

    fn parse_base(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(c(n)),
            Some(Tok::Ident(name)) => {
                if name == "sqrt" {
                    self.expect(Tok::LParen)?;
                    let inner = self.parse_expr()?;
                    self.expect(Tok::RParen)?;
                    Ok(sqrt(inner))
                } else {
                    Ok(var(&name))
                }
            }
            Some(Tok::LParen) => {
                let inner = self.parse_expr()?;
                self.expect(Tok::RParen)?;
                Ok(inner)
            }
            other => Err(format!("unexpected token: {other:?}")),
        }
    }

    fn expect(&mut self, t: Tok) -> Result<(), String> {
        if self.next().as_ref() == Some(&t) {
            Ok(())
        } else {
            Err(format!("expected {t:?}"))
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Const(k) => write!(f, "{k}"),
            Expr::Var(name) => write!(f, "{name}"),
            Expr::Add(a, b) => write!(f, "({a} + {b})"),
            Expr::Sub(a, b) => write!(f, "({a} - {b})"),
            Expr::Mul(a, b) => write!(f, "({a} * {b})"),
            Expr::Div(a, b) => write!(f, "({a} / {b})"),
            Expr::Pow(a, e) => write!(f, "({a}^{e})"),
            Expr::Neg(a) => write!(f, "(-{a})"),
            Expr::Sqrt(a) => write!(f, "sqrt({a})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env1(name: &str, v: f64) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert(name.to_string(), v);
        m
    }

    #[test]
    fn differentiate_matches_finite_difference() {
        // f(x) = x³ − 2x² + 5  → f'(x) = 3x² − 4x. Check at several points against a
        // central finite difference of the ORIGINAL expression (strong correctness test).
        let f = add(sub(pow(var("x"), 3), mul(c(2.0), pow(var("x"), 2))), c(5.0));
        let df = simplify(&differentiate(&f, "x"));
        for &x in &[-2.0, -0.5, 1.0, 3.7] {
            let h = 1e-6;
            let fd = (f.eval(&env1("x", x + h)).unwrap() - f.eval(&env1("x", x - h)).unwrap())
                / (2.0 * h);
            let sym = df.eval(&env1("x", x)).unwrap();
            assert!((sym - fd).abs() < 1e-4, "x={x}: symbolic {sym} vs fd {fd}");
        }
    }

    #[test]
    fn simplify_identities() {
        assert_eq!(simplify(&add(var("x"), c(0.0))), var("x"));
        assert_eq!(simplify(&mul(var("x"), c(1.0))), var("x"));
        assert_eq!(simplify(&mul(var("x"), c(0.0))), c(0.0));
        assert_eq!(simplify(&pow(var("x"), 0)), c(1.0));
        assert_eq!(simplify(&neg(neg(var("x")))), var("x"));
        assert_eq!(simplify(&sub(var("x"), var("x"))), c(0.0));
        assert_eq!(simplify(&add(var("x"), var("x"))), mul(c(2.0), var("x")));
        assert_eq!(simplify(&add(c(2.0), c(3.0))), c(5.0));
    }

    #[test]
    fn symbolic_quadratic_agrees_with_numeric() {
        // x² − 5x + 6 → roots {3, 2}; evaluate the symbolic root expressions.
        let roots = solve_quadratic_symbolic(1.0, -5.0, 6.0);
        assert_eq!(roots.len(), 2);
        let empty = HashMap::new();
        let mut vals: Vec<f64> = roots.iter().map(|r| r.eval(&empty).unwrap()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((vals[0] - 2.0).abs() < 1e-12 && (vals[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn parse_and_differentiate_roundtrip() {
        // Parse a textual polynomial, differentiate symbolically, and check the
        // derivative numerically (d/dx of x^3 - 2x^2 + 5 is 3x^2 - 4x → at x=2: 4).
        let f = parse("x^3 - 2*x^2 + 5").unwrap();
        let df = simplify(&differentiate(&f, "x"));
        assert!((df.eval(&env1("x", 2.0)).unwrap() - 4.0).abs() < 1e-9);
        // precedence: 2 + 3 * 4 = 14, not 20
        assert_eq!(parse("2 + 3 * 4").unwrap().eval(&HashMap::new()).unwrap(), 14.0);
        // sqrt + parens
        let g = parse("sqrt((a + 3))").unwrap();
        assert!((g.eval(&env1("a", 1.0)).unwrap() - 2.0).abs() < 1e-12);
        // bad input errors, not panics
        assert!(parse("2 +* 3").is_err());
    }

    #[test]
    fn citation_hash_is_structural() {
        let a = add(var("x"), c(1.0));
        let b = add(var("x"), c(1.0));
        assert_eq!(expr_citation_hash(&a), expr_citation_hash(&b));
        assert_ne!(expr_citation_hash(&a), expr_citation_hash(&var("x")));
    }
}
