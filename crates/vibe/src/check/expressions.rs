use super::*;

pub(super) fn walk_expr(
    expr: &Expr,
    env: &mut CheckEnv,
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
        ExprKind::Unary { expr, .. } | ExprKind::Try(expr) => {
            walk_expr(expr, env, aliases, granted, ambient, tick, budgeted)
        }
        ExprKind::Await(inner) => {
            let operand = walk_expr(inner, env, aliases, granted, ambient, tick, budgeted)?;
            // Literal values cannot contain task references. Dynamic tasks can
            // execute a checked program with External effects; no effect-polymorphic
            // task type exists in the pinned dialect yet.
            if matches!(&inner.kind, ExprKind::Literal(_)) {
                return Ok(operand);
            }
            if ambient < Effect::External {
                return Err(Diagnostic::new(DiagCode::E200, expr.span,
                    "dynamic await requires an effect function; deferred child effects are not typed"));
            }
            Ok(operand.join(Effect::External))
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
            let call_effect = resolved_call_effect(callee, args, env, aliases, granted, budgeted)?;
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
            if ambient == Effect::Pure
                && !path.as_ref().is_some_and(|p| {
                    env.functions.contains_key(p) || env.enum_calls.contains(p) || builtin_call(p)
                })
            {
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
            if call_effect > ambient {
                return Err(Diagnostic::new(
                    DiagCode::E200,
                    expr.span,
                    "call exceeds permitted effect class",
                ));
            }
            if let Some(p) = path.as_ref().filter(|p| {
                !env.functions.contains_key(*p) && !env.enum_calls.contains(*p) && !builtin_call(p)
            }) {
                if env.strict_grants
                    && call_effect > Effect::Pure
                    && !collection_call(callee, env, aliases)
                {
                    let cap = capability_for(p).unwrap_or(p);
                    let allowed = granted.iter().any(|g| *g == cap || *g == p)
                        || (crate::catalog::is_known(p)
                            && crate::catalog::granted_covers(granted, p));
                    if !allowed {
                        return Err(Diagnostic::new(
                            DiagCode::E300,
                            expr.span,
                            format!("session requires an admitted capability request for `{cap}`"),
                        ));
                    }
                }
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
            let mut e = call_effect;
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
            let e2 = match &right.kind {
                ExprKind::Call { .. } => {
                    walk_expr(right, env, aliases, granted, ambient, tick, budgeted)?
                }
                _ => {
                    let call = Expr {
                        span: right.span,
                        kind: ExprKind::Call {
                            callee: right.clone(),
                            args: Vec::new(),
                        },
                    };
                    walk_expr(&call, env, aliases, granted, ambient, tick, budgeted)?
                }
            };
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
                inner.values.remove(p);
            }
            walk_expr(body, &mut inner, aliases, granted, ambient, tick, budgeted)
        }
        ExprKind::Tween { from, to, over, .. } => {
            let a = walk_expr(from, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(to, env, aliases, granted, ambient, tick, budgeted)?;
            let c = walk_expr(over, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(a.join(b).join(c))
        }
        ExprKind::IfElse {
            cond,
            then_value,
            else_value,
        } => {
            // Both branches are inspected statically even though only one runs.
            let a = walk_expr(cond, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(then_value, env, aliases, granted, ambient, tick, budgeted)?;
            let c = walk_expr(else_value, env, aliases, granted, ambient, tick, budgeted)?;
            Ok(a.join(b).join(c))
        }
    }
}
