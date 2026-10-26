//! Cold checker context and monotone local-function effect summaries.
use super::*;
use std::ops::{Deref, DerefMut};

#[derive(Clone, Default)]
pub(crate) struct CheckEnv {
    types: HashMap<String, Type>,
    pub(super) functions: HashMap<String, Effect>,
    pub(super) enum_calls: HashSet<String>,
    pub(super) strict_grants: bool,
    pub(super) budget_expression: bool,
    pub(crate) values: HashMap<String, Expr>,
}

impl Deref for CheckEnv {
    type Target = HashMap<String, Type>;
    fn deref(&self) -> &Self::Target {
        &self.types
    }
}
impl DerefMut for CheckEnv {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.types
    }
}
impl CheckEnv {
    pub(super) fn function_scope(&self) -> Self {
        Self {
            functions: self.functions.clone(),
            enum_calls: self.enum_calls.clone(),
            strict_grants: self.strict_grants,
            budget_expression: self.budget_expression,
            ..Self::default()
        }
    }
}

pub(super) fn infer_functions(
    program: &Program,
    aliases: &HashMap<String, String>,
    granted: &[&str],
) -> HashMap<String, Effect> {
    let mut env = CheckEnv::default();
    collect_enum_calls(program, &mut env);
    // First definition wins, matching Engine::dispatch_call_path.
    for item in &program.items {
        if let Item::Function(f) = item {
            env.functions
                .entry(f.name.clone())
                .or_insert(f.effect.map(Effect::from_class).unwrap_or(Effect::Pure));
        }
    }
    // Five-point lattice: each entry can increase at most four times.
    for _ in 0..=env.functions.len().saturating_mul(4) {
        let mut changed = false;
        let mut seen = HashSet::new();
        for item in &program.items {
            let Item::Function(f) = item else { continue };
            if !seen.insert(&f.name) {
                continue;
            }
            let mut local = env.function_scope();
            for p in &f.params {
                local.insert(p.name.clone(), Type::from_ast(&p.ty));
            }
            let effect = walk_block(
                &f.body,
                &mut local,
                &mut HashSet::new(),
                aliases,
                granted,
                Effect::External,
                false,
                !f.budget.is_empty(),
            )
            .unwrap_or(Effect::External);
            let prior = env.functions[&f.name];
            let next = prior.join(effect);
            changed |= next != prior;
            env.functions.insert(f.name.clone(), next);
        }
        if !changed {
            break;
        }
    }
    env.functions
}

const COLLECTION_METHODS: &[&str] = &[
    "map",
    "filter",
    "find",
    "any",
    "some",
    "all",
    "every",
    "contains",
    "includes",
    "starts_with",
    "ends_with",
    "replace",
    "trim",
    "join",
    "push",
    "slice",
    "has",
    "keys",
];

pub(super) fn collection_call(
    callee: &Expr,
    env: &CheckEnv,
    aliases: &HashMap<String, String>,
) -> bool {
    if let ExprKind::Member { recv, name } = &callee.kind {
        let namespace = matches!(&recv.kind, ExprKind::Ident(n)
            if aliases.contains_key(n) || VALID_IMPORT_NS.contains(&n.as_str()));
        let variable = matches!(&recv.kind, ExprKind::Ident(n) if env.contains_key(n));
        return (!namespace || variable) && COLLECTION_METHODS.contains(&name.as_str());
    }
    false
}

pub(super) fn collect_enum_calls(program: &Program, env: &mut CheckEnv) {
    for item in &program.items {
        if let Item::Enum(e) = item {
            for v in &e.variants {
                env.enum_calls.insert(format!("{}.{}", e.name, v.name));
            }
        }
    }
}

pub(super) fn builtin_call(path: &str) -> bool {
    matches!(
        path,
        "len" | "contains" | "Ok" | "Err" | "vec2" | "vec3" | "vec4" | "mat3" | "mat4" | "oklch"
    )
}

