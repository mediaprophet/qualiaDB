use super::*;
use crate::bind::dispatch;
use crate::value::EnumValue;

impl<'a, H: Host> Engine<'a, H> {
    pub(crate) fn eval_call_with_first_val(
        &mut self,
        callee: &Expr,
        first: Value,
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
        let mut pos = vec![first];
        let mut named = Vec::new();
        for a in args {
            match a {
                Arg::Pos(e) => pos.push(self.eval_expr(e, env)?),
                Arg::Named(n) => named.push((n.name.clone(), self.eval_expr(&n.value, env)?)),
            }
        }
        self.dispatch_call_path(&path, pos, named, span, env)
    }

    pub(crate) fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Arg],
        span: Span,
        env: &mut Env,
    ) -> Result<Value, Diagnostic> {
        if self.depth >= 64 {
            return Err(Diagnostic::new(DiagCode::E400, span, "call depth exceeded"));
        }
        if let ExprKind::Member { recv, name } = &callee.kind {
            if matches!(name.as_str(), "map" | "filter") {
                return self.eval_collection_method(name, recv, args, span, env);
            }
        }
        let path = match &callee.kind {
            ExprKind::Member { recv, name } => {
                if let ExprKind::Ident(ns) = &recv.kind {
                    if env.vars.contains_key(ns)
                        && matches!(name.as_str(), "map" | "filter")
                    {
                        return self.eval_collection_method(name, recv, args, span, env);
                    }
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
        self.dispatch_call_path(&path, pos, named, span, env)
    }

    pub(crate) fn dispatch_call_path(
        &mut self,
        path: &str,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
        env: &mut Env,
    ) -> Result<Value, Diagnostic> {
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
        if path == "vec2" || path == "vec3" || path == "vec4" || path == "mat3" || path == "mat4" {
            return Ok(Value::List(pos));
        }
        if path == "oklch" {
            let l = pos.first().and_then(Value::as_f64).unwrap_or(0.0);
            let c = pos.get(1).and_then(Value::as_f64).unwrap_or(0.0);
            let h = pos.get(2).and_then(Value::as_f64).unwrap_or(0.0);
            let a = pos.get(3).and_then(Value::as_f64).unwrap_or(1.0);
            let mut rec = std::collections::BTreeMap::new();
            rec.insert("space".into(), Value::String("oklch".into()));
            rec.insert("l".into(), Value::F64(l));
            rec.insert("c".into(), Value::F64(c));
            rec.insert("h".into(), Value::F64(h));
            rec.insert("a".into(), Value::F64(a));
            return Ok(Value::Record(rec));
        }
        // T9: User-defined enum variant construction.
        // `EnumName.Variant(args)` â†’ `Value::Enum(EnumValue { ... })`
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
            if let Some(Item::Function(f)) = program
                .items
                .iter()
                .find(|i| matches!(i, Item::Function(f) if f.name == path))
            {
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
        if let Err(diag) = self.check_phase_capability(path, span) {
            self.depth -= 1;
            return Err(diag);
        }
        let r = dispatch(self.host, path, &pos, &named, span);
        self.depth -= 1;
        r
    }

    fn eval_collection_method(
        &mut self,
        name: &str,
        recv: &Expr,
        args: &[Arg],
        span: Span,
        env: &mut Env,
    ) -> Result<Value, Diagnostic> {
        let recv_val = self.eval_expr(recv, env)?;
        let Value::List(items) = recv_val else {
            return Err(Diagnostic::new(
                DiagCode::E600,
                span,
                format!(".{name} requires a list receiver"),
            ));
        };
        let lambda = match args.first() {
            Some(Arg::Pos(e)) => self.eval_expr(e, env)?,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!(".{name} requires a lambda argument"),
                ));
            }
        };
        let Value::Lambda { params, body } = lambda else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!(".{name} requires a non-capturing lambda"),
            ));
        };
        let mut out = Vec::new();
        for item in items {
            let v = self.apply_lambda(&params, &body, item.clone(), env, span)?;
            if name == "filter" {
                if v.is_truthy() {
                    out.push(item);
                }
            } else {
                out.push(v);
            }
        }
        self.budget.charge(24 + out.len() as u64 * 16, span)?;
        Ok(Value::List(out))
    }

    fn apply_lambda(
        &mut self,
        params: &[String],
        body: &Expr,
        arg: Value,
        env: &Env,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let mut local = Env::default();
        local.aliases = env.aliases.clone();
        local.enums = env.enums.clone();
        if let Some(p) = params.first() {
            local.vars.insert(p.clone(), arg);
        }
        if params.len() > 1 {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "list.map/filter lambdas take one parameter in vibe-0.1",
            ));
        }
        self.depth += 1;
        if self.depth >= 64 {
            self.depth -= 1;
            return Err(Diagnostic::new(DiagCode::E400, span, "call depth exceeded"));
        }
        let r = self.eval_expr(body, &mut local);
        self.depth -= 1;
        r
    }
}
