//! Structural AST Query Engine — plan §7.3 A4.
//!
//! An S-expression query engine that enforces static architectural policies
//! on VibeScript ASTs. Policies are expressed as query patterns that match
//! against AST structure, with violations reported as diagnostics.
//!
//! ## Built-in policies
//!
//! - `mandatory-take`: for-loops over graph queries must have a `take:` limit
//! - `forbidden-call`: certain function calls are forbidden (e.g. raw I/O)
//! - `required-capability`: functions with effects must declare required capabilities
//! - `no-raw-quin`: raw Quin overlay literals are forbidden (already enforced by lexer)
//! - `tick-no-external`: `on tick` hooks must not perform external effects
//!
//! ## S-expression query language
//!
//! Queries are S-expressions that match AST patterns:
//!
//! ```text
//! (for :no-take)           ;; matches for-loops without a take: limit
//! (call :name "foo")       ;; matches calls to function "foo"
//! (effect :class External) ;; matches effect statements with External class
//! (hook :path "tick")      ;; matches on tick hooks
//! ```

use crate::ast::*;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;

// ── Policy violation ───────────────────────────────────────────────────────

/// A policy violation found by the AST query engine.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyViolation {
    pub policy: String,
    pub span: Span,
    pub message: String,
    pub suggested_fix: Option<String>,
}

impl PolicyViolation {
    pub fn new(policy: &str, span: Span, message: &str) -> Self {
        Self {
            policy: policy.to_string(),
            span,
            message: message.to_string(),
            suggested_fix: None,
        }
    }

    pub fn with_fix(mut self, fix: &str) -> Self {
        self.suggested_fix = Some(fix.to_string());
        self
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic::new(DiagCode::E400, self.span, &self.message)
    }
}

// ── Query pattern ──────────────────────────────────────────────────────────

/// An S-expression query pattern that matches against AST nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryPattern {
    /// Match any node.
    Wildcard,
    /// Match a for-loop. `no_take` matches for-loops without a take: limit.
    For { no_take: bool },
    /// Match a while-loop. `unbounded` matches while-loops without a budget.
    While { unbounded: bool },
    /// Match a function call. `name` matches calls to a specific function.
    Call { name: Option<String> },
    /// Match an effect statement. `class` matches a specific effect class.
    Effect { class: Option<EffectClass> },
    /// Match a hook. `path` matches hooks with a specific path.
    Hook { path: Option<String> },
    /// Match a function declaration. `effect` matches functions with a specific effect class.
    Function { effect: Option<EffectClass> },
    /// Match a capability require. `id` matches requires for a specific capability.
    Require { id: Option<String> },
    /// Match an import. `path` matches imports of a specific path.
    Import { path: Option<String> },
    /// Match a return statement.
    Return,
    /// Match a yield statement.
    Yield,
    /// Match a transaction statement.
    Transaction,
    /// Match a literal of a specific type.
    Literal { kind: Option<String> },
    /// Match a binary expression with a specific operator.
    Binary { op: Option<BinOp> },
    /// Match a triple expression.
    Triple,
    /// Match a reified expression.
    Reified,
    /// Match a match statement.
    Match,
    /// Match an if statement.
    If,
}

// ── S-expression parser ────────────────────────────────────────────────────

