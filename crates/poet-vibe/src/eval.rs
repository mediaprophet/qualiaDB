//! AST interpreter. No JIT.

use crate::ast::*;
use crate::bind::{dispatch, Host};
use crate::budget::Budget;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::{EnumValue, Value};
use std::collections::{HashMap, HashSet};

pub struct Env {
    pub vars: HashMap<String, Value>,
    /// Set of variable names that were declared with `mut` (T10).
    pub mutables: HashSet<String>,
    /// Import alias → namespace name (e.g. `g` → `graph`).
    /// Populated from `import "vibe:0.1/graph" as g;` declarations.
    pub aliases: HashMap<String, String>,
    /// User-defined enum declarations (T9). Maps enum name → declaration.
    pub enums: HashMap<String, EnumDecl>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
            mutables: HashSet::new(),
            aliases: HashMap::new(),
            enums: HashMap::new(),
        }
    }
}

/// The valid 0.1 namespace import paths. `vibe:0.1/{ns}`.
pub const VIBE_0_1_NAMESPACES: &[&str] = &[
    "math", "rdf", "quin", "graph", "aura", "pulse", "capability", "time",
];

/// Populate `env.aliases` from a program's import declarations.
/// Each `import "vibe:0.1/graph" as g;` maps `g` → `graph`.
/// If no alias is given, the namespace basename is used (e.g. `graph`).
/// Returns an error if the import path is not a valid 0.1 namespace.
pub fn populate_import_aliases(
    env: &mut Env,
    imports: &[ImportDecl],
) -> Result<(), Diagnostic> {
    for imp in imports {
        let ns = imp
            .path
            .strip_prefix("vibe:0.1/")
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E100,
                    imp.span,
                    format!(
                        "import path must be vibe:0.1/<ns>; got '{}'",
                        imp.path
                    ),
                )
            })?;
        if !VIBE_0_1_NAMESPACES.contains(&ns) {
            return Err(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!(
                    "unknown namespace '{}'; valid: {}",
                    ns,
                    VIBE_0_1_NAMESPACES.join(", ")
                ),
            ));
        }
        let alias = imp.alias.as_deref().unwrap_or(ns);
        env.aliases.insert(alias.to_string(), ns.to_string());
    }
    Ok(())
}

pub struct Engine<'a, H: Host> {
    host: &'a mut H,
    budget: Budget,
    depth: u32,
    /// The program being evaluated, if any. When set, `eval_call` resolves
    /// user-defined function names (e.g. `raise_alert(…)`) against the
    /// program's `Item::Function` items before falling through to host
    /// dispatch. This is what lets `on pulse:message(…) { return
    /// raise_alert(…) }` work — the hook body calls a sibling function.
    ///
    /// Stored as a raw pointer because the program's lifetime is not tied to
    /// `'a` — it is passed by reference to `call_function` / `call_hook` and
    /// must outlive the engine evaluation (the caller guarantees this).
    program: Option<*const Program>,
    /// Optional deontic phase leaser (R2). When set, every host dispatch
    /// (`ns.fn`) is checked against the active phase's capability allow-list.
    /// If the capability is forbidden or not allowed in the current phase,
    /// evaluation aborts with a deontic violation diagnostic.
    ///
    /// This is `Option` so the engine remains backward-compatible: existing
    /// callers that don't set a phase leaser behave exactly as before.
    phase_leaser: Option<&'a mut crate::deontic_interrupt::PhaseLeaser>,
}

enum Flow {
    Next(Value),
    Return(Value),
}

impl<'a, H: Host> Engine<'a, H> {
    pub fn new(host: &'a mut H, budget: Budget) -> Self {
        Self {
            host,
            budget,
            depth: 0,
            program: None,
            phase_leaser: None,
        }
    }

    /// Attach a program so `eval_call` can resolve user-defined functions.
    pub fn with_program(host: &'a mut H, budget: Budget, program: &Program) -> Self {
        Self {
            host,
            budget,
            depth: 0,
            program: Some(program as *const Program),
            phase_leaser: None,
        }
    }

    /// Attach a deontic phase leaser (R2). When set, every host dispatch
    /// is checked against the active phase's capability allow-list.
    pub fn with_phase_leaser(
        host: &'a mut H,
        budget: Budget,
        leaser: &'a mut crate::deontic_interrupt::PhaseLeaser,
    ) -> Self {
        Self {
            host,
            budget,
            depth: 0,
            program: None,
            phase_leaser: Some(leaser),
        }
    }

