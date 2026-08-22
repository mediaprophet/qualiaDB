use super::*;
use crate::bind::dispatch;
use crate::value::EnumValue;

impl<'a, H: Host> Engine<'a, H> {
    pub fn eval_expr(&mut self, expr: &Expr, env: &mut Env) -> Result<Value, Diagnostic> {
        self.budget.tick(expr.span)?;
        match &expr.kind {
            ExprKind::Literal(l) => {
                let v = lit(l);
                match &v {
                    Value::String(s) => self.budget.charge(s.len() as u64, expr.span)?,
                    Value::Quantity(q) => {
                        self.budget.charge(16 + q.unit.len() as u64, expr.span)?;
                    }
                    _ => {}
                }
                Ok(v)
            }
            ExprKind::Ident(n) => {
                if n == "None" {
                    return Ok(Value::Null);
                }
                // Check vars, then fields, then materials, then laws.
                if let Some(v) = env.vars.get(n) {
                    return Ok(v.clone());
                }
                if let Some(v) = env.fields.get(n) {
                    return Ok(v.clone());
                }
                if let Some(v) = env.materials.get(n) {
                    return Ok(v.clone());
                }
                if let Some(v) = env.laws.get(n) {
                    return Ok(v.clone());
                }
                Err(Diagnostic::new(
                    DiagCode::E600,
                    expr.span,
                    format!("undefined {n}"),
                ))
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
                        Value::I64(n) => n.checked_neg().map(Value::I64).ok_or_else(|| {
                            Diagnostic::new(DiagCode::E600, expr.span, "integer overflow")
                        }),
                        Value::U64(n) => {
                            if n == 0 {
                                Ok(Value::U64(0))
                            } else {
                                Err(Diagnostic::new(
                                    DiagCode::E600,
                                    expr.span,
                                    "integer underflow",
                                ))
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
                    return Ok(Value::Bool(
                        l.is_truthy() && self.eval_expr(right, env)?.is_truthy(),
                    ));
                }
                if *op == BinOp::Or {
                    return Ok(Value::Bool(
                        l.is_truthy() || self.eval_expr(right, env)?.is_truthy(),
                    ));
                }
                let r = self.eval_expr(right, env)?;
                binop(*op, &l, &r, expr.span)
            }
            ExprKind::Member { recv, name } => {
                // T9: Check if this is a unit variant of a user-defined enum
                // BEFORE evaluating the receiver (which would fail as undefined
                // ident). `EnumName.Variant` (no call) â†’ unit enum value.
                if let ExprKind::Ident(enum_name) = &recv.kind {
                    if let Some(enum_decl) = env.enums.get(enum_name) {
                        if enum_decl
                            .variants
                            .iter()
                            .any(|v| v.name == *name && v.payload.is_empty())
                        {
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
                self.budget
                    .charge(24 + out.len() as u64 * 16, expr.span)?;
                Ok(Value::List(out))
            }
            ExprKind::Record(fs) => {
                let mut m = std::collections::BTreeMap::new();
                for f in fs {
                    m.insert(f.name.clone(), self.eval_expr(&f.value, env)?);
                }
                self.budget
                    .charge(24 + m.len() as u64 * 32, expr.span)?;
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
            ExprKind::Pipe { left, right } => {
                let left_val = self.eval_expr(left, env)?;
                match &right.kind {
                    ExprKind::Call { callee, args } => {
                        self.eval_call_with_first_val(callee, left_val, args, expr.span, env)
                    }
                    _ => {
                        self.eval_call_with_first_val(right, left_val, &[], expr.span, env)
                    }
                }
            }
            ExprKind::GraphQuery { is_ask, pattern, variables } => {
                let mut rec = std::collections::BTreeMap::new();
                rec.insert("query".to_string(), Value::String(pattern.clone()));
                rec.insert("is_ask".to_string(), Value::Bool(*is_ask));
                rec.insert(
                    "variables".to_string(),
                    Value::List(
                        variables
                            .iter()
                            .map(|v| Value::String(v.clone()))
                            .collect(),
                    ),
                );
                let payload = Value::Record(rec);
                crate::bind::dispatch(
                    self.host,
                    "GraphDatabase.sparql",
                    &[payload.clone()],
                    &[("query".to_string(), Value::String(pattern.clone()))],
                    expr.span,
                )
            }
            ExprKind::ModalLogic { modality, args, body } => {
                let mut evaluated_args = Vec::new();
                for a in args {
                    evaluated_args.push(self.eval_expr(a, env)?);
                }
                let body_val = if let Some(b) = body {
                    self.eval_expr(b, env)?
                } else {
                    Value::Null
                };
                let modal_name = match modality {
                    ModalKind::DeonticObligate => "obligate",
                    ModalKind::DeonticPermit => "permit",
                    ModalKind::DeonticForbid => "forbid",
                    ModalKind::EpistemicKnows => "knows",
                    ModalKind::EpistemicBelieves => "believes",
                    ModalKind::Paraconsistent => "paraconsistent",
                    ModalKind::LtlGlobally => "always",
                    ModalKind::LtlFinally => "eventually",
                    ModalKind::LtlUntil => "until",
                    ModalKind::DlSubsumes => "subsumes",
                    ModalKind::N3Defeasible => "defeasible_rule",
                };
                let mut rec = std::collections::BTreeMap::new();
                rec.insert("modality".to_string(), Value::String(modal_name.to_string()));
                rec.insert("args".to_string(), Value::List(evaluated_args));
                rec.insert("body".to_string(), body_val);
                rec.insert("kind".to_string(), Value::String("term".to_string()));
                rec.insert("evaluated".to_string(), Value::Bool(false));
                let term = Value::Record(rec);
                let engine_id = match modality {
                    ModalKind::DeonticObligate
                    | ModalKind::DeonticPermit
                    | ModalKind::DeonticForbid => "DeonticLogic.evaluate",
                    ModalKind::EpistemicKnows | ModalKind::EpistemicBelieves => {
                        "EpistemicLogic.evaluate"
                    }
                    ModalKind::Paraconsistent => "ParaconsistentLogic.route",
                    ModalKind::LtlGlobally => "TemporalAndDescriptionLogic.ltl.globally",
                    ModalKind::LtlFinally => "TemporalAndDescriptionLogic.ltl.finally",
                    ModalKind::DlSubsumes => "TemporalAndDescriptionLogic.subsumption",
                    ModalKind::LtlUntil | ModalKind::N3Defeasible => {
                        return Ok(term);
                    }
                };
                let granted = self.granted_refs();
                if !crate::catalog::granted_covers(&granted, engine_id) {
                    return Ok(term);
                }
                match dispatch(
                    self.host,
                    engine_id,
                    std::slice::from_ref(&term),
                    &[],
                    expr.span,
                ) {
                    Ok(Value::Record(mut engine)) => {
                        engine
                            .entry("kind".to_string())
                            .or_insert_with(|| Value::String("term".into()));
                        engine.insert("source".into(), term);
                        Ok(Value::Record(engine))
                    }
                    Ok(other) => Ok(other),
                    Err(e)
                        if matches!(e.code, DiagCode::E300 | DiagCode::E100 | DiagCode::E702) =>
                    {
                        Ok(term)
                    }
                    Err(e) => Err(e),
                }
            }
            ExprKind::Interpolate(parts) => {
                let mut buf = String::new();
                for p in parts {
                    let v = self.eval_expr(p, env)?;
                    match v {
                        Value::String(s) => buf.push_str(&s),
                        Value::I64(i) => buf.push_str(&i.to_string()),
                        Value::U64(u) => buf.push_str(&u.to_string()),
                        Value::F64(f) => buf.push_str(&f.to_string()),
                        Value::Bool(b) => buf.push_str(&b.to_string()),
                        Value::Null => buf.push_str("null"),
                        Value::Quantity(q) => {
                            use std::fmt::Write;
                            let _ = write!(buf, "{}{}", q.value, q.unit);
                        }
                        other => buf.push_str(&format!("{other:?}")),
                    }
                }
                self.budget.charge(buf.len() as u64, expr.span)?;
                Ok(Value::String(buf))
            }
            ExprKind::Await(e) => self.eval_expr(e, env),
            ExprKind::Lambda { params, body } => {
                check_non_capturing(body, params, env, expr.span)?;
                Ok(Value::Lambda {
                    params: params.clone(),
                    body: body.clone(),
                })
            }
            ExprKind::Tween {
                from,
                to,
                over,
                ease,
                spring,
            } => self.eval_tween(from, to, over, ease.as_deref(), spring.as_deref(), env, expr.span),
        }
    }

    fn eval_tween(
        &mut self,
        from: &Expr,
        to: &Expr,
        over: &Expr,
        ease: Option<&str>,
        spring: Option<&[NamedArg]>,
        env: &mut Env,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let from_v = self.eval_expr(from, env)?;
        let to_v = self.eval_expr(to, env)?;
        let over_v = self.eval_expr(over, env)?;
        let a = numeric(&from_v, span)?;
        let b = numeric(&to_v, span)?;
        let dur_s = duration_seconds(&over_v, span)?;
        let elapsed = env
            .vars
            .get("t")
            .map(|v| elapsed_seconds(v))
            .flatten()
            .unwrap_or(dur_s);
        let progress = if dur_s <= 0.0 {
            1.0
        } else {
            (elapsed / dur_s).clamp(0.0, 1.0)
        };
        let t = if let Some(args) = spring {
            let mut stiff = 280.0;
            let mut damp = 22.0;
            for na in args {
                let v = self.eval_expr(&na.value, env)?;
                if let Some(n) = v.as_f64() {
                    match na.name.as_str() {
                        "stiffness" => stiff = n,
                        "damping" => damp = n,
                        _ => {}
                    }
                }
            }
            let cfg = crate::animation::spring::SpringConfig::new(stiff, damp);
            let state = crate::animation::spring::SpringState1D::new(a, 0.0, b);
            state.evaluate_at(&cfg, elapsed).0
        } else {
            let name = ease.unwrap_or("linear");
            let curve = crate::animation::curves::EasingCurve::from_name(name).ok_or_else(|| {
                let hint = crate::catalog::did_you_mean(&format!("ease.{name}"))
                    .map(|s| format!("; did you mean `{s}`?"))
                    .unwrap_or_default();
                Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("unknown ease `{name}`{hint}"),
                )
            })?;
            let u = curve.eval(progress);
            a + (b - a) * u
        };
        match (&from_v, &to_v) {
            (Value::Quantity(qa), Value::Quantity(qb)) if qa.unit == qb.unit => {
                Ok(Value::Quantity(crate::value::Quantity {
                    value: t,
                    unit: qa.unit.clone(),
                }))
            }
            _ => Ok(Value::F64(t)),
        }
    }
}

fn numeric(v: &Value, span: Span) -> Result<f64, Diagnostic> {
    v.as_f64().ok_or_else(|| {
        Diagnostic::new(DiagCode::E600, span, "tween operand is not numeric")
    })
}

fn duration_seconds(v: &Value, span: Span) -> Result<f64, Diagnostic> {
    match v {
        Value::Duration(d) => Ok(d.secs as f64 + d.nanos as f64 / 1e9),
        Value::Quantity(q) => {
            if let Some(u) = crate::quantity::lookup_unit(&q.unit) {
                if let Some(s) = crate::quantity::lookup_unit("s") {
                    return u.convert(q.value, &s).map_err(|e| {
                        Diagnostic::new(DiagCode::E100, span, e)
                    });
                }
            }
            match q.unit.as_str() {
                "ms" => Ok(q.value / 1000.0),
                "s" => Ok(q.value),
                _ => Ok(q.value),
            }
        }
        Value::F64(n) => Ok(*n),
        Value::I64(n) => Ok(*n as f64),
        Value::U64(n) => Ok(*n as f64),
        _ => Err(Diagnostic::new(
            DiagCode::E600,
            span,
            "tween duration must be a duration or quantity",
        )),
    }
}

fn elapsed_seconds(v: &Value) -> Option<f64> {
    match v {
        Value::Instant(i) => Some(i.secs as f64 + i.nanos as f64 / 1e9),
        Value::Duration(d) => Some(d.secs as f64 + d.nanos as f64 / 1e9),
        Value::Quantity(q) if q.unit == "ms" => Some(q.value / 1000.0),
        Value::Quantity(q) if q.unit == "s" => Some(q.value),
        Value::F64(n) => Some(*n),
        Value::I64(n) => Some(*n as f64),
        Value::U64(n) => Some(*n as f64),
        _ => None,
    }
}

fn check_non_capturing(
    body: &Expr,
    params: &[String],
    env: &Env,
    span: Span,
) -> Result<(), Diagnostic> {
    let mut free = Vec::new();
    collect_free_idents(body, params, env, &mut free);
    if let Some(name) = free.first() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("lambda captures `{name}`; vibe-0.1 lambdas are non-capturing"),
        ));
    }
    Ok(())
}

fn collect_free_idents(expr: &Expr, params: &[String], env: &Env, out: &mut Vec<String>) {
    match &expr.kind {
        ExprKind::Ident(n) => {
            if params.iter().any(|p| p == n) {
                return;
            }
            if env.aliases.contains_key(n)
                || env.enums.contains_key(n)
                || env.vars.contains_key(n)
                    && matches!(
                        n.as_str(),
                        "math" | "true" | "false" | "null" | "Ok" | "Err" | "vec2" | "vec3" | "vec4"
                    )
            {
                return;
            }
            const ALLOWED: &[&str] = &[
                "math", "rdf", "quin", "graph", "Ok", "Err", "vec2", "vec3", "vec4", "mat3",
                "mat4", "oklch",
            ];
            if ALLOWED.contains(&n.as_str()) {
                return;
            }
            if crate::catalog::family_of(n).is_some()
                || crate::catalog::families().iter().any(|f| *f == n)
            {
                return;
            }
            if !out.contains(n) {
                out.push(n.clone());
            }
        }
        ExprKind::Member { recv, .. } => collect_free_idents(recv, params, env, out),
        ExprKind::Binary { left, right, .. } | ExprKind::Pipe { left, right } => {
            collect_free_idents(left, params, env, out);
            collect_free_idents(right, params, env, out);
        }
        ExprKind::Unary { expr, .. } | ExprKind::Await(expr) | ExprKind::Try(expr) => {
            collect_free_idents(expr, params, env, out);
        }
        ExprKind::Call { callee, args } => {
            collect_free_idents(callee, params, env, out);
            for a in args {
                match a {
                    Arg::Pos(e) | Arg::Named(NamedArg { value: e, .. }) => {
                        collect_free_idents(e, params, env, out);
                    }
                }
            }
        }
        ExprKind::Index { recv, index } => {
            collect_free_idents(recv, params, env, out);
            collect_free_idents(index, params, env, out);
        }
        ExprKind::List(xs) => {
            for x in xs {
                collect_free_idents(x, params, env, out);
            }
        }
        ExprKind::Record(fs) => {
            for f in fs {
                collect_free_idents(&f.value, params, env, out);
            }
        }
        ExprKind::Lambda { params: inner, body } => {
            let mut nested = params.to_vec();
            nested.extend(inner.iter().cloned());
            collect_free_idents(body, &nested, env, out);
        }
        ExprKind::Tween {
            from, to, over, ..
        } => {
            collect_free_idents(from, params, env, out);
            collect_free_idents(to, params, env, out);
            collect_free_idents(over, params, env, out);
        }
        _ => {}
    }
}
