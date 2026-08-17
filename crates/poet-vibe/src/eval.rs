//! AST interpreter. No JIT.

use crate::ast::*;
use crate::bind::{dispatch, Host};
use crate::budget::Budget;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;
use std::collections::HashMap;

pub struct Env {
    pub vars: HashMap<String, Value>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

pub struct Engine<'a, H: Host> {
    host: &'a mut H,
    budget: Budget,
    depth: u32,
}

enum Flow {
    Next(Value),
    Return(Value),
}

impl<'a, H: Host> Engine<'a, H> {
    pub fn new(host: &'a mut H, budget: Budget) -> Self {
        Self {
            host,
            budget,
            depth: 0,
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr, env: &mut Env) -> Result<Value, Diagnostic> {
        self.budget.tick(expr.span)?;
        match &expr.kind {
            ExprKind::Literal(l) => Ok(lit(l)),
            ExprKind::Ident(n) => {
                if n == "Ok" || n == "Err" || n == "Some" || n == "None" {
                    return Ok(Value::Identish(n.clone()));
                }
                env.vars.get(n).cloned().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, expr.span, format!("undefined {n}"))
                })
            }
            ExprKind::QueryVar(n) => Ok(Value::Var(n.clone())),
            ExprKind::Iri(s) => Ok(Value::Iri(s.clone())),
            ExprKind::Prefixed(p, l) => Ok(Value::Prefixed(p.clone(), l.clone())),
            ExprKind::Blank(n) => Ok(Value::Blank(n.clone())),
            ExprKind::Unary { op, expr } => {
                let v = self.eval_expr(expr, env)?;
                match op {
                    UnOp::Not => Ok(Value::Bool(!v.is_truthy())),
                    UnOp::Neg => v
                        .as_f64()
                        .map(|n| Value::F64(-n))
                        .ok_or_else(|| Diagnostic::new(DiagCode::E600, expr.span, "negation")),
                    UnOp::Plus => Ok(v),
                }
            }
            ExprKind::Binary { op, left, right } => {
                let l = self.eval_expr(left, env)?;
                if *op == BinOp::And {
                    return Ok(Value::Bool(l.is_truthy() && self.eval_expr(right, env)?.is_truthy()));
                }
                if *op == BinOp::Or {
                    return Ok(Value::Bool(l.is_truthy() || self.eval_expr(right, env)?.is_truthy()));
                }
                let r = self.eval_expr(right, env)?;
                binop(*op, &l, &r, expr.span)
            }
            ExprKind::Member { recv, name } => {
                let _ = self.eval_expr(recv, env)?;
                // namespace keep: math.max is Call of Member, eval'd in Call
                Ok(Value::Identish(name.clone()))
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
                Ok(Value::List(out))
            }
            ExprKind::Record(fs) => {
                let mut m = std::collections::BTreeMap::new();
                for f in fs {
                    m.insert(f.name.clone(), self.eval_expr(&f.value, env)?);
                }
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
            ExprKind::Await(e) => self.eval_expr(e, env),
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
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
                    format!("{ns}.{name}")
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
        self.depth += 1;
        let r = dispatch(self.host, &path, &pos, &named, span);
        self.depth -= 1;
        r
    }

    fn eval_block(&mut self, block: &Block, env: &mut Env) -> Result<Flow, Diagnostic> {
        let mut last = Value::Null;
        for s in &block.stmts {
            match self.eval_stmt(s, env)? {
                Flow::Return(v) => return Ok(Flow::Return(v)),
                Flow::Next(v) => last = v,
            }
        }
        Ok(Flow::Next(last))
    }

    fn finish_block(&mut self, block: &Block, env: &mut Env) -> Result<Value, Diagnostic> {
        match self.eval_block(block, env)? {
            Flow::Return(v) | Flow::Next(v) => Ok(v),
        }
    }