pub(super) fn resolved_call_effect(
    callee: &Expr,
    args: &[Arg],
    env: &CheckEnv,
    aliases: &HashMap<String, String>,
    granted: &[&str],
    budgeted: bool,
) -> Result<Effect, Diagnostic> {
    let path = call_path(callee, aliases);
    if let ExprKind::Member { recv, name } = &callee.kind {
        let is_namespace = matches!(&recv.kind, ExprKind::Ident(n)
            if aliases.contains_key(n) || VALID_IMPORT_NS.contains(&n.as_str()));
        let runtime_var = matches!(&recv.kind, ExprKind::Ident(n) if env.contains_key(n));
        if (!is_namespace || runtime_var) && COLLECTION_METHODS.contains(&name.as_str()) {
            if matches!(
                name.as_str(),
                "map" | "filter" | "find" | "any" | "some" | "all" | "every"
            ) {
                let Some(Arg::Pos(callback)) = args.first() else {
                    return Ok(Effect::External);
                };
                return callback_effect(callback, env, aliases, granted, budgeted, 0);
            }
            return Ok(Effect::Pure);
        }
    }
    if let Some(p) = path {
        if builtin_call(&p) || env.enum_calls.contains(&p) {
            return Ok(Effect::Pure);
        }
        if let Some(effect) = env.functions.get(&p) {
            if env.budget_expression {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    callee.span,
                    "budget expressions must not call user functions",
                ));
            }
            return Ok(*effect);
        }
        if matches!(
            p.as_str(),
            "time.unix_nanos"
                | "time.monotonic_nanos"
                | "time.proper_time"
                | "receipt.clock"
                | "field.sample"
                | "law.apply"
                | "cue.post"
                | "dag.execute"
                | "hid.poll"
                | "hid.wait"
                | "crypto.sign"
                | "crypto.generate_key"
        ) {
            return Ok(Effect::External);
        }
        // Unknown bare names/dynamic targets are not proven pure. Known native
        // namespace paths retain the pinned 0.1 binding classification.
        if super::bindings::BINDING_PATHS.contains(&p.as_str())
            || crate::catalog::is_known(&p)
            || crate::catalog::looks_like_catalog_path(&p)
            || matches!(
                p.as_str(),
                "vec2"
                    | "vec3"
                    | "vec4"
                    | "mat3"
                    | "mat4"
                    | "oklch"
                    | "Ok"
                    | "Err"
                    | "receipt_empty"
            )
        {
            return Ok(binding_effect(&p));
        }
    }
    Ok(Effect::External)
}

pub(super) fn snapshot_value(value: &Expr, env: &CheckEnv) -> Option<Expr> {
    // Vibe values copy on let/assignment. Do not retain an alias expression
    // that would later resolve against a different mutable source binding.
    if let ExprKind::Ident(n) = &value.kind {
        return env.values.get(n).cloned();
    }
    Some(value.clone())
}

fn callback_effect(
    expr: &Expr,
    env: &CheckEnv,
    aliases: &HashMap<String, String>,
    granted: &[&str],
    budgeted: bool,
    depth: u32,
) -> Result<Effect, Diagnostic> {
    if depth >= 64 {
        return Ok(Effect::External);
    }
    match &expr.kind {
        ExprKind::Lambda { params, body } => {
            let mut local = env.function_scope();
            for p in params {
                local.insert(p.clone(), Type::Unknown);
            }
            walk_expr(
                body,
                &mut local,
                aliases,
                granted,
                Effect::External,
                false,
                budgeted,
            )
        }
        ExprKind::Ident(n) => match env.values.get(n) {
            Some(value) => callback_effect(value, env, aliases, granted, budgeted, depth + 1),
            None => Ok(Effect::External),
        },
        // Container-derived and conditional callbacks can be supported after a
        // value-flow contract exists. Unknown never means Pure.
        _ => Ok(Effect::External),
    }
}

/// Check only new executable items against retained, checked declarations.
pub(crate) fn check_submission(
    definitions: &Program,
    submission: &Program,
    prior: &CheckEnv,
    prior_mutables: &HashSet<String>,
) -> Result<(CheckResult, CheckEnv, HashSet<String>), Diagnostic> {
    let mut aliases = HashMap::new();
    let mut errors = Vec::new();
    collect_import_errors(definitions, &mut aliases, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    let granted: Vec<_> = definitions.requires.iter().map(|c| c.id.as_str()).collect();
    let mut env = prior.clone();
    env.strict_grants = true;
    collect_enum_calls(definitions, &mut env);
    env.functions = infer_functions(definitions, &aliases, &granted);
    let mut mutables = prior_mutables.clone();
    // Recheck declaration bodies, never prior top-level statements. Definitions
    // cannot be replaced by this additive 0.1 API.
    for item in &definitions.items {
        if matches!(item, Item::Function(_) | Item::Hook(_)) {
            check_item(item, &mut env, &mut mutables, &aliases, &granted)?;
        }
    }
    // Declarations alone do not execute their effects.
    let mut effect = Effect::Pure;
    for item in &submission.items {
        let e = check_item(item, &mut env, &mut mutables, &aliases, &granted)?;
        if !matches!(item, Item::Function(_) | Item::Hook(_)) {
            effect = effect.join(e);
        }
    }
    Ok((CheckResult { effect }, env, mutables))
}