    /// Check whether a capability path is allowed by the active phase lease.
    /// Returns `Ok(())` if allowed (or if no phase leaser is attached),
    /// `Err(diagnostic)` if the capability is forbidden or not allowed.
    fn check_phase_capability(&self, path: &str, span: Span) -> Result<(), Diagnostic> {
        let Some(leaser) = &self.phase_leaser else {
            return Ok(());
        };
        // The capability path is `ns.fn` (e.g., `graph.query`).
        // The phase lease checks the namespace (e.g., `graph`).
        let cap = path.split('.').next().unwrap_or(path);
        if leaser.is_interrupted() {
            return Err(Diagnostic::new(
                DiagCode::E700,
                span,
                format!(
                    "deontic interrupt: agent is halted, capability '{}' denied",
                    cap
                ),
            ));
        }
        if !leaser.is_leased(cap) {
            return Err(Diagnostic::new(
                DiagCode::E700,
                span,
                format!(
                    "deontic phase violation: capability '{}' is not leased in the current phase",
                    cap
                ),
            ));
        }
        Ok(())
    }

    pub fn eval_expr(&mut self, expr: &Expr, env: &mut Env) -> Result<Value, Diagnostic> {
        self.budget.tick(expr.span)?;
        match &expr.kind {
            ExprKind::Literal(l) => Ok(lit(l)),
            ExprKind::Ident(n) => {
                if n == "None" {
                    return Ok(Value::Null);
                }
                env.vars.get(n).cloned().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, expr.span, format!("undefined {n}"))
                })
            }
            ExprKind::QueryVar(n) => Ok(Value::Var(n.clone())),
            ExprKind::Iri(s) => Ok(Value::Iri(s.clone())),
            ExprKind::Prefixed(p, l) => Ok(Value::Prefixed(p.clone(), l.clone())),
            ExprKind::Blank(n) => Ok(Value::Blank(n.clone())),
            ExprKind::Unary { op, expr } => {
                let v = self.eval_expr(expr, env)?;
                match op {
                    UnOp::Not => Ok(Value::Bool(!v.is_truthy())),
                    UnOp::Neg => match v {
                        Value::I64(n) => n
                            .checked_neg()
                            .map(Value::I64)
                            .ok_or_else(|| Diagnostic::new(DiagCode::E600, expr.span, "integer overflow")),
                        Value::U64(n) => {
                            if n == 0 {
                                Ok(Value::U64(0))
                            } else {
                                Err(Diagnostic::new(DiagCode::E600, expr.span, "integer underflow"))
                            }
                        }
                        Value::F64(n) => Ok(Value::F64(-n)),
                        Value::Quantity(q) => Ok(Value::Quantity(crate::value::Quantity {
                            value: -q.value,
                            unit: q.unit,
                        })),
                        _ => v
                            .as_f64()
                            .map(|n| Value::F64(-n))
                            .ok_or_else(|| Diagnostic::new(DiagCode::E600, expr.span, "negation")),
                    },
                    UnOp::Plus => Ok(v),
                }
            }
            ExprKind::Binary { op, left, right } => {
                let l = self.eval_expr(left, env)?;
                if *op == BinOp::And {
                    return Ok(Value::Bool(l.is_truthy() && self.eval_expr(right, env)?.is_truthy()));
                }
                if *op == BinOp::Or {
                    return Ok(Value::Bool(l.is_truthy() || self.eval_expr(right, env)?.is_truthy()));
                }
                let r = self.eval_expr(right, env)?;
                binop(*op, &l, &r, expr.span)
            }
            ExprKind::Member { recv, name } => {
                // T9: Check if this is a unit variant of a user-defined enum
                // BEFORE evaluating the receiver (which would fail as undefined
                // ident). `EnumName.Variant` (no call) → unit enum value.
                if let ExprKind::Ident(enum_name) = &recv.kind {
                    if let Some(enum_decl) = env.enums.get(enum_name) {
                        if enum_decl.variants.iter().any(|v| v.name == *name && v.payload.is_empty()) {
                            return Ok(Value::Enum(EnumValue::unit(enum_name, name)));
                        }
                    }
                }
                let val = self.eval_expr(recv, env)?;
                match val {
                    Value::Record(m) => Ok(m.get(name).cloned().unwrap_or(Value::Null)),
                    other => Err(Diagnostic::new(
                        DiagCode::E600,
                        expr.span,
                        format!("cannot access member `{name}` on {other:?}"),
                    )),
                }
            }
            ExprKind::Call { callee, args } => self.eval_call(callee, args, expr.span, env),
            ExprKind::Try(inner) => match self.eval_expr(inner, env)? {
                Value::Err(e) => Err(Diagnostic::new(
                    DiagCode::E600,
                    expr.span,
                    format!("? on Err: {e:?}"),
                )),
                Value::Ok(v) => Ok(*v),
                other => Ok(other),
            },
            ExprKind::List(xs) => {
                let mut out = Vec::new();
                for x in xs {
                    out.push(self.eval_expr(x, env)?);
                }
                Ok(Value::List(out))
            }
            ExprKind::Record(fs) => {
                let mut m = std::collections::BTreeMap::new();
                for f in fs {
                    m.insert(f.name.clone(), self.eval_expr(&f.value, env)?);
                }
                Ok(Value::Record(m))
            }
            ExprKind::Triple {
                subject,
                predicate,
                object,
            } => Ok(Value::Triple(
                Box::new(self.eval_expr(subject, env)?),
                Box::new(self.eval_expr(predicate, env)?),
                Box::new(self.eval_expr(object, env)?),
            )),
            ExprKind::Reified {
                subject,
                predicate,
                object,
                reifier,
            } => Ok(Value::Reified {
                s: Box::new(self.eval_expr(subject, env)?),
                p: Box::new(self.eval_expr(predicate, env)?),
                o: Box::new(self.eval_expr(object, env)?),
                r: Box::new(self.eval_expr(reifier, env)?),
            }),
            ExprKind::Index { recv, index } => {
                let list = self.eval_expr(recv, env)?;
                let i = self.eval_expr(index, env)?.as_i64().unwrap_or(0) as usize;
                match list {
                    Value::List(xs) => xs.get(i).cloned().ok_or_else(|| {
                        Diagnostic::new(DiagCode::E600, expr.span, "index out of range")
                    }),
                    _ => Err(Diagnostic::new(DiagCode::E600, expr.span, "not a list")),
                }
            }
            ExprKind::Await(e) => self.eval_expr(e, env),
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Arg],
        span: Span,
        env: &mut Env,
    ) -> Result<Value, Diagnostic> {
        if self.depth >= 64 {
            return Err(Diagnostic::new(DiagCode::E400, span, "call depth exceeded"));
        }
        let path = match &callee.kind {
            ExprKind::Member { recv, name } => {
                if let ExprKind::Ident(ns) = &recv.kind {
                    // Resolve import aliases: `g.query` → `graph.query`
                    let resolved_ns = env.aliases.get(ns).map(|s| s.as_str()).unwrap_or(ns);
                    format!("{resolved_ns}.{name}")
                } else {
                    name.clone()
                }
            }
            ExprKind::Ident(n) => n.clone(),
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "call target is not a name",
                ))
            }
        };
        let mut pos = Vec::new();
        let mut named = Vec::new();
        for a in args {
            match a {
                Arg::Pos(e) => pos.push(self.eval_expr(e, env)?),
                Arg::Named(n) => named.push((n.name.clone(), self.eval_expr(&n.value, env)?)),
            }
        }
        if path == "Ok" {
            return Ok(Value::Ok(Box::new(
                pos.into_iter().next().unwrap_or(Value::Null),
            )));
        }
        if path == "Err" {
            return Ok(Value::Err(Box::new(
                pos.into_iter().next().unwrap_or(Value::Null),
            )));
        }
        // T9: User-defined enum variant construction.
        // `EnumName.Variant(args)` → `Value::Enum(EnumValue { ... })`
        if let Some(dot_pos) = path.find('.') {
            let enum_name = &path[..dot_pos];
            let variant_name = &path[dot_pos + 1..];
            if let Some(enum_decl) = env.enums.get(enum_name) {
                if enum_decl.variants.iter().any(|v| v.name == variant_name) {
                    return Ok(Value::Enum(EnumValue::with_payload(
                        enum_name,
                        variant_name,
                        pos,
                    )));
                }
            }
        }
        // Try resolving as a user-defined function in the attached program.
        if let Some(program_ptr) = self.program {
            let program = unsafe { &*program_ptr };
            if let Some(Item::Function(f)) = program.items.iter().find(|i| {
                matches!(i, Item::Function(f) if f.name == path)
            }) {
                if let Some(steps) = budget_steps(&f.budget, env, self) {
                    self.budget.steps_left = self.budget.steps_left.min(steps);
                }
                let mut local = Env::default();
                local.aliases = env.aliases.clone();
                local.enums = env.enums.clone();
                for (p, a) in f.params.iter().zip(pos.into_iter()) {
                    local.vars.insert(p.name.clone(), a);
                }
                self.depth += 1;
                let r = self.finish_block(&f.body, &mut local);
                self.depth -= 1;
                return r;
            }
        }
        self.depth += 1;
        // R2: Deontic phase lease check. If a phase leaser is attached,
        // verify the capability namespace is allowed in the current phase
        // before dispatching to the host. This is the gate that prevents
        // an agent from calling `graph.write` during a read-only phase.
        if let Err(diag) = self.check_phase_capability(&path, span) {
            self.depth -= 1;
            return Err(diag);
        }
        let r = dispatch(self.host, &path, &pos, &named, span);
        self.depth -= 1;
        r
    }

    fn eval_block(&mut self, block: &Block, env: &mut Env) -> Result<Flow, Diagnostic> {
        let mut last = Value::Null;
        for s in &block.stmts {
            match self.eval_stmt(s, env)? {
                Flow::Return(v) => return Ok(Flow::Return(v)),
                Flow::Next(v) => last = v,
            }
        }
        Ok(Flow::Next(last))
    }

    fn finish_block(&mut self, block: &Block, env: &mut Env) -> Result<Value, Diagnostic> {
        match self.eval_block(block, env)? {
            Flow::Return(v) | Flow::Next(v) => Ok(v),
        }
    }

    fn eval_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> Result<Flow, Diagnostic> {
        self.budget.tick(stmt.span())?;
        match stmt {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                env.vars.insert(name.clone(), v);
                if *mutable {
                    env.mutables.insert(name.clone());
                } else {
                    env.mutables.remove(name);
                }
                Ok(Flow::Next(Value::Null))
            }
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let v = self.eval_expr(value, env)?;
                if let Some(n) = target.ident_name() {
                    if !env.mutables.contains(n) {
                        return Err(Diagnostic::new(
                            DiagCode::E701,
                            *span,
                            format!("cannot assign to immutable binding `{n}` (declare with `let mut`)"),
                        ));
                    }
                    env.vars.insert(n.to_string(), v.clone());
                }
                Ok(Flow::Next(v))
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                if self.eval_expr(cond, env)?.is_truthy() {
                    self.eval_block(then_block, env)
                } else if let Some(e) = else_block {
                    self.eval_stmt(e, env)
                } else {
                    Ok(Flow::Next(Value::Null))
                }
            }
            Stmt::For { name, iter, body, .. } => {
                let list = self.eval_expr(iter, env)?;
                let xs = match list {
                    Value::List(xs) => xs,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagCode::E600,
                            stmt.span(),
                            "for-loop needs a list",
                        ))
                    }
                };
                let mut last = Value::Null;
                for x in xs {
                    env.vars.insert(name.clone(), x);
                    match self.eval_block(body, env)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next(v) => last = v,
                    }
                }
                Ok(Flow::Next(last))
            }
            Stmt::While { cond, body, .. } => {
                let mut last = Value::Null;
                while self.eval_expr(cond, env)?.is_truthy() {
                    match self.eval_block(body, env)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next(v) => last = v,
                    }
                }
                Ok(Flow::Next(last))
            }
            Stmt::Return { value, .. } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                Ok(Flow::Return(v))
            }
            Stmt::Transaction { body, span, .. } => {
                self.host.graph_begin(*span)?;
                match self.eval_block(body, env) {
                    Ok(flow) => {
                        let _ = self.host.graph_abort(*span);
                        Ok(flow)
                    }
                    Err(err) => {
                        let _ = self.host.graph_abort(*span);
                        Err(err)
                    }
                }
            }
            Stmt::Block(body) => self.eval_block(body, env),
            Stmt::Effect { expr, .. } | Stmt::Expr { expr, .. } => {
                Ok(Flow::Next(self.eval_expr(expr, env)?))
            }
            Stmt::Yield { value, .. } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                Ok(Flow::Next(v))
            }
            Stmt::Match { scrutinee, arms, .. } => {
                let v = self.eval_expr(scrutinee, env)?;
                for arm in arms {
                    if match_pat(&arm.pattern, &v, env) {
                        return match &arm.body {
                            ArmBody::Block(b) => self.eval_block(b, env),
                            ArmBody::Expr(e) => Ok(Flow::Next(self.eval_expr(e, env)?)),
                        };
                    }
                }
                Ok(Flow::Next(Value::Null))
            }
        }
    }

    pub fn eval_program(&mut self, program: &Program, env: &mut Env) -> Result<Value, Diagnostic> {
        let mut last = Value::Null;
        for item in &program.items {
            match item {
                Item::Const(c) => {
                    let v = self.eval_expr(&c.value, env)?;
                    env.vars.insert(c.name.clone(), v);
                }
                Item::Statement(s) => match self.eval_stmt(s, env)? {
                    Flow::Return(v) => return Ok(v),
                    Flow::Next(v) => last = v,
                },
                Item::Function(_) | Item::Hook(_) => {}
                Item::Enum(e) => {
                    env.enums.insert(e.name.clone(), e.clone());
                }
            }
        }
        Ok(last)
    }

    pub fn call_function(
        &mut self,
        program: &Program,
        name: &str,
        args: Vec<Value>,
        env: &mut Env,
    ) -> Result<Value, Diagnostic> {
        let f = program.items.iter().find_map(|i| match i {
            Item::Function(f) if f.name == name => Some(f),
            _ => None,
        });
        let f = f.ok_or_else(|| {
            Diagnostic::new(DiagCode::E600, Span::point(0), format!("no function {name}"))
        })?;
        if let Some(steps) = budget_steps(&f.budget, env, self) {
            self.budget.steps_left = self.budget.steps_left.min(steps);
        }
        let mut local = Env::default();
        // Inherit import aliases and enum declarations from the calling env.
        local.aliases = env.aliases.clone();
        local.enums = env.enums.clone();
        for (p, a) in f.params.iter().zip(args.into_iter()) {
            local.vars.insert(p.name.clone(), a);
        }
        // Attach the program so nested calls can resolve sibling functions.
        let prev = self.program;
        self.program = Some(program as *const Program);
        let r = self.finish_block(&f.body, &mut local);
        self.program = prev;
        r
    }

    /// Dispatch a hook by event path (e.g. `["pulse", "message"]` or `["tick"]`).
    ///
    /// Finds the first `on <path>(…)` hook in the program whose path matches,
    /// binds the supplied argument values to its parameters, and evaluates its
    /// body. Returns `Ok(Value::Null)` if no matching hook exists (the host
    /// may choose to ignore unhandled events silently).
    pub fn call_hook(
        &mut self,
        program: &Program,
        path: &[String],
        args: Vec<Value>,
        env: &mut Env,
    ) -> Result<Value, Diagnostic> {
        let h = program.items.iter().find_map(|i| match i {
            Item::Hook(h) if h.path == path => Some(h),
            _ => None,
        });
        let h = match h {
            Some(h) => h,
            None => return Ok(Value::Null),
        };
        if let Some(steps) = budget_steps(&h.budget, env, self) {
            self.budget.steps_left = self.budget.steps_left.min(steps);
        }
        let mut local = Env::default();
        local.aliases = env.aliases.clone();
        local.enums = env.enums.clone();
        for (p, a) in h.params.iter().zip(args.into_iter()) {
            local.vars.insert(p.name.clone(), a);
        }
        // Attach the program so the hook body can call sibling functions.
        let prev = self.program;
        self.program = Some(program as *const Program);
        let r = self.finish_block(&h.body, &mut local);
        self.program = prev;
        r
    }
}