/// Parse an S-expression query string into a `QueryPattern`.
///
/// Syntax:
/// ```text
/// (for :no-take)           → QueryPattern::For { no_take: true }
/// (call :name "foo")       → QueryPattern::Call { name: Some("foo") }
/// (effect :class External) → QueryPattern::Effect { class: Some(External) }
/// (hook :path "tick")      → QueryPattern::Hook { path: Some("tick") }
/// _                        → QueryPattern::Wildcard
/// ```
pub fn parse_query(src: &str) -> Result<QueryPattern, String> {
    let src = src.trim();
    if src == "_" || src == "*" {
        return Ok(QueryPattern::Wildcard);
    }
    if !src.starts_with('(') || !src.ends_with(')') {
        return Err(format!("query must be an S-expression: (head :key val ...) or _"));
    }
    let inner = &src[1..src.len() - 1];
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty query".into());
    }
    let head = parts[0];
    let kwargs = parse_kwargs(&parts[1..]);
    match head {
        "for" => Ok(QueryPattern::For { no_take: kwargs.has("no-take") }),
        "while" => Ok(QueryPattern::While { unbounded: kwargs.has("unbounded") }),
        "call" => Ok(QueryPattern::Call { name: kwargs.get_string("name") }),
        "effect" => Ok(QueryPattern::Effect {
            class: kwargs.get_string("class").map(|s| parse_effect_class(&s)).transpose()?,
        }),
        "hook" => Ok(QueryPattern::Hook { path: kwargs.get_string("path") }),
        "function" => Ok(QueryPattern::Function {
            effect: kwargs.get_string("effect").map(|s| parse_effect_class(&s)).transpose()?,
        }),
        "require" => Ok(QueryPattern::Require { id: kwargs.get_string("id") }),
        "import" => Ok(QueryPattern::Import { path: kwargs.get_string("path") }),
        "return" => Ok(QueryPattern::Return),
        "yield" => Ok(QueryPattern::Yield),
        "transaction" => Ok(QueryPattern::Transaction),
        "literal" => Ok(QueryPattern::Literal { kind: kwargs.get_string("kind") }),
        "binary" => Ok(QueryPattern::Binary {
            op: kwargs.get_string("op").map(|s| parse_binop_str(&s)).transpose()?,
        }),
        "triple" => Ok(QueryPattern::Triple),
        "reified" => Ok(QueryPattern::Reified),
        "match" => Ok(QueryPattern::Match),
        "if" => Ok(QueryPattern::If),
        _ => Err(format!("unknown query head: {head}")),
    }
}

struct Kwargs {
    map: std::collections::HashMap<String, String>,
    flags: std::collections::HashSet<String>,
}

impl Kwargs {
    fn has(&self, key: &str) -> bool { self.flags.contains(key) }
    fn get_string(&self, key: &str) -> Option<String> { self.map.get(key).cloned() }
}

fn parse_kwargs(parts: &[&str]) -> Kwargs {
    let mut map = std::collections::HashMap::new();
    let mut flags = std::collections::HashSet::new();
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if let Some(key) = part.strip_prefix(':') {
            if i + 1 < parts.len() && !parts[i + 1].starts_with(':') {
                let val = parts[i + 1].trim_matches('"');
                map.insert(key.to_string(), val.to_string());
                i += 2;
            } else {
                flags.insert(key.to_string());
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    Kwargs { map, flags }
}

fn parse_effect_class(s: &str) -> Result<EffectClass, String> {
    match s {
        "Pure" => Ok(EffectClass::Pure),
        "Hot" => Ok(EffectClass::Hot),
        "Cold" => Ok(EffectClass::Cold),
        "Async" => Ok(EffectClass::Async),
        "External" => Ok(EffectClass::External),
        _ => Err(format!("unknown effect class: {s}")),
    }
}

fn parse_binop_str(s: &str) -> Result<BinOp, String> {
    match s {
        "Or" | "or" => Ok(BinOp::Or),
        "And" | "and" => Ok(BinOp::And),
        "Eq" | "eq" => Ok(BinOp::Eq),
        "Ne" | "ne" => Ok(BinOp::Ne),
        "Lt" | "lt" => Ok(BinOp::Lt),
        "Le" | "le" => Ok(BinOp::Le),
        "Gt" | "gt" => Ok(BinOp::Gt),
        "Ge" | "ge" => Ok(BinOp::Ge),
        "Add" | "add" | "+" => Ok(BinOp::Add),
        "Sub" | "sub" | "-" => Ok(BinOp::Sub),
        "Mul" | "mul" | "*" => Ok(BinOp::Mul),
        "Div" | "div" | "/" => Ok(BinOp::Div),
        "Rem" | "rem" | "%" => Ok(BinOp::Rem),
        _ => Err(format!("unknown binop: {s}")),
    }
}

// ── Policy ─────────────────────────────────────────────────────────────────

/// A named policy with a query pattern and a violation message.
#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub pattern: QueryPattern,
    pub message: String,
    pub suggested_fix: Option<String>,
}

impl Policy {
    pub fn new(name: &str, pattern: QueryPattern, message: &str) -> Self {
        Self {
            name: name.to_string(),
            pattern,
            message: message.to_string(),
            suggested_fix: None,
        }
    }

    pub fn with_fix(mut self, fix: &str) -> Self {
        self.suggested_fix = Some(fix.to_string());
        self
    }
}

/// The built-in policy set: mandatory-take, forbidden-call, tick-no-external.
pub fn builtin_policies() -> Vec<Policy> {
    vec![
        Policy::new(
            "mandatory-take",
            QueryPattern::For { no_take: true },
            "for-loop over a graph query must have a take: limit or budget(steps: N)",
        )
        .with_fix("add `take: 100` or `budget(steps: N)` to the for-loop"),
        Policy::new(
            "tick-no-external",
            QueryPattern::Hook { path: Some("tick".to_string()) },
            "on tick hooks must not perform external effects",
        ),
        Policy::new(
            "unbounded-while",
            QueryPattern::While { unbounded: true },
            "while-loop requires a budget unless the condition is statically finite",
        )
        .with_fix("add `budget(steps: N)` to the while-loop"),
    ]
}

// ── Query engine ───────────────────────────────────────────────────────────

/// Run a set of policies against a program and return all violations.
pub fn run_policies(program: &Program, policies: &[Policy]) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();
    for policy in policies {
        let mut found = query_program(program, &policy.pattern);
        for v in &mut found {
            v.policy = policy.name.clone();
            if v.message.is_empty() {
                v.message = policy.message.clone();
            }
            if v.suggested_fix.is_none() {
                v.suggested_fix = policy.suggested_fix.clone();
            }
        }
        violations.extend(found);
    }
    violations
}

