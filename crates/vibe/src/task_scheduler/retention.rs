//! Nonallocating preflight before retained-value Debug formatting or cloning.
use super::registry::error;
use crate::{
    ast::{Arg, Expr, ExprKind},
    error::Diagnostic,
    span::Span,
    value::Value,
};

struct Work {
    remaining: usize,
    span: Span,
}
impl Work {
    fn take(&mut self, count: usize, depth: usize) -> Result<(), Diagnostic> {
        if depth > 32 {
            return Err(error(self.span, "task result nesting exceeds 32"));
        }
        self.remaining = self.remaining.checked_sub(count).ok_or_else(|| {
            error(
                self.span,
                "task result traversal exceeds 16384 nodes/elements",
            )
        })?;
        Ok(())
    }
    fn value(&mut self, value: &Value, depth: usize) -> Result<(), Diagnostic> {
        self.take(1, depth)?;
        match value {
            Value::List(items) => {
                for item in items {
                    self.value(item, depth + 1)?;
                }
            }
            Value::Record(items) => {
                for item in items.values() {
                    self.value(item, depth + 1)?;
                }
            }
            Value::Triple(s, p, o) => {
                self.value(s, depth + 1)?;
                self.value(p, depth + 1)?;
                self.value(o, depth + 1)?;
            }
            Value::Reified { s, p, o, r } => {
                self.value(s, depth + 1)?;
                self.value(p, depth + 1)?;
                self.value(o, depth + 1)?;
                self.value(r, depth + 1)?;
            }
            Value::Ok(item) | Value::Err(item) => self.value(item, depth + 1)?,
            Value::Enum(item) => {
                for value in &item.payload {
                    self.value(value, depth + 1)?;
                }
            }
            Value::Lambda { params, body } => {
                self.take(params.len(), depth)?;
                self.expression(body, depth + 1)?;
            }
            Value::Frame(frame) => {
                self.take(frame.origin.len(), depth)?;
                self.take(frame.basis.len(), depth)?;
                for row in &frame.basis {
                    self.take(row.len(), depth)?;
                }
            }
            Value::Pose(pose) => {
                self.take(pose.position.len(), depth)?;
                self.take(pose.orientation.len(), depth)?;
            }
            Value::Transform(transform) => {
                self.take(transform.translate.len(), depth)?;
                self.take(transform.rotate.len(), depth)?;
                self.take(transform.scale.len(), depth)?;
                self.take(transform.skew.len(), depth)?;
            }
            Value::Mixture(mixture) => self.take(mixture.components.len(), depth)?,
            // Remaining domain values contain bounded-depth scalar/string data or
            // opaque resource references. Backing provider memory is not counted here.
            _ => {}
        }
        Ok(())
    }
    fn expression(&mut self, expression: &Expr, depth: usize) -> Result<(), Diagnostic> {
        self.take(1, depth)?;
        match &expression.kind {
            ExprKind::Binary { left, right, .. } | ExprKind::Pipe { left, right } => {
                self.expression(left, depth + 1)?;
                self.expression(right, depth + 1)?;
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::Await(expr)
            | ExprKind::Try(expr)
            | ExprKind::Member { recv: expr, .. } => self.expression(expr, depth + 1)?,
            ExprKind::Call { callee, args } => {
                self.expression(callee, depth + 1)?;
                for argument in args {
                    match argument {
                        Arg::Pos(value) => self.expression(value, depth + 1)?,
                        Arg::Named(value) => self.expression(&value.value, depth + 1)?,
                    }
                }
            }
            ExprKind::Index { recv, index } => {
                self.expression(recv, depth + 1)?;
                self.expression(index, depth + 1)?;
            }
            ExprKind::List(items) | ExprKind::Interpolate(items) => {
                for item in items {
                    self.expression(item, depth + 1)?;
                }
            }
            ExprKind::Record(items) => {
                for item in items {
                    self.expression(&item.value, depth + 1)?;
                }
            }
            ExprKind::Triple {
                subject,
                predicate,
                object,
            } => {
                self.expression(subject, depth + 1)?;
                self.expression(predicate, depth + 1)?;
                self.expression(object, depth + 1)?;
            }
            ExprKind::Reified {
                subject,
                predicate,
                object,
                reifier,
            } => {
                self.expression(subject, depth + 1)?;
                self.expression(predicate, depth + 1)?;
                self.expression(object, depth + 1)?;
                self.expression(reifier, depth + 1)?;
            }
            ExprKind::ModalLogic { args, body, .. } => {
                for arg in args {
                    self.expression(arg, depth + 1)?;
                }
                if let Some(body) = body {
                    self.expression(body, depth + 1)?;
                }
            }
            ExprKind::Lambda { params, body } => {
                self.take(params.len(), depth)?;
                self.expression(body, depth + 1)?;
            }
            ExprKind::Tween {
                from,
                to,
                over,
                spring,
                ..
            } => {
                self.expression(from, depth + 1)?;
                self.expression(to, depth + 1)?;
                self.expression(over, depth + 1)?;
                if let Some(spring) = spring {
                    for arg in spring {
                        self.expression(&arg.value, depth + 1)?;
                    }
                }
            }
            ExprKind::IfElse {
                cond,
                then_value,
                else_value,
            } => {
                self.expression(cond, depth + 1)?;
                self.expression(then_value, depth + 1)?;
                self.expression(else_value, depth + 1)?;
            }
            ExprKind::GraphQuery { variables, .. } => self.take(variables.len(), depth)?,
            ExprKind::Literal(_)
            | ExprKind::Ident(_)
            | ExprKind::QueryVar(_)
            | ExprKind::Iri(_)
            | ExprKind::Prefixed(_, _)
            | ExprKind::Blank(_) => {}
        }
        Ok(())
    }
}

pub(super) fn retained_bytes(value: &Value, span: Span) -> Result<usize, Diagnostic> {
    Work {
        remaining: 16_384,
        span,
    }
    .value(value, 0)?;
    struct Counter(usize);
    impl std::fmt::Write for Counter {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0 = self.0.checked_add(text.len()).ok_or(std::fmt::Error)?;
            if self.0 > 1024 * 1024 {
                return Err(std::fmt::Error);
            }
            Ok(())
        }
    }
    let mut counter = Counter(0);
    std::fmt::write(&mut counter, format_args!("{value:?}")).map_err(|_| {
        error(
            span,
            "task result exceeds the 1 MiB retained representation limit",
        )
    })?;
    Ok(counter.0)
}
