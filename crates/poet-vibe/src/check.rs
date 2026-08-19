//! Type and effect checker.

use crate::ast::*;
use crate::effects::{binding_effect, capability_for, Effect};
use crate::error::{DiagCode, Diagnostic};
use crate::types::Type;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct CheckResult {
    pub effect: Effect,
}

pub fn check_program(program: &Program) -> Result<CheckResult, Diagnostic> {
    // Validate import paths before checking items.
    let mut seen_aliases = HashSet::new();
    for imp in &program.imports {
        let path = imp.path.as_str();
        if !path.starts_with("vibe:0.1/") {
            return Err(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!("import path must be vibe:0.1/<ns>; got '{path}'"),
            ));
        }
        let ns = &path["vibe:0.1/".len()..];
        const VALID: &[&str] = &[
            "math", "rdf", "quin", "graph", "aura", "pulse", "capability", "time",
            "conservation", "causal",
        ];
        if !VALID.contains(&ns) {
            return Err(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!(
                    "unknown namespace '{ns}'; valid: {}",
                    VALID.join(", ")
                ),
            ));
        }
        let alias = imp.alias.as_deref().unwrap_or(ns);
        if !seen_aliases.insert(alias) {
            return Err(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!("duplicate import alias '{alias}'"),
            ));
        }
    }
    let granted: Vec<&str> = program.requires.iter().map(|c| c.id.as_str()).collect();
    let mut max_effect = Effect::Pure;
    let mut env = HashMap::new();
    let mut mutables = HashSet::new();
    for item in &program.items {
        match item {
            Item::Function(f) => {
                let e = check_function(f, &granted)?;
                max_effect = max_effect.join(e);
            }
            Item::Hook(h) => {
                let e = check_hook(h, &granted)?;
                max_effect = max_effect.join(e);
            }
            Item::Const(c) => {
                walk_expr(&c.value, &mut env, &granted, Effect::Pure, false, false)?;
            }
            Item::Statement(s) => {
                walk_stmt(s, &mut env, &mut mutables, &granted, Effect::External, false, false)?;
            }
            Item::Enum(_) => {
                // Enum declarations are pure type declarations — no effect.
            }
            Item::Field(_) => {
                // Field declarations are pure type declarations — no effect.
            }
            Item::Material(_) => {
                // Material declarations are pure data declarations — no effect.
            }
            Item::Law(l) => {
                // Law declarations have a condition and consequence.
                // The condition is a Pure predicate; the consequence is an
                // External effect (it transforms state).
                walk_expr(&l.condition, &mut env, &granted, Effect::Pure, false, false)?;
                walk_expr(&l.consequence, &mut env, &granted, Effect::External, false, false)?;
                max_effect = max_effect.join(Effect::External);
            }
        }
    }
    Ok(CheckResult { effect: max_effect })
}

pub fn check_cell(expr: &Expr) -> Result<CheckResult, Diagnostic> {
    let mut env = HashMap::new();
    let effect = walk_expr(expr, &mut env, &[], Effect::Pure, true, false)?;
    if effect > Effect::Pure {
        return Err(Diagnostic::new(
            DiagCode::E200,
            expr.span,
            "Pure cell cannot perform External effects",
        ));
    }
    Ok(CheckResult { effect })
}

fn check_function(
    f: &FunctionDecl,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    let declared = f.effect.map(Effect::from_class).unwrap_or(Effect::Pure);
    let mut env = HashMap::new();
    let mut mutables = HashSet::new();
    for p in &f.params {
        env.insert(p.name.clone(), Type::from_ast(&p.ty));
    }
    let has_budget = !f.budget.is_empty();
    let body = walk_block(&f.body, &mut env, &mut mutables, granted, declared, false, has_budget)?;
    if body > declared && f.effect.is_some() {
        return Err(Diagnostic::new(
            DiagCode::E200,
            f.span,
            "function body exceeds declared effect class",
        ));
    }
    Ok(body.join(declared))
}

fn check_hook(h: &HookDecl, granted: &[&str]) -> Result<Effect, Diagnostic> {
    let is_tick = h.path == ["tick"];
    let declared = if is_tick { Effect::Hot } else { Effect::External };
    if is_tick {
        // N7: on tick must not query the graph
    }
    let mut env = HashMap::new();
    let mut mutables = HashSet::new();
    for p in &h.params {
        env.insert(p.name.clone(), Type::from_ast(&p.ty));
    }
    let has_budget = !h.budget.is_empty();
    walk_block(&h.body, &mut env, &mut mutables, granted, declared, is_tick, has_budget)
}

