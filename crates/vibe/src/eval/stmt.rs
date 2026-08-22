use super::*;

impl<'a, H: Host> Engine<'a, H> {
    pub(crate) fn eval_block(&mut self, block: &Block, env: &mut Env) -> Result<Flow, Diagnostic> {
        let mut last = Value::Null;
        for s in &block.stmts {
            match self.eval_stmt(s, env)? {
                Flow::Return(v) => return Ok(Flow::Return(v)),
                Flow::Next(v) => last = v,
            }
        }
        Ok(Flow::Next(last))
    }

    pub(crate) fn finish_block(&mut self, block: &Block, env: &mut Env) -> Result<Value, Diagnostic> {
        match self.eval_block(block, env)? {
            Flow::Return(v) | Flow::Next(v) => Ok(v),
        }
    }

    pub(crate) fn eval_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> Result<Flow, Diagnostic> {
        self.budget.tick(stmt.span())?;
        match stmt {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                env.vars.insert(name.clone(), v);
                if *mutable {
                    env.mutables.insert(name.clone());
                } else {
                    env.mutables.remove(name);
                }
                Ok(Flow::Next(Value::Null))
            }
            Stmt::LetPat {
                pattern,
                value,
                mutable,
                ..
            } => {
                let v = self.eval_expr(value, env)?;
                if !match_pat(pattern, &v, env) {
                    return Err(Diagnostic::new(
                        DiagCode::E600,
                        stmt.span(),
                        "destructuring let pattern failed to match",
                    ));
                }
                if *mutable {
                    let mut names = Vec::new();
                    collect_pat_idents(pattern, &mut names);
                    for n in names {
                        env.mutables.insert(n);
                    }
                }
                Ok(Flow::Next(Value::Null))
            }
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let v = self.eval_expr(value, env)?;
                if let Some(n) = target.ident_name() {
                    if !env.mutables.contains(n) {
                        return Err(Diagnostic::new(
                            DiagCode::E701,
                            *span,
                            format!(
                                "cannot assign to immutable binding `{n}` (declare with `let mut`)"
                            ),
                        ));
                    }
                    env.vars.insert(n.to_string(), v.clone());
                }
                Ok(Flow::Next(v))
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                if self.eval_expr(cond, env)?.is_truthy() {
                    self.eval_block(then_block, env)
                } else if let Some(e) = else_block {
                    self.eval_stmt(e, env)
                } else {
                    Ok(Flow::Next(Value::Null))
                }
            }
            Stmt::For {
                name, iter, body, ..
            } => {
                let list = self.eval_expr(iter, env)?;
                let xs = match list {
                    Value::List(xs) => xs,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagCode::E600,
                            stmt.span(),
                            "for-loop needs a list",
                        ))
                    }
                };
                let mut last = Value::Null;
                for x in xs {
                    env.vars.insert(name.clone(), x);
                    match self.eval_block(body, env)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next(v) => last = v,
                    }
                }
                Ok(Flow::Next(last))
            }
            Stmt::While { cond, body, .. } => {
                let mut last = Value::Null;
                while self.eval_expr(cond, env)?.is_truthy() {
                    match self.eval_block(body, env)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next(v) => last = v,
                    }
                }
                Ok(Flow::Next(last))
            }
            Stmt::Return { value, .. } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                Ok(Flow::Return(v))
            }
            Stmt::Transaction { body, span, .. } => {
                self.host.graph_begin(*span)?;
                match self.eval_block(body, env) {
                    Ok(flow) => {
                        let _ = self.host.graph_abort(*span);
                        Ok(flow)
                    }
                    Err(err) => {
                        let _ = self.host.graph_abort(*span);
                        Err(err)
                    }
                }
            }
            Stmt::Block(body) => self.eval_block(body, env),
            Stmt::Effect { expr, .. } | Stmt::Expr { expr, .. } => {
                Ok(Flow::Next(self.eval_expr(expr, env)?))
            }
            Stmt::Yield { value, .. } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                Ok(Flow::Next(v))
            }
            Stmt::Match {
                scrutinee, arms, ..
            } => {
                let v = self.eval_expr(scrutinee, env)?;
                for arm in arms {
                    if match_pat(&arm.pattern, &v, env) {
                        return match &arm.body {
                            ArmBody::Block(b) => self.eval_block(b, env),
                            ArmBody::Expr(e) => Ok(Flow::Next(self.eval_expr(e, env)?)),
                        };
                    }
                }
                Ok(Flow::Next(Value::Null))
            }
        }
    }
}
