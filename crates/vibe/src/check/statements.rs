use super::expressions::infer_expr_type;
use super::*;

pub(super) fn walk_stmt(
    stmt: &Stmt,
    env: &mut CheckEnv,
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
            let t = if let Some(ty) = ty {
                Type::from_ast(ty)
            } else if value
                .as_ref()
                .is_some_and(|v| expr_is_bounded_source(v, env, aliases))
            {
                Type::List(Box::new(Type::Unknown))
            } else {
                ty.as_ref().map(Type::from_ast).unwrap_or(Type::Unknown)
            };
            if ty.is_some() {
                if let Some(v) = value {
                    let inferred = infer_expr_type(v, env, aliases);
                    if !t.is_compatible(&inferred) {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            v.span,
                            format!("type mismatch in `let {name}`: expected {t:?}, got {inferred:?}"),
                        ));
                    }
                }
            }
            env.insert(name.clone(), t);
            if let Some(v) = value {
                if let Some(value) = snapshot_value(v, env) {
                    env.values.insert(name.clone(), value);
                } else {
                    env.values.remove(name);
                }
            } else {
                env.values.remove(name);
            }
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
            let n = target.ident_name().ok_or_else(|| Diagnostic::new(
                DiagCode::E701,
                *span,
                "assignment target must be a mutable whole binding; member/index assignment is unsupported in vibe-0.1",
            ))?;
            if !mutables.contains(n) {
                return Err(Diagnostic::new(
                    DiagCode::E701,
                    *span,
                    format!("cannot assign to immutable binding `{n}` (declare with `let mut`)"),
                ));
            }
            if let Some(value) = snapshot_value(value, env) {
                env.values.insert(n.to_string(), value);
            } else {
                env.values.remove(n);
            }
            let a = walk_expr(target, env, aliases, granted, ambient, tick, budgeted)?;
            let b = walk_expr(value, env, aliases, granted, ambient, tick, budgeted)?;
            if let ExprKind::Ident(name) = &target.kind {
                if let Some(target_ty) = env.get(name) {
                    let rhs_ty = infer_expr_type(value, env, aliases);
                    if !target_ty.is_compatible(&rhs_ty) {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            value.span,
                            format!("cannot assign {rhs_ty:?} to `{name}` of type {target_ty:?}"),
                        ));
                    }
                }
            }
            Ok(a.join(b))
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            let cond_ty = infer_expr_type(cond, env, aliases);
            if !cond_ty.is_compatible(&Type::Bool) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    cond.span,
                    format!("`if` condition must be Bool, got {cond_ty:?}"),
                ));
            }
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
            env.values.remove(name);
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
            let cond_ty = infer_expr_type(cond, env, aliases);
            if !cond_ty.is_compatible(&Type::Bool) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    cond.span,
                    format!("`while` condition must be Bool, got {cond_ty:?}"),
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
                let mut arm_env = env.clone();
                let mut arm_mutables = mutables.clone();
                bind_pattern_in_env(&arm.pattern, &mut arm_env, &mut arm_mutables, false);
                match &arm.body {
                    ArmBody::Block(b) => {
                        e = e.join(walk_block(
                            b,
                            &mut arm_env,
                            &mut arm_mutables,
                            aliases,
                            granted,
                            ambient,
                            tick,
                            budgeted,
                        )?);
                    }
                    ArmBody::Expr(x) => {
                        e = e.join(walk_expr(
                            x,
                            &mut arm_env,
                            aliases,
                            granted,
                            ambient,
                            tick,
                            budgeted,
                        )?);
                    }
                }
            }
            Ok(e)
        }
        Stmt::Return { value, span } => {
            if let Some(expected) = env.expected_return.as_ref() {
                if let Some(v) = value {
                    let actual = infer_expr_type(v, env, aliases);
                    if !expected.is_compatible(&actual) {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            v.span,
                            format!("return type mismatch: expected {expected:?}, got {actual:?}"),
                        ));
                    }
                } else if *expected != Type::Unknown {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        *span,
                        format!("missing return value of type {expected:?}"),
                    ));
                }
            }
            if let Some(v) = value {
                walk_expr(v, env, aliases, granted, ambient, tick, budgeted)
            } else {
                Ok(Effect::Pure)
            }
        }
        Stmt::Yield { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, env, aliases, granted, ambient, tick, budgeted)
            } else {
                Ok(Effect::Pure)
            }
        }
        Stmt::Transaction { body, .. } => {
            if env.strict_grants && !granted.iter().any(|g| *g == "graph.write") {
                return Err(Diagnostic::new(
                    DiagCode::E300,
                    stmt.span(),
                    "session transaction requires admitted graph.write capability",
                ));
            }
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