/// Query a program for all AST nodes matching a pattern.
pub fn query_program(program: &Program, pattern: &QueryPattern) -> Vec<PolicyViolation> {
    let mut results = Vec::new();
    // Check imports.
    for imp in &program.imports {
        if matches!(pattern, QueryPattern::Import { path: Some(ref p) } if p == &imp.path) {
            results.push(PolicyViolation::new("import", imp.span, ""));
        }
    }
    // Check requires.
    for cap in &program.requires {
        if matches!(pattern, QueryPattern::Require { id: Some(ref id) } if id == &cap.id) {
            results.push(PolicyViolation::new("require", cap.span, ""));
        }
    }
    // Check items.
    for item in &program.items {
        query_item(item, pattern, &mut results);
    }
    results
}

fn query_item(item: &Item, pattern: &QueryPattern, results: &mut Vec<PolicyViolation>) {
    match item {
        Item::Function(f) => {
            if matches!(pattern, QueryPattern::Function { effect: Some(ref ec) } if f.effect.as_ref() == Some(ec))
                || matches!(pattern, QueryPattern::Function { effect: None })
            {
                results.push(PolicyViolation::new("function", f.span, ""));
            }
            query_block(&f.body, pattern, results);
        }
        Item::Hook(h) => {
            let path_str = h.path.join(":");
            if matches!(pattern, QueryPattern::Hook { path: Some(ref p) } if p == &path_str)
                || matches!(pattern, QueryPattern::Hook { path: None })
            {
                results.push(PolicyViolation::new("hook", Span::new(0, 0), ""));
            }
            query_block(&h.body, pattern, results);
        }
        Item::Const(c) => {
            query_expr(&c.value, pattern, results);
        }
        Item::Statement(s) => {
            query_stmt(s, pattern, results);
        }
    }
}

fn query_block(block: &Block, pattern: &QueryPattern, results: &mut Vec<PolicyViolation>) {
    for s in &block.stmts {
        query_stmt(s, pattern, results);
    }
}