fn lit(l: &Literal) -> Value {
    match l {
        Literal::Null => Value::Null,
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Int(n) => Value::I64(*n),
        Literal::UInt(n) => Value::U64(*n),
        Literal::Float(bits) => Value::F64(f64::from_bits(*bits)),
        Literal::String(s) => Value::String(s.clone()),
    }
}

fn binop(op: BinOp, l: &Value, r: &Value, span: Span) -> Result<Value, Diagnostic> {
    if matches!(op, BinOp::Eq | BinOp::Ne) {
        let eq = l == r;
        return Ok(Value::Bool(if op == BinOp::Eq { eq } else { !eq }));
    }

    if let (Value::I64(a), Value::I64(b)) = (l, r) {
        return match op {
            BinOp::Add => a
                .checked_add(*b)
                .map(Value::I64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")),
            BinOp::Sub => a
                .checked_sub(*b)
                .map(Value::I64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")),
            BinOp::Mul => a
                .checked_mul(*b)
                .map(Value::I64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")),
            BinOp::Div => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_div(*b)
                        .map(Value::I64)
                        .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on division"))
                }
            }
            BinOp::Rem => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_rem(*b)
                        .map(Value::I64)
                        .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder"))
                }
            }
            BinOp::Lt => Ok(Value::Bool(a < b)),
            BinOp::Le => Ok(Value::Bool(a <= b)),
            BinOp::Gt => Ok(Value::Bool(a > b)),
            BinOp::Ge => Ok(Value::Bool(a >= b)),
            _ => Ok(Value::I64(*a)),
        };
    }

    if let (Value::U64(a), Value::U64(b)) = (l, r) {
        return match op {
            BinOp::Add => a
                .checked_add(*b)
                .map(Value::U64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")),
            BinOp::Sub => a
                .checked_sub(*b)
                .map(Value::U64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")),
            BinOp::Mul => a
                .checked_mul(*b)
                .map(Value::U64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")),
            BinOp::Div => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_div(*b)
                        .map(Value::U64)
                        .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on division"))
                }
            }
            BinOp::Rem => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_rem(*b)
                        .map(Value::U64)
                        .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder"))
                }
            }
            BinOp::Lt => Ok(Value::Bool(a < b)),
            BinOp::Le => Ok(Value::Bool(a <= b)),
            BinOp::Gt => Ok(Value::Bool(a > b)),
            BinOp::Ge => Ok(Value::Bool(a >= b)),
            _ => Ok(Value::U64(*a)),
        };
    }

    if let (Value::U64(a), Value::I64(b)) = (l, r) {
        if *b >= 0 {
            let b_u64 = *b as u64;
            return match op {
                BinOp::Add => a
                    .checked_add(b_u64)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")),
                BinOp::Sub => a
                    .checked_sub(b_u64)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")),
                BinOp::Mul => a
                    .checked_mul(b_u64)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")),
                BinOp::Div => {
                    if b_u64 == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a.checked_div(b_u64)
                            .map(Value::U64)
                            .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on division"))
                    }
                }
                BinOp::Rem => {
                    if b_u64 == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a.checked_rem(b_u64)
                            .map(Value::U64)
                            .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder"))
                    }
                }
                BinOp::Lt => Ok(Value::Bool(*a < b_u64)),
                BinOp::Le => Ok(Value::Bool(*a <= b_u64)),
                BinOp::Gt => Ok(Value::Bool(*a > b_u64)),
                BinOp::Ge => Ok(Value::Bool(*a >= b_u64)),
                _ => Ok(Value::U64(*a)),
            };
        } else {
            let neg_b = (-*b) as u64;
            return match op {
                BinOp::Add => a
                    .checked_sub(neg_b)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer underflow on addition")),
                BinOp::Sub => a
                    .checked_add(neg_b)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")),
                _ => Err(Diagnostic::new(DiagCode::E600, span, "unsupported op on unsigned and negative integer")),
            };
        }
    }

    if let (Value::I64(a), Value::U64(b)) = (l, r) {
        if *a >= 0 {
            let a_u64 = *a as u64;
            return match op {
                BinOp::Add => a_u64
                    .checked_add(*b)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")),
                BinOp::Sub => a_u64
                    .checked_sub(*b)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")),
                BinOp::Mul => a_u64
                    .checked_mul(*b)
                    .map(Value::U64)
                    .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")),
                BinOp::Div => {
                    if *b == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a_u64
                            .checked_div(*b)
                            .map(Value::U64)
                            .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on division"))
                    }
                }
                BinOp::Rem => {
                    if *b == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a_u64
                            .checked_rem(*b)
                            .map(Value::U64)
                            .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder"))
                    }
                }
                BinOp::Lt => Ok(Value::Bool(a_u64 < *b)),
                BinOp::Le => Ok(Value::Bool(a_u64 <= *b)),
                BinOp::Gt => Ok(Value::Bool(a_u64 > *b)),
                BinOp::Ge => Ok(Value::Bool(a_u64 >= *b)),
                _ => Ok(Value::U64(a_u64)),
            };
        } else {
            return Err(Diagnostic::new(DiagCode::E600, span, "unsupported op on negative integer and unsigned"));
        }
    }

    let a = l.as_f64().ok_or_else(|| {
        Diagnostic::new(DiagCode::E600, span, "numeric operator on non-number")
    })?;
    let b = r.as_f64().ok_or_else(|| {
        Diagnostic::new(DiagCode::E600, span, "numeric operator on non-number")
    })?;
    if matches!(op, BinOp::Div | BinOp::Rem) && b == 0.0 {
        return Err(Diagnostic::new(DiagCode::E600, span, "division by zero"));
    }
    let n = match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        BinOp::Rem => a % b,
        BinOp::Lt => return Ok(Value::Bool(a < b)),
        BinOp::Le => return Ok(Value::Bool(a <= b)),
        BinOp::Gt => return Ok(Value::Bool(a > b)),
        BinOp::Ge => return Ok(Value::Bool(a >= b)),
        _ => a,
    };
    Ok(Value::F64(n))
}

