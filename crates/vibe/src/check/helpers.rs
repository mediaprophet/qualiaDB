use super::*;

pub(super) fn invalidate_pattern(pattern: &Pattern, env: &mut CheckEnv) {
    let mut unused = HashSet::new();
    bind_pattern_in_env(pattern, env, &mut unused, false);
}

pub(super) fn root_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Ident(n) => Some(n),
        ExprKind::Member { recv, .. } | ExprKind::Index { recv, .. } => root_name(recv),
        _ => None,
    }
}

pub(super) fn quantity_unit_of(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Literal(Literal::Quantity { unit, .. }) => Some(unit.as_str()),
        _ => None,
    }
}

pub(super) fn bind_pattern_in_env(
    pat: &Pattern,
    env: &mut CheckEnv,
    mutables: &mut HashSet<String>,
    mutable: bool,
) {
    match pat {
        Pattern::Ident(name) => {
            env.values.remove(name);
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

pub(super) fn call_path(expr: &Expr, aliases: &HashMap<String, String>) -> Option<String> {
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

pub(super) fn is_take_arg(arg: &Arg) -> bool {
    matches!(arg, Arg::Named(n) if n.name == "take")
}

pub(super) fn is_literal_true(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Literal(Literal::Bool(true)))
}

pub(super) fn expr_is_bounded_source(
    expr: &Expr,
    env: &CheckEnv,
    aliases: &HashMap<String, String>,
) -> bool {
    match &expr.kind {
        ExprKind::Try(inner) => expr_is_bounded_source(inner, env, aliases),
        ExprKind::IfElse {
            then_value,
            else_value,
            ..
        } => {
            expr_is_bounded_source(then_value, env, aliases)
                && expr_is_bounded_source(else_value, env, aliases)
        }
        ExprKind::List(_) => true,
        ExprKind::Ident(n) => matches!(env.get(n), Some(Type::List(_))),
        ExprKind::Call { callee, args, .. } => {
            call_path(callee, aliases).as_deref() == Some("graph.query")
                && args.iter().any(is_take_arg)
        }
        _ => false,
    }
}

pub(super) fn iter_is_bounded(
    iter: &Expr,
    env: &CheckEnv,
    aliases: &HashMap<String, String>,
) -> bool {
    expr_is_bounded_source(iter, env, aliases)
}