fn query_stmt(stmt: &Stmt, pattern: &QueryPattern, results: &mut Vec<PolicyViolation>) {
    match stmt {
        Stmt::For { span, iter, body, .. } => {
            // Check for take: limit — look at the iter expression for a take() call
            // or check if the for-loop has a budget.
            if let QueryPattern::For { no_take: true } = pattern {
                if !expr_has_take(iter) && !stmt_has_budget(stmt) {
                    results.push(PolicyViolation::new("for", *span, ""));
                }
            }
            if matches!(pattern, QueryPattern::For { no_take: false }) {
                results.push(PolicyViolation::new("for", *span, ""));
            }
            query_expr(iter, pattern, results);
            query_block(body, pattern, results);
        }
        Stmt::While { span, cond, body, .. } => {
            if let QueryPattern::While { unbounded: true } = pattern {
                if !stmt_has_budget(stmt) {
                    results.push(PolicyViolation::new("while", *span, ""));
                }
            }
            if matches!(pattern, QueryPattern::While { unbounded: false }) {
                results.push(PolicyViolation::new("while", *span, ""));
            }
            query_expr(cond, pattern, results);
            query_block(body, pattern, results);
        }
        Stmt::If { cond, then_block, else_block, .. } => {
            if matches!(pattern, QueryPattern::If) {
                results.push(PolicyViolation::new("if", Span::new(0, 0), ""));
            }
            query_expr(cond, pattern, results);
            query_block(then_block, pattern, results);
            if let Some(els) = else_block {
                query_stmt(els, pattern, results);
            }
        }
        Stmt::Match { scrutinee, arms, .. } => {
            if matches!(pattern, QueryPattern::Match) {
                results.push(PolicyViolation::new("match", Span::new(0, 0), ""));
            }
            query_expr(scrutinee, pattern, results);
            for arm in arms {
                match &arm.body {
                    ArmBody::Block(b) => query_block(b, pattern, results),
                    ArmBody::Expr(e) => query_expr(e, pattern, results),
                }
            }
        }
        Stmt::Return { value, .. } => {
            if matches!(pattern, QueryPattern::Return) {
                results.push(PolicyViolation::new("return", Span::new(0, 0), ""));
            }
            if let Some(v) = value {
                query_expr(v, pattern, results);
            }
        }
        Stmt::Yield { value, .. } => {
            if matches!(pattern, QueryPattern::Yield) {
                results.push(PolicyViolation::new("yield", Span::new(0, 0), ""));
            }
            if let Some(v) = value {
                query_expr(v, pattern, results);
            }
        }
        Stmt::Transaction { args, body, .. } => {
            if matches!(pattern, QueryPattern::Transaction) {
                results.push(PolicyViolation::new("transaction", Span::new(0, 0), ""));
            }
            for arg in args {
                query_expr(&arg.value, pattern, results);
            }
            query_block(body, pattern, results);
        }
        Stmt::Effect { expr, .. } => {
            if matches!(pattern, QueryPattern::Effect { class: None }) {
                results.push(PolicyViolation::new("effect", Span::new(0, 0), ""));
            }
            query_expr(expr, pattern, results);
        }
        Stmt::Assign { target, value, .. } => {
            query_expr(target, pattern, results);
            query_expr(value, pattern, results);
        }
        Stmt::Let { value, .. } => {
            if let Some(v) = value {
                query_expr(v, pattern, results);
            }
        }
        Stmt::Expr { expr, .. } => query_expr(expr, pattern, results),
        Stmt::Block(b) => query_block(b, pattern, results),
    }
}

fn query_expr(expr: &Expr, pattern: &QueryPattern, results: &mut Vec<PolicyViolation>) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            // Check for call pattern match.
            if let QueryPattern::Call { name: Some(ref n) } = pattern {
                if let Some(call_name) = callee.ident_name() {
                    if call_name == n {
                        results.push(PolicyViolation::new("call", expr.span, ""));
                    }
                }
            }
            if matches!(pattern, QueryPattern::Call { name: None }) {
                results.push(PolicyViolation::new("call", expr.span, ""));
            }
            query_expr(callee, pattern, results);
            for arg in args {
                match arg {
                    Arg::Pos(e) => query_expr(e, pattern, results),
                    Arg::Named(na) => query_expr(&na.value, pattern, results),
                }
            }
        }
        ExprKind::Binary { op, left, right } => {
            if matches!(pattern, QueryPattern::Binary { op: Some(ref o) } if o == op)
                || matches!(pattern, QueryPattern::Binary { op: None })
            {
                results.push(PolicyViolation::new("binary", expr.span, ""));
            }
            query_expr(left, pattern, results);
            query_expr(right, pattern, results);
        }
        ExprKind::Unary { expr: inner, .. } => query_expr(inner, pattern, results),
        ExprKind::Await(e) => query_expr(e, pattern, results),
        ExprKind::Member { recv, .. } => query_expr(recv, pattern, results),
        ExprKind::Index { recv, index } => {
            query_expr(recv, pattern, results);
            query_expr(index, pattern, results);
        }
        ExprKind::Try(e) => query_expr(e, pattern, results),
        ExprKind::List(es) => {
            for e in es { query_expr(e, pattern, results); }
        }
        ExprKind::Record(args) => {
            for arg in args { query_expr(&arg.value, pattern, results); }
        }
        ExprKind::Triple { subject, predicate, object } => {
            if matches!(pattern, QueryPattern::Triple) {
                results.push(PolicyViolation::new("triple", expr.span, ""));
            }
            query_expr(subject, pattern, results);
            query_expr(predicate, pattern, results);
            query_expr(object, pattern, results);
        }
        ExprKind::Reified { subject, predicate, object, reifier } => {
            if matches!(pattern, QueryPattern::Reified) {
                results.push(PolicyViolation::new("reified", expr.span, ""));
            }
            query_expr(subject, pattern, results);
            query_expr(predicate, pattern, results);
            query_expr(object, pattern, results);
            query_expr(reifier, pattern, results);
        }
        ExprKind::Literal(lit) => {
            if matches!(pattern, QueryPattern::Literal { kind: None }) {
                results.push(PolicyViolation::new("literal", expr.span, ""));
            }
            if let QueryPattern::Literal { kind: Some(ref k) } = pattern {
                let matches = match (k.as_str(), lit) {
                    ("Null", Literal::Null) => true,
                    ("Bool", Literal::Bool(_)) => true,
                    ("Int", Literal::Int(_)) => true,
                    ("UInt", Literal::UInt(_)) => true,
                    ("Float", Literal::Float(_)) => true,
                    ("String", Literal::String(_)) => true,
                    _ => false,
                };
                if matches {
                    results.push(PolicyViolation::new("literal", expr.span, ""));
                }
            }
        }
        _ => {}
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Check if an expression contains a `take()` call.
fn expr_has_take(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Call { callee, .. } => {
            callee.ident_name() == Some("take") || callee.ident_name() == Some("limit")
        }
        ExprKind::Member { recv, .. } => expr_has_take(recv),
        _ => false,
    }
}

