//! Type and effect checker.

use crate::ast::*;
use crate::effects::{binding_effect, capability_for, Effect};
use crate::error::{DiagCode, Diagnostic};
use crate::types::Type;
use std::collections::{HashMap, HashSet};
mod bindings;
mod context;
mod expressions;
mod helpers;
mod statements;
#[cfg(test)]
mod tests;
use context::collection_call;
use context::snapshot_value;
use context::{builtin_call, collect_enum_calls, infer_functions, resolved_call_effect};
pub(crate) use context::{check_submission, CheckEnv};
use expressions::walk_expr;
use helpers::*;
use statements::walk_stmt;

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
    "web",
    "ui",
    "form",
    "input",
    "validate",
    "selection",
    "layout",
    "cbor_ld",
    "reflect",
    "record",
    "uri",
    "error",
    "num",
    "decimal",
    "autodiff",
    "str",
    "regex",
    "list",
    "deque",
    "pqueue",
    "set",
    "simplex",
    "hypergraph",
    "sheaf",
    "topology",
    "homotopy",
    "context",
    "manifold",
    "inference",
    "ast",
    "diagnostic",
    "effect",
    "code",
    "task",
    "supervisor",
    "contract",
    "state",
    "value",
    "runtime",
    "continuation",
    "memory",
    "simd",
    "channel",
];

/// Checks program-all.
pub fn check_program_all(program: &Program) -> (CheckResult, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut aliases = HashMap::new();
    collect_import_errors(program, &mut aliases, &mut errors);

    let granted: Vec<&str> = program.requires.iter().map(|c| c.id.as_str()).collect();
    let mut max_effect = Effect::Pure;
    let mut env = CheckEnv::default();
    collect_enum_calls(program, &mut env);
    env.functions = infer_functions(program, &aliases, &granted);
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

/// Checks program.
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
    env: &mut CheckEnv,
    mutables: &mut HashSet<String>,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    match item {
        Item::Function(f) => check_function_in(f, env, aliases, granted),
        Item::Hook(h) => check_hook(h, env, aliases, granted),
        Item::Const(c) => {
            let effect = walk_expr(&c.value, env, aliases, granted, Effect::Pure, false, false)?;
            if effect > Effect::Pure {
                return Err(Diagnostic::new(
                    DiagCode::E200,
                    c.span,
                    "constant expression exceeds Pure effect class",
                ));
            }
            env.insert(
                c.name.clone(),
                c.ty.as_ref().map(Type::from_ast).unwrap_or(Type::Unknown),
            );
            if let Some(value) = snapshot_value(&c.value, env) {
                env.values.insert(c.name.clone(), value);
            } else {
                env.values.remove(&c.name);
            }
            Ok(effect)
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
        Item::Material(m) => {
            let mut effect = Effect::Pure;
            for prop in &m.properties {
                effect = effect.join(walk_expr(
                    &prop.value,
                    env,
                    aliases,
                    granted,
                    Effect::External,
                    false,
                    false,
                )?);
            }
            Ok(effect)
        }
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
            if e.join(effect) > declared {
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
            for endpoint in [&b.left, &b.right] {
                if let Some(name) = root_name(endpoint) {
                    env.values.remove(name);
                }
            }
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

/// Checks cell.
pub fn check_cell(expr: &Expr) -> Result<CheckResult, Diagnostic> {
    let mut env = CheckEnv::default();
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

/// Checks function.
fn check_function_in(
    f: &FunctionDecl,
    outer: &CheckEnv,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    let declared = f.effect.map(Effect::from_class).unwrap_or(Effect::Pure);
    let mut env = outer.function_scope();
    let mut mutables = HashSet::new();
    for p in &f.params {
        env.insert(p.name.clone(), Type::from_ast(&p.ty));
    }
    let has_budget = !f.budget.is_empty();
    for arg in &f.budget {
        env.budget_expression = true;
        let effect = walk_expr(
            &arg.value,
            &mut env,
            aliases,
            granted,
            Effect::Pure,
            false,
            false,
        )?;
        env.budget_expression = false;
        if effect > Effect::Pure {
            return Err(Diagnostic::new(
                DiagCode::E200,
                arg.span,
                "budget expression must be Pure",
            ));
        }
    }
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
    outer: &CheckEnv,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> Result<Effect, Diagnostic> {
    let is_tick = h.path == ["tick"];
    let declared = if is_tick {
        Effect::Hot
    } else {
        Effect::External
    };
    let mut env = outer.function_scope();
    let mut mutables = HashSet::new();
    for p in &h.params {
        env.insert(p.name.clone(), Type::from_ast(&p.ty));
    }
    let has_budget = !h.budget.is_empty();
    for arg in &h.budget {
        env.budget_expression = true;
        let effect = walk_expr(
            &arg.value,
            &mut env,
            aliases,
            granted,
            Effect::Pure,
            false,
            false,
        )?;
        env.budget_expression = false;
        if effect > Effect::Pure {
            return Err(Diagnostic::new(
                DiagCode::E200,
                arg.span,
                "budget expression must be Pure",
            ));
        }
    }
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
    env: &mut CheckEnv,
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
    // The 0.1 evaluator shares Env across executable blocks. The checker keeps
    // lexical type checking, but cannot retain stale callable facts for writes
    // that may have happened at runtime in any branch/iteration.
    env.values
        .retain(|name, value| scoped_env.values.get(name) == Some(value));
    Ok(e)
}