fn walk_block(
    block: &Block,
    env: &mut HashMap<String, Type>,
    mutables: &mut HashSet<String>,
    granted: &[&str],
    ambient: Effect,
    tick: bool,
    budgeted: bool,
) -> Result<Effect, Diagnostic> {
    let mut scoped_env = env.clone();
    let mut scoped_mutables = mutables.clone();
    let mut e = Effect::Pure;
    for s in &block.stmts {
        e = e.join(walk_stmt(s, &mut scoped_env, &mut scoped_mutables, granted, ambient, tick, budgeted)?);
    }
    Ok(e)
}

fn walk_stmt(
    stmt: &Stmt,
    env: &mut HashMap<String, Type>,
    mutables: &mut HashSet<String>,
    granted: &[&str],
    ambient: Effect,
    tick: bool,
    budgeted: bool,
) -> Result<Effect, Diagnostic> {
    match stmt {
        Stmt::Let { mutable, name, value, ty, .. } => {
            let t = if value.as_ref().is_some_and(|v| expr_is_bounded_source(v, env)) {
                Type::List(Box::new(Type::Unknown))
            } else {
                ty.as_ref().map(Type::from_ast).unwrap_or(Type::Unknown)
            };
            env.insert(name.clone(), t);
            if *mutable {
                mutables.insert(name.clone());
            } else {
                mutables.remove(name);
            }
            if let Some(v) = value {
                return walk_expr(v, env, granted, ambient, tick, budgeted);
            }
            Ok(Effect::Pure)
        }
        Stmt::Assign { target, value, span } => {
            if let Some(n) = target.ident_name() {
                if !mutables.contains(n) {
                    return Err(Diagnostic::new(
                        DiagCode::E701,
                        *span,
                        format!("cannot assign to immutable binding `{n}` (declare with `let mut`)"),
                    ));
                }
            }
            let a = walk_expr(target, env, granted, ambient, tick, budgeted)?;
            let b = walk_expr(value, env, granted, ambient, tick, budgeted)?;
            Ok(a.join(b))
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            let mut e = walk_expr(cond, env, granted, ambient, tick, budgeted)?;
            e = e.join(walk_block(then_block, env, mutables, granted, ambient, tick, budgeted)?);
            if let Some(els) = else_block {
                e = e.join(walk_stmt(els, env, mutables, granted, ambient, tick, budgeted)?);
            }
            Ok(e)
        }
        Stmt::For { iter, body, name, span, .. } => {
            let e = walk_expr(iter, env, granted, ambient, tick, budgeted)?;
            if !iter_is_bounded(iter, env) && !budgeted {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    *span,
                    "for-loop is not provably bounded; add take: N or budget(steps: N)",
                ));
            }
            env.insert(name.clone(), Type::Unknown);
            Ok(e.join(walk_block(body, env, mutables, granted, ambient, tick, budgeted)?))
        }
        Stmt::While { cond, body, span, .. } => {
            if is_literal_true(cond) && !budgeted {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    *span,
                    "unbounded while loop",
                ));
            }
            if !budgeted {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    *span,
                    "while-loop requires a budget unless the condition is statically finite",
                ));
            }
            let e = walk_expr(cond, env, granted, ambient, tick, budgeted)?;
            Ok(e.join(walk_block(body, env, mutables, granted, ambient, tick, budgeted)?))
        }
        Stmt::Match { scrutinee, arms, .. } => {
            let mut e = walk_expr(scrutinee, env, granted, ambient, tick, budgeted)?;
            for arm in arms {
                match &arm.body {
                    ArmBody::Block(b) => {
                        e = e.join(walk_block(b, env, mutables, granted, ambient, tick, budgeted)?);
                    }
                    ArmBody::Expr(x) => {
                        e = e.join(walk_expr(x, env, granted, ambient, tick, budgeted)?);
                    }
                }
            }
            Ok(e)
        }
        Stmt::Return { value, .. } | Stmt::Yield { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, env, granted, ambient, tick, budgeted)
            } else {
                Ok(Effect::Pure)
            }
        }
        Stmt::Transaction { body, .. } => {
            let e = walk_block(body, env, mutables, granted, ambient, tick, budgeted)?;
            Ok(e.join(Effect::External))
        }
        Stmt::Effect { expr, span, .. } => {
            if tick {
                return Err(Diagnostic::new(
                    DiagCode::E200,
                    *span,
                    "on tick cannot perform external effects",
                ));
            }
            Ok(walk_expr(expr, env, granted, ambient, tick, budgeted)?.join(Effect::External))
        }
        Stmt::Expr { expr, .. } => walk_expr(expr, env, granted, ambient, tick, budgeted),
        Stmt::Block(b) => walk_block(b, env, mutables, granted, ambient, tick, budgeted),
    }
}

