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

/// Collect up to 8 diagnostics instead of stopping at the first (audit §7.12).
pub const MAX_DIAGNOSTICS: usize = 8;

const VALID_IMPORT_NS: &[&str] = &[
    "math",
    "rdf",
    "quin",
    "graph",
    "aura",
    "pulse",
    "capability",
    "time",
    "conservation",
    "causal",
    "dag",
    "deontic",
    "hid",
    "cue",
    "crypto",
    "zk",
];

pub fn check_program_all(program: &Program) -> (CheckResult, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut aliases = HashMap::new();
    collect_import_errors(program, &mut aliases, &mut errors);

    let granted: Vec<&str> = program.requires.iter().map(|c| c.id.as_str()).collect();
    let mut max_effect = Effect::Pure;
    let mut env = HashMap::new();
    let mut mutables = HashSet::new();
    for item in &program.items {
        if errors.len() >= MAX_DIAGNOSTICS {
            break;
        }
        match check_item(item, &mut env, &mut mutables, &aliases, &granted) {
            Ok(e) => max_effect = max_effect.join(e),
            Err(e) => {
                if !errors
                    .iter()
                    .any(|x| x.span == e.span && x.message == e.message)
                {
                    errors.push(e);
                }
            }
        }
    }
    (CheckResult { effect: max_effect }, errors)
}

pub fn check_program(program: &Program) -> Result<CheckResult, Diagnostic> {
    let (result, errors) = check_program_all(program);
    match errors.into_iter().next() {
        Some(e) => Err(e),
        None => Ok(result),
    }
}

/// After a successful check, fail closed on prefixed names missing from a loaded vocab chunk.
pub fn check_program_with_vocab(
    program: &Program,
    chunks: &[crate::vocab::VocabChunk],
) -> Result<CheckResult, Diagnostic> {
    let result = check_program(program)?;
    if let Some((prefix, local, span)) = crate::vocab::unknown_prefixed(program, chunks)
        .into_iter()
        .next()
    {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("unknown vocab term `{prefix}:{local}` (not in loaded chunk)"),
        ));
    }
    Ok(result)
}

fn collect_import_errors(
    program: &Program,
    aliases: &mut HashMap<String, String>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut seen_aliases = HashSet::new();
    for imp in &program.imports {
        if errors.len() >= MAX_DIAGNOSTICS {
            return;
        }
        let path = imp.path.as_str();
        // The vibe:0.1/ prefix is optional (T64). The version lives on
        // the AST tag, not in a sacred string prefix. Both
        // `import "vibe:0.1/math" as math;` and `import "math" as math;`
        // are valid.
        let ns = path.strip_prefix("vibe:0.1/").unwrap_or(path);
        if !VALID_IMPORT_NS.contains(&ns) {
            errors.push(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!(
                    "unknown namespace '{ns}'; valid: {}",
                    VALID_IMPORT_NS.join(", ")
                ),
            ));
            continue;
        }
        let alias = imp.alias.as_deref().unwrap_or(ns);
        if !seen_aliases.insert(alias) {
            errors.push(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!("duplicate import alias '{alias}'"),
            ));
            continue;
        }
        aliases.insert(alias.to_string(), ns.to_string());
    }
}

