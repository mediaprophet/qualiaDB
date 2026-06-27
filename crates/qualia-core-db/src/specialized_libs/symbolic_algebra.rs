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

use crate::NQuin;
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
    /// Natural exponential `e^u`.
    Exp(Box<Expr>),
    /// Natural logarithm `ln(u)` (domain `u > 0`).
    Ln(Box<Expr>),
    Sin(Box<Expr>),
    Cos(Box<Expr>),
    /// Tangent `tan(u)` (undefined where `cos(u) = 0`).
    Tan(Box<Expr>),
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
pub fn exp(a: Expr) -> Expr { Expr::Exp(Box::new(a)) }
pub fn ln(a: Expr) -> Expr { Expr::Ln(Box::new(a)) }
pub fn sin(a: Expr) -> Expr { Expr::Sin(Box::new(a)) }
pub fn cos(a: Expr) -> Expr { Expr::Cos(Box::new(a)) }
pub fn tan(a: Expr) -> Expr { Expr::Tan(Box::new(a)) }

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
            Expr::Exp(a) => a.eval(env)?.exp(),
            Expr::Ln(a) => {
                let x = a.eval(env)?;
                if x <= 0.0 { return None; }
                x.ln()
            }
            Expr::Sin(a) => a.eval(env)?.sin(),
            Expr::Cos(a) => a.eval(env)?.cos(),
            Expr::Tan(a) => a.eval(env)?.tan(),
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
        // (e^f)' = e^f · f'
        Expr::Exp(a) => mul(exp((**a).clone()), differentiate(a, wrt)),
        // (ln f)' = f' / f
        Expr::Ln(a) => div(differentiate(a, wrt), (**a).clone()),
        // (sin f)' = cos(f) · f'
        Expr::Sin(a) => mul(cos((**a).clone()), differentiate(a, wrt)),
        // (cos f)' = −sin(f) · f'
        Expr::Cos(a) => mul(neg(sin((**a).clone())), differentiate(a, wrt)),
        // (tan f)' = f' / cos²(f)   (sec²)
        Expr::Tan(a) => div(differentiate(a, wrt), pow(cos((**a).clone()), 2)),
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
        Expr::Exp(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(z) if *z == 0.0 => c(1.0), // e⁰ = 1
                Expr::Ln(inner) => (**inner).clone(),  // e^{ln u} = u
                _ => exp(a),
            }
        }
        Expr::Ln(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(o) if *o == 1.0 => c(0.0), // ln 1 = 0
                Expr::Exp(inner) => (**inner).clone(), // ln(e^u) = u
                _ => ln(a),
            }
        }
        Expr::Sin(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(z) if *z == 0.0 => c(0.0), // sin 0 = 0
                _ => sin(a),
            }
        }
        Expr::Cos(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(z) if *z == 0.0 => c(1.0), // cos 0 = 1
                _ => cos(a),
            }
        }
        Expr::Tan(a) => {
            let a = simplify_once(a);
            match &a {
                Expr::Const(z) if *z == 0.0 => c(0.0), // tan 0 = 0
                _ => tan(a),
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

/// Distribute every product over sums and expand small positive integer powers, so the
/// result contains no `Mul`/`Pow` node with an additive child. Semantically equal to the
/// input (verify by evaluation). Powers above 8 are left unexpanded to bound blow-up.
pub fn expand(expr: &Expr) -> Expr {
    match expr {
        Expr::Const(_) | Expr::Var(_) => expr.clone(),
        Expr::Add(a, b) => add(expand(a), expand(b)),
        Expr::Sub(a, b) => sub(expand(a), expand(b)),
        Expr::Neg(a) => neg(expand(a)),
        Expr::Sqrt(a) => sqrt(expand(a)),
        Expr::Exp(a) => exp(expand(a)),
        Expr::Ln(a) => ln(expand(a)),
        Expr::Sin(a) => sin(expand(a)),
        Expr::Cos(a) => cos(expand(a)),
        Expr::Tan(a) => tan(expand(a)),
        Expr::Div(a, b) => div(expand(a), expand(b)),
        Expr::Mul(a, b) => expand_mul(&expand(a), &expand(b)),
        Expr::Pow(a, e) => expand_pow(&expand(a), *e),
    }
}

fn expand_mul(a: &Expr, b: &Expr) -> Expr {
    match (a, b) {
        (Expr::Add(a1, a2), _) => add(expand_mul(a1, b), expand_mul(a2, b)),
        (Expr::Sub(a1, a2), _) => sub(expand_mul(a1, b), expand_mul(a2, b)),
        (_, Expr::Add(b1, b2)) => add(expand_mul(a, b1), expand_mul(a, b2)),
        (_, Expr::Sub(b1, b2)) => sub(expand_mul(a, b1), expand_mul(a, b2)),
        (Expr::Neg(a1), _) => neg(expand_mul(a1, b)),
        (_, Expr::Neg(b1)) => neg(expand_mul(a, b1)),
        _ => mul(a.clone(), b.clone()),
    }
}

fn expand_pow(base: &Expr, e: i32) -> Expr {
    if e <= 1 || e > 8 {
        return pow(base.clone(), e);
    }
    let mut acc = base.clone();
    for _ in 1..e {
        acc = expand_mul(&acc, base);
    }
    acc
}

/// Factor a real quadratic `a·x² + b·x + c` into `a·(x − r₁)·(x − r₂)` when it has real
/// roots. Returns `None` when the discriminant is negative (no real factorisation) or
/// `a = 0`. Root constants are snapped to integers/halves when numerically close, so the
/// common rational case factors cleanly.
pub fn factor_quadratic(a: f64, b: f64, cc: f64, varname: &str) -> Option<Expr> {
    if a == 0.0 {
        return None;
    }
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let clean = |r: f64| {
        let halves = (r * 2.0).round() / 2.0;
        if (halves - r).abs() < 1e-9 { halves } else { r }
    };
    let r1 = clean((-b + sq) / (2.0 * a));
    let r2 = clean((-b - sq) / (2.0 * a));
    let prod = mul(sub(var(varname), c(r1)), sub(var(varname), c(r2)));
    Some(if (a - 1.0).abs() < 1e-12 { prod } else { mul(c(a), prod) })
}

/// A stable provenance hash of an expression's canonical form, for citing symbolic
/// results back into the graph (Phase 3.8 bridge). Two structurally-equal expressions
/// hash equally; this is `q_hash` over the canonical `Display` string.
pub fn expr_citation_hash(expr: &Expr) -> u64 {
    crate::q_hash(&format!("{expr}"))
}

// ── Expr ↔ NQuin tree encoding (Phase 3.8) ──────────────────────────────────────
// A symbolic expression is serialised into a post-order `Vec<NQuin>`: each node is one
// quin that references its children by their index in the vec (the root is the last
// element). This lets symbolic results be STORED in the graph and CITED, not just hashed.
//
// Per-node quin layout (predicate = node-kind tag via q_hash):
//   const   object = f64 bits
//   var     object = name packed LE (≤ 8 bytes), metadata = byte length
//   add/sub/mul/div  object = left child index, context = right child index
//   pow     object = base child index, metadata = exponent (i32 as u64)
//   neg/sqrt object = child index

fn name_tag(kind: &str) -> u64 { crate::q_hash(kind) }

fn pack_name(name: &str) -> (u64, u64) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(8);
    let mut v = 0u64;
    for (i, &b) in bytes.iter().take(8).enumerate() {
        v |= (b as u64) << (i * 8);
    }
    (v, len as u64)
}

fn unpack_name(v: u64, len: u64) -> String {
    let len = (len as usize).min(8);
    let mut s = String::with_capacity(len);
    for i in 0..len {
        s.push(((v >> (i * 8)) & 0xFF) as u8 as char);
    }
    s
}

fn push_node(out: &mut Vec<NQuin>, predicate: u64, object: u64, context: u64, metadata: u64) -> usize {
    let idx = out.len();
    out.push(NQuin { subject: idx as u64, predicate, object, context, metadata, parity: 0 });
    idx
}

fn encode(e: &Expr, out: &mut Vec<NQuin>) -> usize {
    match e {
        Expr::Const(k) => push_node(out, name_tag("cas:const"), k.to_bits(), 0, 0),
        Expr::Var(name) => {
            let (packed, len) = pack_name(name);
            push_node(out, name_tag("cas:var"), packed, 0, len)
        }
        Expr::Add(a, b) => { let (l, r) = (encode(a, out), encode(b, out)); push_node(out, name_tag("cas:add"), l as u64, r as u64, 0) }
        Expr::Sub(a, b) => { let (l, r) = (encode(a, out), encode(b, out)); push_node(out, name_tag("cas:sub"), l as u64, r as u64, 0) }
        Expr::Mul(a, b) => { let (l, r) = (encode(a, out), encode(b, out)); push_node(out, name_tag("cas:mul"), l as u64, r as u64, 0) }
        Expr::Div(a, b) => { let (l, r) = (encode(a, out), encode(b, out)); push_node(out, name_tag("cas:div"), l as u64, r as u64, 0) }
        Expr::Pow(a, exp) => { let l = encode(a, out); push_node(out, name_tag("cas:pow"), l as u64, 0, (*exp as i64) as u64) }
        Expr::Neg(a) => { let l = encode(a, out); push_node(out, name_tag("cas:neg"), l as u64, 0, 0) }
        Expr::Sqrt(a) => { let l = encode(a, out); push_node(out, name_tag("cas:sqrt"), l as u64, 0, 0) }
        Expr::Exp(a) => { let l = encode(a, out); push_node(out, name_tag("cas:exp"), l as u64, 0, 0) }
        Expr::Ln(a) => { let l = encode(a, out); push_node(out, name_tag("cas:ln"), l as u64, 0, 0) }
        Expr::Sin(a) => { let l = encode(a, out); push_node(out, name_tag("cas:sin"), l as u64, 0, 0) }
        Expr::Cos(a) => { let l = encode(a, out); push_node(out, name_tag("cas:cos"), l as u64, 0, 0) }
        Expr::Tan(a) => { let l = encode(a, out); push_node(out, name_tag("cas:tan"), l as u64, 0, 0) }
    }
}

/// Serialise an expression into a post-order `Vec<NQuin>` (the root is the last element).
pub fn to_quins(expr: &Expr) -> Vec<NQuin> {
    let mut out = Vec::new();
    encode(expr, &mut out);
    out
}

fn decode(quins: &[NQuin], idx: usize) -> Result<Expr, String> {
    let node = quins.get(idx).ok_or_else(|| format!("child index {idx} out of range"))?;
    let p = node.predicate;
    let child = |i: u64| decode(quins, i as usize);
    if p == name_tag("cas:const") {
        Ok(c(f64::from_bits(node.object)))
    } else if p == name_tag("cas:var") {
        Ok(var(&unpack_name(node.object, node.metadata)))
    } else if p == name_tag("cas:add") {
        Ok(add(child(node.object)?, child(node.context)?))
    } else if p == name_tag("cas:sub") {
        Ok(sub(child(node.object)?, child(node.context)?))
    } else if p == name_tag("cas:mul") {
        Ok(mul(child(node.object)?, child(node.context)?))
    } else if p == name_tag("cas:div") {
        Ok(div(child(node.object)?, child(node.context)?))
    } else if p == name_tag("cas:pow") {
        Ok(pow(child(node.object)?, node.metadata as i64 as i32))
    } else if p == name_tag("cas:neg") {
        Ok(neg(child(node.object)?))
    } else if p == name_tag("cas:sqrt") {
        Ok(sqrt(child(node.object)?))
    } else if p == name_tag("cas:exp") {
        Ok(exp(child(node.object)?))
    } else if p == name_tag("cas:ln") {
        Ok(ln(child(node.object)?))
    } else if p == name_tag("cas:sin") {
        Ok(sin(child(node.object)?))
    } else if p == name_tag("cas:cos") {
        Ok(cos(child(node.object)?))
    } else if p == name_tag("cas:tan") {
        Ok(tan(child(node.object)?))
    } else {
        Err(format!("unknown CAS node tag in quin {idx}"))
    }
}

/// Reconstruct an expression from a post-order `Vec<NQuin>` produced by [`to_quins`].
pub fn from_quins(quins: &[NQuin]) -> Result<Expr, String> {
    if quins.is_empty() {
        return Err("empty quin sequence".to_string());
    }
    decode(quins, quins.len() - 1)
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
                // Unary functions: name '(' expr ')'.
                let unary: Option<fn(Expr) -> Expr> = match name.as_str() {
                    "sqrt" => Some(sqrt),
                    "exp" => Some(exp),
                    "ln" => Some(ln),
                    "sin" => Some(sin),
                    "cos" => Some(cos),
                    "tan" => Some(tan),
                    _ => None,
                };
                if let Some(ctor) = unary {
                    self.expect(Tok::LParen)?;
                    let inner = self.parse_expr()?;
                    self.expect(Tok::RParen)?;
                    Ok(ctor(inner))
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
            Expr::Exp(a) => write!(f, "exp({a})"),
            Expr::Ln(a) => write!(f, "ln({a})"),
            Expr::Sin(a) => write!(f, "sin({a})"),
            Expr::Cos(a) => write!(f, "cos({a})"),
            Expr::Tan(a) => write!(f, "tan({a})"),
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
    fn expand_distributes_and_preserves_value() {
        // (x + 1)·(x + 2) expands to a sum with no Mul-over-sum; value matches at samples.
        let e = mul(add(var("x"), c(1.0)), add(var("x"), c(2.0)));
        let ex = expand(&e);
        for &x in &[-3.0, 0.0, 2.5, 7.0] {
            let want = e.eval(&env1("x", x)).unwrap();
            let got = ex.eval(&env1("x", x)).unwrap();
            assert!((want - got).abs() < 1e-9, "expand changed value at x={x}");
        }
        // (x + 1)^3 expands and still evaluates correctly.
        let cube = pow(add(var("x"), c(1.0)), 3);
        let cube_x = expand(&cube);
        assert!((cube_x.eval(&env1("x", 2.0)).unwrap() - 27.0).abs() < 1e-9);
    }

    #[test]
    fn factor_quadratic_inverts_expand() {
        // x² − 5x + 6 factors to (x−2)(x−3); expanding the factors recovers the value.
        let f = factor_quadratic(1.0, -5.0, 6.0, "x").unwrap();
        let original = add(sub(pow(var("x"), 2), mul(c(5.0), var("x"))), c(6.0));
        for &x in &[-1.0, 0.0, 2.0, 3.0, 5.5] {
            let a = f.eval(&env1("x", x)).unwrap();
            let b = original.eval(&env1("x", x)).unwrap();
            assert!((a - b).abs() < 1e-9, "factored != original at x={x}");
        }
        // Negative discriminant → no real factorisation.
        assert!(factor_quadratic(1.0, 0.0, 1.0, "x").is_none());
    }

    #[test]
    fn expr_quin_roundtrip() {
        // Encode an expression to NQuins and decode it back unchanged.
        let e = parse("x^2 + 3*x + 2").unwrap();
        let quins = to_quins(&e);
        assert!(!quins.is_empty());
        let back = from_quins(&quins).unwrap();
        assert_eq!(e, back);

        // Multi-char variable names (≤ 8 bytes) and sqrt/neg survive the round-trip.
        let e2 = sqrt(neg(sub(var("price"), c(4.0))));
        assert_eq!(from_quins(&to_quins(&e2)).unwrap(), e2);
    }

    #[test]
    fn transcendental_diff_eval_parse_and_quins() {
        // d/dx[sin x] = cos x, d/dx[e^x] = e^x, d/dx[ln x] = 1/x, d/dx[tan x] = 1/cos²x —
        // checked against a central finite difference of the original (strong test).
        for f in [sin(var("x")), cos(var("x")), exp(var("x")), ln(var("x")), tan(var("x"))] {
            let df = simplify(&differentiate(&f, "x"));
            for &x in &[0.4, 1.1, 2.3] {
                let h = 1e-6;
                let fd = (f.eval(&env1("x", x + h)).unwrap() - f.eval(&env1("x", x - h)).unwrap())
                    / (2.0 * h);
                let sym = df.eval(&env1("x", x)).unwrap();
                assert!((sym - fd).abs() < 1e-4, "{f}: symbolic {sym} vs fd {fd} at x={x}");
            }
        }
        // Inverse-pair simplifications.
        assert_eq!(simplify(&ln(exp(var("x")))), var("x"));
        assert_eq!(simplify(&exp(ln(var("x")))), var("x"));
        assert_eq!(simplify(&sin(c(0.0))), c(0.0));
        assert_eq!(simplify(&cos(c(0.0))), c(1.0));
        // Parser + Display + quin round-trip on a transcendental expression.
        let e = parse("sin(x) + exp(2*x) - ln(x)").unwrap();
        assert!((e.eval(&env1("x", 1.0)).unwrap()
            - (1.0_f64.sin() + 2.0_f64.exp() - 1.0_f64.ln()))
        .abs()
            < 1e-9);
        assert_eq!(from_quins(&to_quins(&e)).unwrap(), e);
    }

    #[test]
    fn citation_hash_is_structural() {
        let a = add(var("x"), c(1.0));
        let b = add(var("x"), c(1.0));
        assert_eq!(expr_citation_hash(&a), expr_citation_hash(&b));
        assert_ne!(expr_citation_hash(&a), expr_citation_hash(&var("x")));
    }
}