fn walk_expr(
    expr: &Expr,
    env: &mut HashMap<String, Type>,
    granted: &[&str],
    ambient: Effect,
    tick: bool,
    budgeted: bool,
) -> Result<Effect, Diagnostic> {
    match &expr.kind {
        ExprKind::Literal(_)
        | ExprKind::Ident(_)
        | ExprKind::QueryVar(_)
        | ExprKind::Iri(_)
        | ExprKind::Prefixed(_, _)
        | ExprKind::Blank(_) => Ok(Effect::Pure),
        ExprKind::Binary { left, right, .. } => {
            let a = walk_expr(left, env, granted, ambient, tick, budgeted)?;
            let b = walk_expr(right, env, granted, ambient, tick, budgeted)?;
            Ok(a.join(b))
        }
        ExprKind::Unary { expr, .. } | ExprKind::Await(expr) | ExprKind::Try(expr) => {
            walk_expr(expr, env, granted, ambient, tick, budgeted)
        }
        ExprKind::Member { recv, .. } => walk_expr(recv, env, granted, ambient, tick, budgeted),
        ExprKind::Index { recv, index, .. } => {
            let a = walk_expr(recv, env, granted, ambient, tick, budgeted)?;
            let b = walk_expr(index, env, granted, ambient, tick, budgeted)?;
            Ok(a.join(b))
        }
        ExprKind::Call { callee, args, .. } => {
            let path = call_path(callee);
            if tick {
                if let Some(p) = &path {
                    if p.starts_with("graph.") {
                        return Err(Diagnostic::new(
                            DiagCode::E200,
                            expr.span,
                            "on tick must not query the graph",
                        ));
                    }
                }
            }
            if ambient == Effect::Pure {
                if let Some(p) = &path {
                    if binding_effect(p) > Effect::Pure {
                        return Err(Diagnostic::new(
                            DiagCode::E200,
                            expr.span,
                            "Pure cell cannot perform External effects",
                        ));
                    }
                }
            }
            if let Some(p) = &path {
                if let Some(cap) = capability_for(p) {
                    if !granted.iter().any(|g| *g == cap || g.starts_with(cap)) {
                        return Err(Diagnostic::new(
                            DiagCode::E300,
                            expr.span,
                            format!("missing capability(\"{cap}\") for {p}"),
                        ));
                    }
                }
                if p == "graph.query" && !args.iter().any(is_take_arg) {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        expr.span,
                        "graph.query requires take: N",
                    ));
                }
            }
            let mut e = path
                .as_deref()
                .map(binding_effect)
                .unwrap_or(Effect::Pure);
            e = e.join(walk_expr(callee, env, granted, ambient, tick, budgeted)?);
            for a in args {
                let ex = match a {
                    Arg::Pos(x) | Arg::Named(NamedArg { value: x, .. }) => x,
                };
                e = e.join(walk_expr(ex, env, granted, ambient, tick, budgeted)?);
            }
            Ok(e)
        }
        ExprKind::List(xs) => {
            let mut e = Effect::Pure;
            for x in xs {
                e = e.join(walk_expr(x, env, granted, ambient, tick, budgeted)?);
            }
            Ok(e)
        }
        ExprKind::Record(fs) => {
            let mut e = Effect::Pure;
            for f in fs {
                e = e.join(walk_expr(&f.value, env, granted, ambient, tick, budgeted)?);
            }
            Ok(e)
        }
        ExprKind::Triple {
            subject,
            predicate,
            object,
        }
        | ExprKind::Reified {
            subject,
            predicate,
            object,
            ..
        } => {
            let mut e = walk_expr(subject, env, granted, ambient, tick, budgeted)?;
            e = e.join(walk_expr(predicate, env, granted, ambient, tick, budgeted)?);
            e = e.join(walk_expr(object, env, granted, ambient, tick, budgeted)?);
            if let ExprKind::Reified { reifier, .. } = &expr.kind {
                e = e.join(walk_expr(reifier, env, granted, ambient, tick, budgeted)?);
            }
            Ok(e)
        }
    }
}

fn call_path(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Member { recv, name } => {
            if let ExprKind::Ident(ns) = &recv.kind {
                return Some(format!("{ns}.{name}"));
            }
            None
        }
        ExprKind::Ident(n) => Some(n.clone()),
        _ => None,
    }
}

fn is_take_arg(arg: &Arg) -> bool {
    matches!(arg, Arg::Named(n) if n.name == "take")
}

fn is_literal_true(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Literal(Literal::Bool(true)))
}

fn expr_is_bounded_source(expr: &Expr, env: &HashMap<String, Type>) -> bool {
    match &expr.kind {
        ExprKind::Try(inner) => expr_is_bounded_source(inner, env),
        ExprKind::List(_) => true,
        ExprKind::Ident(n) => matches!(env.get(n), Some(Type::List(_))),
        ExprKind::Call { callee, args, .. } => {
            call_path(callee).as_deref() == Some("graph.query") && args.iter().any(is_take_arg)
        }
        _ => false,
    }
}

fn iter_is_bounded(expr: &Expr, env: &HashMap<String, Type>) -> bool {
    expr_is_bounded_source(expr, env)
}