fn check_item(
    item: &Item,
    env: &mut HashMap<String, Type>,
    mutables: &mut HashSet<String>,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    match item {
        Item::Function(f) => check_function(f, aliases, granted),
        Item::Hook(h) => check_hook(h, aliases, granted),
        Item::Const(c) => {
            walk_expr(&c.value, env, aliases, granted, Effect::Pure, false, false)?;
            Ok(Effect::Pure)
        }
        Item::Statement(s) => walk_stmt(
            s,
            env,
            mutables,
            aliases,
            granted,
            Effect::External,
            false,
            false,
        ),
        Item::Enum(_) => Ok(Effect::Pure),
        Item::Field(f) => {
            let ty = crate::types::Type::from_ast(&f.ty);
            let is_physical = matches!(
                ty,
                crate::types::Type::F64
                    | crate::types::Type::Quantity
                    | crate::types::Type::I64
                    | crate::types::Type::U64
            );
            if is_physical && f.unit.is_none() {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    f.span,
                    format!(
                        "field '{}' has physical type '{}' but no unit IRI — \
                         physical fields require an explicit unit (X5). \
                         Use `qudt:DimensionlessUnit` for dimensionless quantities.",
                        f.name, f.ty.name
                    ),
                ));
            }
            Ok(Effect::Pure)
        }
        Item::Material(_) => Ok(Effect::Pure),
        Item::Cell(c) => {
            let declared = c.effect.map(Effect::from_class).unwrap_or(Effect::Pure);
            let mut cell_env = env.clone();
            for p in &c.params {
                cell_env.insert(p.name.clone(), Type::from_ast(&p.ty));
            }
            let mut effect = Effect::Pure;
            if let Some(w) = &c.when {
                let we = walk_expr(w, &mut cell_env, aliases, granted, declared, false, false)?;
                effect = effect.join(we);
            }
            let e = walk_expr(
                &c.expr,
                &mut cell_env,
                aliases,
                granted,
                declared,
                false,
                false,
            )?;
            if e > declared && c.effect.is_some() {
                return Err(Diagnostic::new(
                    DiagCode::E200,
                    c.span,
                    "cell expression exceeds declared effect class",
                ));
            }
            Ok(effect.join(e))
        }
        Item::Law(l) => {
            walk_expr(
                &l.condition,
                env,
                aliases,
                granted,
                Effect::Pure,
                false,
                false,
            )?;
            walk_expr(
                &l.consequence,
                env,
                aliases,
                granted,
                Effect::External,
                false,
                false,
            )?;
            Ok(Effect::External)
        }
        Item::Bind(b) => {
            walk_expr(
                &b.left,
                env,
                aliases,
                granted,
                Effect::External,
                false,
                false,
            )?;
            walk_expr(
                &b.right,
                env,
                aliases,
                granted,
                Effect::External,
                false,
                false,
            )?;
            if let Some((lo, hi)) = &b.clamp {
                walk_expr(lo, env, aliases, granted, Effect::Pure, false, false)?;
                walk_expr(hi, env, aliases, granted, Effect::Pure, false, false)?;
            }
            Ok(Effect::Pure)
        }
        Item::Present(p) => {
            let mut effect = Effect::Pure;
            for prop in &p.properties {
                let e = walk_expr(
                    &prop.value,
                    env,
                    aliases,
                    granted,
                    Effect::External,
                    false,
                    false,
                )?;
                effect = effect.join(e);
            }
            Ok(effect)
        }
    }
}

pub fn check_cell(expr: &Expr) -> Result<CheckResult, Diagnostic> {
    let mut env = HashMap::new();
    let aliases = HashMap::new();
    let effect = walk_expr(expr, &mut env, &aliases, &[], Effect::Pure, true, false)?;
    if effect > Effect::Pure {
        return Err(Diagnostic::new(
            DiagCode::E200,
            expr.span,
            "Pure cell cannot perform External effects",
        ));
    }
    Ok(CheckResult { effect })
}

pub(crate) fn check_function(
    f: &FunctionDecl,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    let declared = f.effect.map(Effect::from_class).unwrap_or(Effect::Pure);
    let mut env = HashMap::new();
    let mut mutables = HashSet::new();
    for p in &f.params {
        env.insert(p.name.clone(), Type::from_ast(&p.ty));
    }
    let has_budget = !f.budget.is_empty();
    let body = walk_block(
        &f.body,
        &mut env,
        &mut mutables,
        aliases,
        granted,
        declared,
        false,
        has_budget,
    )?;
    if body > declared && f.effect.is_some() {
        return Err(Diagnostic::new(
            DiagCode::E200,
            f.span,
            "function body exceeds declared effect class",
        ));
    }
    Ok(body.join(declared))
}