fn match_pat(p: &Pattern, v: &Value, env: &mut Env) -> bool {
    match p {
        Pattern::Wildcard => true,
        Pattern::Ident(n) => {
            env.vars.insert(n.clone(), v.clone());
            true
        }
        Pattern::Literal(l) => lit(l) == *v,
        Pattern::None => matches!(v, Value::Null),
        Pattern::Ok(inner) => match v {
            Value::Ok(x) => match_pat(inner, x, env),
            _ => false,
        },
        Pattern::Err(inner) => match v {
            Value::Err(x) => match_pat(inner, x, env),
            _ => false,
        },
        Pattern::Some(inner) => match_pat(inner, v, env),
        // T9: User-defined enum variant pattern.
        Pattern::Variant {
            enum_name,
            variant_name,
            args,
        } => match v {
            Value::Enum(e) if e.enum_name == *enum_name && e.variant_name == *variant_name => {
                if args.is_empty() {
                    e.payload.is_empty()
                } else if args.len() == e.payload.len() {
                    args.iter()
                        .zip(e.payload.iter())
                        .all(|(pat, val)| match_pat(pat, val, env))
                } else {
                    false
                }
            }
            _ => false,
        },
    }
}

fn budget_steps<H: Host>(args: &[NamedArg], env: &mut Env, eng: &mut Engine<H>) -> Option<u64> {
    for a in args {
        if a.name == "steps" {
            return eng.eval_expr(&a.value, env).ok().and_then(|v| v.as_i64()).map(|n| n as u64);
        }
    }
    None
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::MockHost;
    use crate::deontic_interrupt::{Phase, PhaseLeaser};

    #[test]
    fn r2_no_phase_leaser_backward_compatible() {
        // Without a phase leaser, the engine behaves exactly as before.
        let mut host = MockHost::default();
        let mut env = Env::default();
        let mut engine = Engine::new(&mut host, Budget::default());
        let expr = crate::parse::parse_cell("= math.max(1, 2)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_ok());
    }

    #[test]
    fn r2_phase_leaser_allows_leased_capability() {
        let mut host = MockHost::default();
        let mut leaser = PhaseLeaser::new();
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        let mut env = Env::default();
        let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
        let expr = crate::parse::parse_cell("= math.max(1, 2)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_ok(), "math should be allowed: {:?}", result);
    }

    #[test]
    fn r2_phase_leaser_blocks_unleased_capability() {
        let mut host = MockHost::default();
        let mut leaser = PhaseLeaser::new();
        // Only allow "math" in this phase — "graph" is not leased.
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        let mut env = Env::default();
        let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
        // graph.query is not allowed in this phase.
        let expr = crate::parse::parse_cell("= graph.query(?s, ?p, ?o, take: 10)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_err(), "graph should be blocked");
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagCode::E700);
        assert!(err.message.contains("graph"));
    }

    #[test]
    fn r2_phase_leaser_blocks_after_interrupt() {
        let mut host = MockHost::default();
        let mut leaser = PhaseLeaser::new();
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        // Trigger an interrupt — all capabilities should be revoked.
        let interrupt = crate::deontic_interrupt::DeonticInterrupt::prohibition_breach(
            "graph", "execute", None,
        );
        leaser.trigger_interrupt(interrupt);

        let mut env = Env::default();
        let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
        let expr = crate::parse::parse_cell("= math.max(1, 2)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_err(), "math should be blocked after interrupt");
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagCode::E700);
        assert!(err.message.contains("halted"));
    }

    // ── T9: User-defined enum / match ADT tests ───────────────────────────

    fn eval_program_src(src: &str) -> Result<Value, Diagnostic> {
        let program = crate::parse::parse_program(src)?;
        crate::check::check_program(&program)?;
        let mut host = MockHost::default();
        let mut env = Env::default();
        // Register enums and consts first.
        let mut engine = Engine::with_program(&mut host, Budget::default(), &program);
        engine.eval_program(&program, &mut env)?;
        // Then call main().
        crate::eval_function(&program, "main", Vec::new(), &mut host, &mut env)
    }

    #[test]
    fn t9_enum_unit_variant_construction() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                return Shape.Point;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        let e = result.as_enum().unwrap();
        assert_eq!(e.enum_name, "Shape");
        assert_eq!(e.variant_name, "Point");
        assert!(e.payload.is_empty());
    }

    #[test]
    fn t9_enum_payload_variant_construction() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                return Shape.Circle(3.14);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        let e = result.as_enum().unwrap();
        assert_eq!(e.enum_name, "Shape");
        assert_eq!(e.variant_name, "Circle");
        assert_eq!(e.payload.len(), 1);
        assert_eq!(e.payload[0], Value::F64(3.14));
    }

    #[test]
    fn t9_enum_match_unit_variant() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                let s = Shape.Point;
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => 1,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(0));
    }

    #[test]
    fn t9_enum_match_payload_variant_binds_inner() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                let s = Shape.Circle(5.0);
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => r,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(5.0));
    }

    #[test]
    fn t9_enum_match_multi_payload_variant() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
                Rect(f64, f64),
            }
            fn main() {
                let s = Shape.Rect(3.0, 4.0);
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => r,
                    Shape.Rect(w, h) => w * h,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(12.0));
    }

    #[test]
    fn t9_enum_match_wildcard_fallback() {
        let src = r#"
            enum Color {
                Red,
                Green,
                Blue,
            }
            fn main() {
                let c = Color.Green;
                match c {
                    Color.Red => 0xFF0000,
                    _ => 0x00FF00,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(0x00FF00));
    }

    #[test]
    fn t9_enum_match_wrong_variant_does_not_match() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                let s = Shape.Circle(1.0);
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => 1,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(1));
    }

    #[test]
    fn mut_let_allows_reassignment() {
        let src = r#"
            fn main() {
                let mut x = 1;
                x = 2;
                return x;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(2));
    }

    #[test]
    fn let_rejects_reassignment() {
        let src = r#"
            fn main() {
                let x = 1;
                x = 2;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E701);
    }

    #[test]
    fn mut_check_catches_reassignment_in_block() {
        let src = r#"
            fn main() {
                let mut x = 1;
                {
                    let y = 10;
                    y = 20;
                }
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E701);
    }

    #[test]
    fn i64_add_overflow() {
        let src = r#"
            fn main() {
                let x = 9223372036854775807 + 1;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn i64_sub_overflow() {
        let src = r#"
            fn main() {
                let x = -9223372036854775808 - 1;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn i64_mul_overflow() {
        let src = r#"
            fn main() {
                let x = 9223372036854775807 * 2;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn u64_add_overflow() {
        let src = r#"
            fn main() {
                let x = 18446744073709551615 + 1;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn i64_normal_add_still_works() {
        let src = r#"
            fn main() {
                let x = 1 + 2;
                return x;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn f64_overflow_not_checked() {
        let src = r#"
            fn main() {
                let x = 1e308 * 1e308;
                return x;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(f64::INFINITY));
    }

    #[test]
    fn math_abs_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.abs(-5);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(5));
    }

    #[test]
    fn math_abs_f64_returns_f64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.abs(-5.0);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(5.0));
    }

    #[test]
    fn math_min_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.min(3, 7);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn math_max_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.max(3, 7);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(7));
    }

    #[test]
    fn math_floor_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.floor(5);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(5));
    }

    #[test]
    fn math_sqrt_always_f64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.sqrt(9);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(3.0));
    }
}
