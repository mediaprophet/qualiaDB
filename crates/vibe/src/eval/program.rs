use super::*;

impl<'a, H: Host> Engine<'a, H> {
    pub fn eval_program(&mut self, program: &Program, env: &mut Env) -> Result<Value, Diagnostic> {
        self.granted = program.requires.iter().map(|c| c.id.clone()).collect();
        let mut last = Value::Null;
        let has_cells = program.items.iter().any(|i| matches!(i, Item::Cell(_)));
        let mut cell_graph = if has_cells {
            Some(
                crate::reactive_cell::ReactiveCellGraph::from_program(program)
                    .map_err(|e| Diagnostic::new(DiagCode::E600, program.span, e.to_string()))?,
            )
        } else {
            None
        };
        let mut cells_stepped = false;
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
                Item::Field(f) => {
                    let mut rec = std::collections::BTreeMap::new();
                    rec.insert("name".to_string(), Value::String(f.name.clone()));
                    rec.insert("ty".to_string(), Value::String(f.ty.name.clone()));
                    if let Some(u) = &f.unit {
                        rec.insert("unit".to_string(), Value::String(u.clone()));
                    }
                    rec.insert(
                        "support".to_string(),
                        Value::String(format!("{:?}", f.support).to_lowercase()),
                    );
                    rec.insert(
                        "representation".to_string(),
                        Value::String(format!("{:?}", f.representation).to_lowercase()),
                    );
                    env.fields.insert(f.name.clone(), Value::Record(rec));
                }
                Item::Material(m) => {
                    let mut rec = std::collections::BTreeMap::new();
                    rec.insert("name".to_string(), Value::String(m.name.clone()));
                    for prop in &m.properties {
                        let v = self.eval_expr(&prop.value, env)?;
                        rec.insert(prop.name.clone(), v);
                    }
                    env.materials.insert(m.name.clone(), Value::Record(rec));
                }
                Item::Present(p) => {
                    let mut sheaf =
                        crate::presentation::PresentationSheaf::new(Value::String(p.name.clone()));
                    let mut css = std::collections::BTreeMap::new();
                    for prop in &p.properties {
                        let v = self.eval_expr(&prop.value, env)?;
                        match prop.name.as_str() {
                            "speech" | "announce" | "auditory" => {
                                let text = match &v {
                                    Value::String(s) => s.clone(),
                                    other => format!("{other}"),
                                };
                                sheaf.add(crate::presentation::Presentation::speech(&text));
                            }
                            "braille" => {
                                let cells: Vec<u8> = match &v {
                                    Value::String(s) => s.bytes().map(|b| b & 0x3F).collect(),
                                    Value::List(xs) => xs
                                        .iter()
                                        .filter_map(|x| x.as_i64().map(|n| n as u8))
                                        .collect(),
                                    _ => Vec::new(),
                                };
                                sheaf.add(crate::presentation::Presentation::braille_cells(cells));
                            }
                            "haptic" => {
                                sheaf.add(crate::presentation::Presentation::haptic_pattern(vec![
                                    (100, 0.5),
                                ]));
                            }
                            "svg" => {
                                let s = match &v {
                                    Value::String(s) => s.clone(),
                                    other => format!("{other}"),
                                };
                                sheaf.add(crate::presentation::Presentation::svg(&s));
                            }
                            _ => {
                                css.insert(prop.name.clone(), v);
                            }
                        }
                    }
                    if !css.is_empty() {
                        sheaf.add(crate::presentation::Presentation::css(css));
                    }
                    let rec = sheaf.to_value();
                    env.vars.insert(p.name.clone(), rec.clone());
                    last = rec;
                }
                Item::Cell(c) => {
                    if let Some(g) = cell_graph.as_mut() {
                        if !cells_stepped {
                            let updated = g.step_with(self, env, 0.0).map_err(|e| {
                                Diagnostic::new(DiagCode::E600, c.span, e.to_string())
                            })?;
                            cells_stepped = true;
                            if let Some((_, v)) = updated.last() {
                                last = v.clone();
                            }
                        } else if let Some(v) = g.get_value(&c.name) {
                            last = v.clone();
                        }
                    }
                }
                Item::Bind(b) => {
                    self.apply_bind(b, env)?;
                }
                Item::Law(l) => {
                    let mut rec = std::collections::BTreeMap::new();
                    rec.insert("name".to_string(), Value::String(l.name.clone()));
                    rec.insert("has_condition".to_string(), Value::Bool(true));
                    rec.insert("has_consequence".to_string(), Value::Bool(true));
                    env.laws.insert(l.name.clone(), Value::Record(rec));
                }
            }
        }
        Ok(last)
    }

    fn apply_bind(&mut self, b: &BindDecl, env: &mut Env) -> Result<(), Diagnostic> {
        let left_v = self.eval_expr(&b.left, env)?;
        let right_v = self.eval_expr(&b.right, env)?;
        let mut chosen = match b.resolve {
            BindResolve::Left => left_v,
            BindResolve::Right => right_v.clone(),
            BindResolve::Latest => {
                if !matches!(right_v, Value::Null) {
                    right_v.clone()
                } else {
                    left_v
                }
            }
        };
        if let Some((lo, hi)) = &b.clamp {
            let lo_v = self.eval_expr(lo, env)?;
            let hi_v = self.eval_expr(hi, env)?;
            if let (Some(x), Some(lo_n), Some(hi_n)) =
                (chosen.as_f64(), lo_v.as_f64(), hi_v.as_f64())
            {
                let c = x.clamp(lo_n, hi_n);
                chosen = match &chosen {
                    Value::Quantity(q) => Value::Quantity(crate::value::Quantity {
                        value: c,
                        unit: q.unit.clone(),
                    }),
                    _ => Value::F64(c),
                };
            }
        }
        assign_path(env, &b.left, chosen.clone())?;
        assign_path(env, &b.right, chosen)?;
        Ok(())
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
            Diagnostic::new(
                DiagCode::E600,
                Span::point(0),
                format!("no function {name}"),
            )
        })?;
        if let Some(steps) = budget_steps(&f.budget, env, self) {
            self.budget.steps_left = self.budget.steps_left.min(steps);
        }
        let mut local = Env::default();
        // Inherit import aliases, enum declarations, and field/material/law
        // declarations from the calling env.
        local.aliases = env.aliases.clone();
        local.enums = env.enums.clone();
        local.fields = env.fields.clone();
        local.materials = env.materials.clone();
        local.laws = env.laws.clone();
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
    /// Finds the first `on <path>(â€¦)` hook in the program whose path matches,
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

fn assign_path(env: &mut Env, target: &Expr, value: Value) -> Result<(), Diagnostic> {
    match &target.kind {
        ExprKind::Ident(n) => {
            env.vars.insert(n.clone(), value);
            Ok(())
        }
        ExprKind::Member { recv, name } => {
            if let ExprKind::Ident(root) = &recv.kind {
                match env.vars.get_mut(root) {
                    Some(Value::Record(map)) => {
                        map.insert(name.clone(), value);
                        Ok(())
                    }
                    Some(_) => Err(Diagnostic::new(
                        DiagCode::E600,
                        target.span,
                        format!("cannot bind into non-record `{root}`"),
                    )),
                    None => {
                        let mut map = std::collections::BTreeMap::new();
                        map.insert(name.clone(), value);
                        env.vars.insert(root.clone(), Value::Record(map));
                        Ok(())
                    }
                }
            } else {
                Err(Diagnostic::new(
                    DiagCode::E100,
                    target.span,
                    "bind target must be an identifier or record.field",
                ))
            }
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            target.span,
            "bind target must be an identifier or record.field",
        )),
    }
}