fn check_hook(
    h: &HookDecl,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    let is_tick = h.path == ["tick"];
    let declared = if is_tick {
        Effect::Hot
    } else {
        Effect::External
    };
    let mut env = HashMap::new();
    let mut mutables = HashSet::new();
    for p in &h.params {
        env.insert(p.name.clone(), Type::from_ast(&p.ty));
    }
    let has_budget = !h.budget.is_empty();
    walk_block(
        &h.body,
        &mut env,
        &mut mutables,
        aliases,
        granted,
        declared,
        is_tick,
        has_budget,
    )
}

fn walk_block(
    block: &Block,
    env: &mut HashMap<String, Type>,
    mutables: &mut HashSet<String>,
    aliases: &HashMap<String, String>,
    granted: &[&str],
    ambient: Effect,
    tick: bool,
    budgeted: bool,
) -> Result<Effect, Diagnostic> {
    let mut scoped_env = env.clone();
    let mut scoped_mutables = mutables.clone();
    let mut e = Effect::Pure;
    for s in &block.stmts {
        e = e.join(walk_stmt(
            s,
            &mut scoped_env,
            &mut scoped_mutables,
            aliases,
            granted,
            ambient,
            tick,
            budgeted,
        )?);
    }
    Ok(e)
}

fn walk_stmt(
    stmt: &Stmt,
    env: &mut HashMap<String, Type>,
    mutables: &mut HashSet<String>,
    aliases: &HashMap<String, String>,
    granted: &[&str],
    ambient: Effect,
    tick: bool,
    budgeted: bool,
) -> Result<Effect, Diagnostic> {
    match stmt {
        Stmt::Let {
            mutable,
            name,
            value,
            ty,
            ..
        } => {
            let t = if value
                .as_ref()
                .is_some_and(|v| expr_is_bounded_source(v, env, aliases))
            {
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
                return walk_expr(v, env, aliases, granted, ambient, tick, budgeted);
            }
            Ok(Effect::Pure)
        }
        Stmt::LetPat {
            mutable,
            pattern,
            value,
            ..
        } => {
            bind_pattern_in_env(pattern, env, mutables, *mutable);
            walk_expr(value, env, aliases, granted, ambient, tick, budgeted)
        }
        Stmt::Assign {
            target,
            value,
            span,
        } => {
            if let Some(n) = target.ident_name() {
                if !mutables.contains(n) {
                    return Err(Diagnostic::new(
                        DiagCode::E701,
                        *span,
                        format!(
                            "cannot assign to immutable binding `{n}` (declare with `let mut`)"
                        ),
                    ));
                }
            }
            let a = walk_expr(target, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(value, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(a.join(b))
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            let mut e = walk_expr(cond, env, aliases, granted, ambient, tick, budgeted)?;
            e = e.join(walk_block(
                then_block, env, mutables, aliases, granted, ambient, tick, budgeted,
            )?);
            if let Some(els) = else_block {
                e = e.join(walk_stmt(
                    els, env, mutables, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            Ok(e)
        }
        Stmt::For {
            iter,
            body,
            name,
            span,
            ..
        } => {
            let e = walk_expr(iter, env, aliases, granted, ambient, tick, budgeted)?;
            if !iter_is_bounded(iter, env, aliases) && !budgeted {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    *span,
                    "for-loop is not provably bounded; add take: N or budget(steps: N)",
                ));
            }
            env.insert(name.clone(), Type::Unknown);
            Ok(e.join(walk_block(
                body, env, mutables, aliases, granted, ambient, tick, budgeted,
            )?))
        }
        Stmt::While {
            cond, body, span, ..
        } => {
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
            let e = walk_expr(cond, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(e.join(walk_block(
                body, env, mutables, aliases, granted, ambient, tick, budgeted,
            )?))
        }
        Stmt::Match {
            scrutinee, arms, ..
        } => {
            let mut e = walk_expr(scrutinee, env, aliases, granted, ambient, tick, budgeted)?;
            for arm in arms {
                match &arm.body {
                    ArmBody::Block(b) => {
                        e = e.join(walk_block(
                            b, env, mutables, aliases, granted, ambient, tick, budgeted,
                        )?);
                    }
                    ArmBody::Expr(x) => {
                        e = e.join(walk_expr(
                            x, env, aliases, granted, ambient, tick, budgeted,
                        )?);
                    }
                }
            }
            Ok(e)
        }
        Stmt::Return { value, .. } | Stmt::Yield { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, env, aliases, granted, ambient, tick, budgeted)
            } else {
                Ok(Effect::Pure)
            }
        }
        Stmt::Transaction { body, .. } => {
            let e = walk_block(
                body, env, mutables, aliases, granted, ambient, tick, budgeted,
            )?;
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
            Ok(
                walk_expr(expr, env, aliases, granted, ambient, tick, budgeted)?
                    .join(Effect::External),
            )
        }
        Stmt::Expr { expr, .. } => walk_expr(expr, env, aliases, granted, ambient, tick, budgeted),
        Stmt::Block(b) => walk_block(b, env, mutables, aliases, granted, ambient, tick, budgeted),
    }
}

fn walk_expr(
    expr: &Expr,
    env: &mut HashMap<String, Type>,
    aliases: &HashMap<String, String>,
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
        ExprKind::Binary { op, left, right } => {
            if matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
            ) {
                if let (Some(u1), Some(u2)) = (quantity_unit_of(left), quantity_unit_of(right)) {
                    if u1 != u2 {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            expr.span,
                            format!("unit mismatch: cannot {op:?} quantities of `{u1}` and `{u2}`"),
                        ));
                    }
                }
            }
            let a = walk_expr(left, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(right, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(a.join(b))
        }
        ExprKind::Unary { expr, .. } | ExprKind::Await(expr) | ExprKind::Try(expr) => {
            walk_expr(expr, env, aliases, granted, ambient, tick, budgeted)
        }
        ExprKind::Member { recv, .. } => {
            walk_expr(recv, env, aliases, granted, ambient, tick, budgeted)
        }
        ExprKind::Index { recv, index, .. } => {
            let a = walk_expr(recv, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(index, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(a.join(b))
        }
        ExprKind::Call { callee, args, .. } => {
            let path = call_path(callee, aliases);
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
                if p == "capability.invoke" {
                    let target_id = args.iter().find_map(|a| match a {
                        Arg::Pos(Expr {
                            kind: ExprKind::Literal(Literal::String(s)),
                            ..
                        }) => Some(s.as_str()),
                        Arg::Named(NamedArg {
                            name,
                            value:
                                Expr {
                                    kind: ExprKind::Literal(Literal::String(s)),
                                    ..
                                },
                            ..
                        }) if name == "capability" || name == "id" => Some(s.as_str()),
                        _ => None,
                    });
                    if let Some(target) = target_id {
                        if crate::catalog::looks_like_catalog_path(target)
                            && !crate::catalog::granted_covers(granted, target)
                        {
                            return Err(Diagnostic::new(
                                DiagCode::E300,
                                expr.span,
                                format!("missing capability(\"{target}\") for capability.invoke(\"{target}\")"),
                            ));
                        }
                    }
                }
                if crate::catalog::looks_like_catalog_path(p) {
                    if !crate::catalog::is_known(p) {
                        let mut msg = format!("unknown capability `{p}`");
                        if let Some(s) = crate::catalog::did_you_mean(p) {
                            msg.push_str(&format!("; did you mean `{s}`?"));
                        }
                        return Err(Diagnostic::new(DiagCode::E100, expr.span, msg));
                    }
                    if !crate::catalog::granted_covers(granted, p) {
                        let fam = crate::catalog::family_of(p).unwrap_or(p);
                        return Err(Diagnostic::new(
                            DiagCode::E300,
                            expr.span,
                            format!("missing capability(\"{p}\") for {p}"),
                        )
                        .with_fix(format!(
                            "add `using {fam};` or requires [ capability(\"{p}\") ];"
                        )));
                    }
                } else if let Some(cap) = capability_for(p) {
                    if !granted.iter().any(|g| *g == cap || *g == *p) {
                        return Err(Diagnostic::new(
                            DiagCode::E300,
                            expr.span,
                            format!("missing capability(\"{cap}\") for {p}"),
                        )
                        .with_fix(format!("add requires [ capability(\"{cap}\") ];")));
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
            let mut e = path.as_deref().map(binding_effect).unwrap_or(Effect::Pure);
            e = e.join(walk_expr(
                callee, env, aliases, granted, ambient, tick, budgeted,
            )?);
            for a in args {
                let ex = match a {
                    Arg::Pos(x) | Arg::Named(NamedArg { value: x, .. }) => x,
                };
                e = e.join(walk_expr(
                    ex, env, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            Ok(e)
        }
        ExprKind::List(xs) => {
            let mut e = Effect::Pure;
            for x in xs {
                e = e.join(walk_expr(
                    x, env, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            Ok(e)
        }
        ExprKind::Record(fs) => {
            let mut e = Effect::Pure;
            for f in fs {
                e = e.join(walk_expr(
                    &f.value, env, aliases, granted, ambient, tick, budgeted,
                )?);
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
            let mut e = walk_expr(subject, env, aliases, granted, ambient, tick, budgeted)?;
            e = e.join(walk_expr(
                predicate, env, aliases, granted, ambient, tick, budgeted,
            )?);
            e = e.join(walk_expr(
                object, env, aliases, granted, ambient, tick, budgeted,
            )?);
            if let ExprKind::Reified { reifier, .. } = &expr.kind {
                e = e.join(walk_expr(
                    reifier, env, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            Ok(e)
        }
        ExprKind::Pipe { left, right } => {
            let e1 = walk_expr(left, env, aliases, granted, ambient, tick, budgeted)?;
            let e2 = walk_expr(right, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(e1.join(e2))
        }
        ExprKind::GraphQuery { .. } => {
            let ok = granted.iter().any(|g| {
                *g == "graph.read"
                    || *g == "GraphDatabase.sparql"
                    || g.eq_ignore_ascii_case("GraphDatabase")
            });
            if !ok {
                return Err(Diagnostic::new(
                    DiagCode::E300,
                    expr.span,
                    "graph? requires capability(\"graph.read\") or using GraphDatabase",
                )
                .with_fix(
                    "add `using GraphDatabase;` or requires [ capability(\"graph.read\") ];",
                ));
            }
            Ok(Effect::External)
        }
        ExprKind::ModalLogic { args, body, .. } => {
            let mut e = Effect::Pure;
            for a in args {
                e = e.join(walk_expr(
                    a, env, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            if let Some(b) = body {
                e = e.join(walk_expr(
                    b, env, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            Ok(e)
        }
        ExprKind::Interpolate(parts) => {
            let mut e = Effect::Pure;
            for p in parts {
                e = e.join(walk_expr(
                    p, env, aliases, granted, ambient, tick, budgeted,
                )?);
            }
            Ok(e)
        }
        ExprKind::Lambda { params, body } => {
            let mut inner = env.clone();
            for p in params {
                inner.insert(p.clone(), Type::Unknown);
            }
            walk_expr(body, &mut inner, aliases, granted, ambient, tick, budgeted)
        }
        ExprKind::Tween { from, to, over, .. } => {
            let a = walk_expr(from, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(to, env, aliases, granted, ambient, tick, budgeted)?;
            let c = walk_expr(over, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(a.join(b).join(c))
        }
    }
}

fn quantity_unit_of(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Literal(Literal::Quantity { unit, .. }) => Some(unit.as_str()),
        _ => None,
    }
}

fn bind_pattern_in_env(
    pat: &Pattern,
    env: &mut HashMap<String, Type>,
    mutables: &mut HashSet<String>,
    mutable: bool,
) {
    match pat {
        Pattern::Ident(name) => {
            env.insert(name.clone(), Type::Unknown);
            if mutable {
                mutables.insert(name.clone());
            } else {
                mutables.remove(name);
            }
        }
        Pattern::Record(fields) => {
            for (_, p) in fields {
                bind_pattern_in_env(p, env, mutables, mutable);
            }
        }
        Pattern::List(elements) => {
            for p in elements {
                bind_pattern_in_env(p, env, mutables, mutable);
            }
        }
        Pattern::Constructor { args, .. } | Pattern::Variant { args, .. } => {
            for p in args {
                bind_pattern_in_env(p, env, mutables, mutable);
            }
        }
        Pattern::Ok(p) | Pattern::Err(p) | Pattern::Some(p) => {
            bind_pattern_in_env(p, env, mutables, mutable);
        }
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::None => {}
    }
}

fn call_path(expr: &Expr, aliases: &HashMap<String, String>) -> Option<String> {
    match &expr.kind {
        ExprKind::Member { recv, name } => {
            if let ExprKind::Ident(ns) = &recv.kind {
                let resolved_ns = aliases.get(ns).map(|s| s.as_str()).unwrap_or(ns);
                return Some(format!("{resolved_ns}.{name}"));
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

fn expr_is_bounded_source(
    expr: &Expr,
    env: &HashMap<String, Type>,
    aliases: &HashMap<String, String>,
) -> bool {
    match &expr.kind {
        ExprKind::Try(inner) => expr_is_bounded_source(inner, env, aliases),
        ExprKind::List(_) => true,
        ExprKind::Ident(n) => matches!(env.get(n), Some(Type::List(_))),
        ExprKind::Call { callee, args, .. } => {
            call_path(callee, aliases).as_deref() == Some("graph.query")
                && args.iter().any(is_take_arg)
        }
        _ => false,
    }
}

fn iter_is_bounded(
    iter: &Expr,
    env: &HashMap<String, Type>,
    aliases: &HashMap<String, String>,
) -> bool {
    expr_is_bounded_source(iter, env, aliases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_program;

    #[test]
    fn check_program_all_collects_function_and_cell_errors() {
        let src = r#"
            fn a() { return HID.poll(); }
            fn b() { return HID.poll(); }
            cell c := HID.poll();
        "#;
        let program = parse_program(src).expect("parse");
        let (_, errors) = check_program_all(&program);
        assert!(
            errors.len() >= 3,
            "expected function + cell diagnostics, got {}",
            errors.len()
        );
        assert!(errors.len() <= MAX_DIAGNOSTICS);
    }

    #[test]
    fn check_program_all_caps_at_eight() {
        let mut src = String::new();
        for i in 0..12 {
            src.push_str(&format!("fn f{i}() {{ return HID.poll(); }}\n"));
        }
        let program = parse_program(&src).expect("parse");
        let (_, errors) = check_program_all(&program);
        assert_eq!(errors.len(), MAX_DIAGNOSTICS);
    }

    #[test]
    fn vocab_unknown_term_is_check_error() {
        let chunk = crate::vocab::parse_chunk(include_bytes!("../fixtures/vocab/clinic.n3"))
            .expect("chunk");
        let program = parse_program(
            r#"
            prefix snomed: <http://snomed.info/id/>;
            pure fn f() { return snomed:not_in_chunk; }
            "#,
        )
        .expect("parse");
        let err = check_program_with_vocab(&program, &[chunk]).expect_err("unknown");
        assert_eq!(err.code, DiagCode::E100);
        assert!(err.message.contains("snomed:not_in_chunk"));
    }

    #[test]
    fn vocab_known_clinic_term_checks() {
        let chunk = crate::vocab::parse_chunk(include_bytes!("../fixtures/vocab/clinic.n3"))
            .expect("chunk");
        let program = parse_program(
            r#"
            prefix snomed: <http://snomed.info/id/>;
            pure fn f() { return snomed:386661006; }
            "#,
        )
        .expect("parse");
        check_program_with_vocab(&program, &[chunk]).expect("known term");
    }
}