    fn eval_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> Result<Flow, Diagnostic> {
        self.budget.tick(stmt.span())?;
        match stmt {
            Stmt::Let { name, value, .. } => {
                let v = if let Some(e) = value {
                    self.eval_expr(e, env)?
                } else {
                    Value::Null
                };
                env.vars.insert(name.clone(), v);
                Ok(Flow::Next(Value::Null))
            }
            Stmt::Assign { target, value, .. } => {
                let v = self.eval_expr(value, env)?;
                if let Some(n) = target.ident_name() {
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
            Stmt::For { name, iter, body, .. } => {
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
            Stmt::Match { scrutinee, arms, .. } => {
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

    pub fn eval_program(&mut self, program: &Program, env: &mut Env) -> Result<Value, Diagnostic> {
        let mut last = Value::Null;
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
            }
        }
        Ok(last)
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
            Diagnostic::new(DiagCode::E600, Span::point(0), format!("no function {name}"))
        })?;
        if let Some(steps) = budget_steps(&f.budget, env, self) {
            self.budget.steps_left = self.budget.steps_left.min(steps);
        }
        let mut local = Env::default();
        for (p, a) in f.params.iter().zip(args.into_iter()) {
            local.vars.insert(p.name.clone(), a);
        }
        self.finish_block(&f.body, &mut local)
    }
}

fn lit(l: &Literal) -> Value {
    match l {
        Literal::Null => Value::Null,
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Int(n) => Value::I64(*n),
        Literal::UInt(n) => Value::U64(*n),
        Literal::Float(bits) => Value::F64(f64::from_bits(*bits)),
        Literal::String(s) => Value::String(s.clone()),
    }
}

fn binop(op: BinOp, l: &Value, r: &Value, span: Span) -> Result<Value, Diagnostic> {
    if matches!(op, BinOp::Eq | BinOp::Ne) {
        let eq = l == r;
        return Ok(Value::Bool(if op == BinOp::Eq { eq } else { !eq }));
    }
    let a = l.as_f64().ok_or_else(|| {
        Diagnostic::new(DiagCode::E600, span, "numeric operator on non-number")
    })?;
    let b = r.as_f64().ok_or_else(|| {
        Diagnostic::new(DiagCode::E600, span, "numeric operator on non-number")
    })?;
    if matches!(op, BinOp::Div | BinOp::Rem) && b == 0.0 {
        return Err(Diagnostic::new(DiagCode::E600, span, "division by zero"));
    }
    let n = match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        BinOp::Rem => a % b,
        BinOp::Lt => return Ok(Value::Bool(a < b)),
        BinOp::Le => return Ok(Value::Bool(a <= b)),
        BinOp::Gt => return Ok(Value::Bool(a > b)),
        BinOp::Ge => return Ok(Value::Bool(a >= b)),
        _ => a,
    };
    if matches!(l, Value::I64(_)) && matches!(r, Value::I64(_)) && n.fract() == 0.0 {
        Ok(Value::I64(n as i64))
    } else {
        Ok(Value::F64(n))
    }
}

fn match_pat(p: &Pattern, v: &Value, env: &mut Env) -> bool {
    match p {
        Pattern::Wildcard => true,
        Pattern::Ident(n) => {
            env.vars.insert(n.clone(), v.clone());
            true
        }
        Pattern::Literal(l) => lit(l) == *v,
        Pattern::None => matches!(v, Value::Null),
        Pattern::Ok(inner) => match v {
            Value::Ok(x) => match_pat(inner, x, env),
            _ => false,
        },
        Pattern::Err(inner) => match v {
            Value::Err(x) => match_pat(inner, x, env),
            _ => false,
        },
        Pattern::Some(inner) => match_pat(inner, v, env),
    }
}

fn budget_steps<H: Host>(args: &[NamedArg], env: &mut Env, eng: &mut Engine<H>) -> Option<u64> {
    for a in args {
        if a.name == "steps" {
            return eng.eval_expr(&a.value, env).ok().and_then(|v| v.as_i64()).map(|n| n as u64);
        }
    }
    None
}