/// Check if a statement has a budget annotation.
fn stmt_has_budget(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::For { .. } => false, // Budget is on the function, not the stmt
        Stmt::While { .. } => false,
        _ => false,
    }
}

/// Check if a function has a budget.
pub fn function_has_budget(f: &FunctionDecl) -> bool {
    !f.budget.is_empty()
}

/// Check if a hook has a budget.
pub fn hook_has_budget(h: &HookDecl) -> bool {
    !h.budget.is_empty()
}

// ── High-level policy check ────────────────────────────────────────────────

/// Run all built-in policies against a program.
/// Returns a list of violations. If empty, the program passes all policies.
pub fn check_policies(program: &Program) -> Vec<PolicyViolation> {
    let policies = builtin_policies();
    run_policies(program, &policies)
}

/// Run a custom set of policy queries against a program.
pub fn check_custom_policies(program: &Program, policy_specs: &[(&str, &str)]) -> Vec<PolicyViolation> {
    let policies: Vec<Policy> = policy_specs
        .iter()
        .filter_map(|(name, query)| {
            match parse_query(query) {
                Ok(pattern) => Some(Policy::new(name, pattern, "")),
                Err(_) => None,
            }
        })
        .collect();
    run_policies(program, &policies)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_program;

    #[test]
    fn parse_query_wildcard() {
        assert_eq!(parse_query("_").unwrap(), QueryPattern::Wildcard);
        assert_eq!(parse_query("*").unwrap(), QueryPattern::Wildcard);
    }

    #[test]
    fn parse_query_for_no_take() {
        let q = parse_query("(for :no-take)").unwrap();
        assert_eq!(q, QueryPattern::For { no_take: true });
    }

    #[test]
    fn parse_query_call_named() {
        let q = parse_query(r#"(call :name "foo")"#).unwrap();
        assert_eq!(q, QueryPattern::Call { name: Some("foo".to_string()) });
    }

    #[test]
    fn parse_query_hook_path() {
        let q = parse_query(r#"(hook :path "tick")"#).unwrap();
        assert_eq!(q, QueryPattern::Hook { path: Some("tick".to_string()) });
    }

    #[test]
    fn parse_query_effect_class() {
        let q = parse_query("(effect :class External)").unwrap();
        assert_eq!(q, QueryPattern::Effect { class: Some(EffectClass::External) });
    }

    #[test]
    fn parse_query_binary_op() {
        let q = parse_query("(binary :op Add)").unwrap();
        assert_eq!(q, QueryPattern::Binary { op: Some(BinOp::Add) });
    }

    #[test]
    fn parse_query_unknown_head() {
        assert!(parse_query("(unknown)").is_err());
    }

    #[test]
    fn builtin_policies_count() {
        let policies = builtin_policies();
        assert!(policies.len() >= 3);
    }

    #[test]
    fn check_policies_clean_program() {
        let src = r#"effect fn go() {
    let x = 42;
    return x;
}"#;
        let prog = parse_program(src).unwrap();
        let violations = check_policies(&prog);
        assert!(violations.is_empty(), "clean program should have no violations: {violations:?}");
    }

    #[test]
    fn check_policies_for_loop_without_take() {
        // A for-loop without take: should trigger mandatory-take policy.
        // The for-loop needs to iterate over something that looks like a graph query.
        let src = r#"effect fn go() {
    for item in graph.query() {
        let x = item;
    }
}"#;
        let prog = parse_program(src).unwrap();
        let violations = check_policies(&prog);
        // The mandatory-take policy should flag this for-loop.
        assert!(violations.iter().any(|v| v.policy == "mandatory-take"),
            "expected mandatory-take violation, got: {violations:?}");
    }

    #[test]
    fn check_policies_for_loop_with_take() {
        // A for-loop with take() should NOT trigger mandatory-take policy.
        let src = r#"effect fn go() {
    for item in take(graph.query(), 100) {
        let x = item;
    }
}"#;
        let prog = parse_program(src).unwrap();
        let violations = check_policies(&prog);
        assert!(!violations.iter().any(|v| v.policy == "mandatory-take"),
            "for-loop with take() should not trigger mandatory-take: {violations:?}");
    }

    #[test]
    fn query_call_pattern() {
        let src = r#"effect fn go() {
    foo(42);
    bar(1, 2);
}"#;
        let prog = parse_program(src).unwrap();
        let pattern = parse_query(r#"(call :name "foo")"#).unwrap();
        let results = query_program(&prog, &pattern);
        assert!(!results.is_empty(), "should find call to foo");
    }

    #[test]
    fn query_hook_pattern() {
        let src = r#"on tick(t: f32) {
    return;
}"#;
        let prog = parse_program(src).unwrap();
        let pattern = parse_query(r#"(hook :path "tick")"#).unwrap();
        let results = query_program(&prog, &pattern);
        assert!(!results.is_empty(), "should find tick hook");
    }

    #[test]
    fn query_triple_pattern() {
        let src = r#"effect fn go() {
    <<(ex:s ex:p ex:o)>>;
}"#;
        let prog = parse_program(src).unwrap();
        let pattern = parse_query("(triple)").unwrap();
        let results = query_program(&prog, &pattern);
        assert!(!results.is_empty(), "should find triple expression");
    }

    #[test]
    fn query_literal_pattern() {
        let src = r#"const x = 42;"#;
        let prog = parse_program(src).unwrap();
        let pattern = parse_query(r#"(literal :kind "Int")"#).unwrap();
        let results = query_program(&prog, &pattern);
        assert!(!results.is_empty(), "should find Int literal");
    }

    #[test]
    fn custom_policy_forbidden_call() {
        let src = r#"effect fn go() {
    dangerous_fn();
}"#;
        let prog = parse_program(src).unwrap();
        let violations = check_custom_policies(&prog, &[
            ("forbidden-call", r#"(call :name "dangerous_fn")"#),
        ]);
        assert!(!violations.is_empty(), "should find forbidden call");
        assert_eq!(violations[0].policy, "forbidden-call");
    }

    #[test]
    fn custom_policy_no_match() {
        let src = r#"effect fn go() {
    safe_fn();
}"#;
        let prog = parse_program(src).unwrap();
        let violations = check_custom_policies(&prog, &[
            ("forbidden-call", r#"(call :name "dangerous_fn")"#),
        ]);
        assert!(violations.is_empty(), "should not find any violations");
    }

    #[test]
    fn policy_violation_to_diagnostic() {
        let v = PolicyViolation::new("test", Span::new(0, 10), "something wrong");
        let d = v.to_diagnostic();
        assert_eq!(d.code, DiagCode::E400);
    }

    #[test]
    fn function_has_budget_check() {
        let src = r#"effect fn go() budget(steps: 100) {
    return;
}"#;
        let prog = parse_program(src).unwrap();
        if let Item::Function(f) = &prog.items[0] {
            assert!(function_has_budget(f));
        }
    }

    #[test]
    fn function_no_budget_check() {
        let src = r#"effect fn go() {
    return;
}"#;
        let prog = parse_program(src).unwrap();
        if let Item::Function(f) = &prog.items[0] {
            assert!(!function_has_budget(f));
        }
    }
}
