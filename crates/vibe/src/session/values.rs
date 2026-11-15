//! Retain declared primitive/container types across submissions. This adds no
//! coercion or 0.2 type lattice and does not claim complete 0.1 type inference.
use super::*;
use crate::types::Type;
use std::fmt::Write;

pub(super) fn check_retained_size(env: &Env, limit: usize) -> Result<(), Diagnostic> {
    struct Counter(usize);
    impl std::fmt::Write for Counter {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.0 = self.0.checked_sub(s.len()).ok_or(std::fmt::Error)?;
            Ok(())
        }
    }
    let mut counter = Counter(limit);
    write!(
        &mut counter,
        "{:?}{:?}{:?}{:?}{:?}{:?}",
        env.vars, env.fields, env.materials, env.laws, env.enums, env.aliases
    )
    .map_err(|_| {
        Diagnostic::new(
            DiagCode::E400,
            Span::point(0),
            "session retained-value representation limit exceeded",
        )
    })
}

pub(super) fn validate(checked: &CheckEnv, env: &Env) -> Result<(), Diagnostic> {
    for (name, value) in &env.vars {
        if let Some(ty) = checked.get(name) {
            if !matches_type(value, ty) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    Span::point(0),
                    format!("session binding `{name}` does not match retained type {ty:?}"),
                ));
            }
        }
    }
    Ok(())
}
pub(super) fn validate_source(checked: &CheckEnv) -> Result<(), Diagnostic> {
    for (name, expr) in &checked.values {
        if let (Some(ty), Some(value)) = (checked.get(name), literal_value(expr)) {
            if !matches_type(&value, ty) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    expr.span,
                    format!("session binding `{name}` does not match retained type {ty:?}"),
                ));
            }
        }
    }
    Ok(())
}
fn literal_value(expr: &Expr) -> Option<Value> {
    match &expr.kind {
        ExprKind::Literal(lit) => Some(crate::eval::lit(lit)),
        ExprKind::List(xs) => xs
            .iter()
            .map(literal_value)
            .collect::<Option<Vec<_>>>()
            .map(Value::List),
        _ => None,
    }
}
fn matches_type(value: &Value, ty: &Type) -> bool {
    match ty {
        Type::I64 => matches!(value, Value::I64(_)),
        Type::U64 => matches!(value, Value::U64(_)),
        Type::F64 => matches!(value, Value::F64(_)),
        Type::Bool => matches!(value, Value::Bool(_)),
        Type::String => matches!(value, Value::String(_)),
        Type::Iri => matches!(value, Value::Iri(_) | Value::Prefixed(_, _) | Value::String(_)),
        Type::BlankNode => matches!(value, Value::Blank(_)),
        Type::Var => matches!(value, Value::Var(_)),
        Type::TripleTerm => matches!(value, Value::Triple(..)),
        Type::Reifier => matches!(value, Value::Reified { .. }),
        Type::QuinRef => matches!(value, Value::QuinRef(_)),
        Type::AssetRef => matches!(value, Value::AssetRef(_)),
        Type::TensorRef => matches!(value, Value::TensorRef(_)),
        Type::GeometryRef => matches!(value, Value::GeometryRef(_)),
        Type::Instant => matches!(value, Value::Instant(_)),
        Type::Duration => matches!(value, Value::Duration(_)),
        Type::Quantity => matches!(value, Value::Quantity(_)),
        Type::Frame => matches!(value, Value::Frame(_)),
        Type::Pose => matches!(value, Value::Pose(_)),
        Type::Transform => matches!(value, Value::Transform(_)),
        Type::FieldRef => matches!(value, Value::FieldRef(_)),
        Type::MaterialRef => matches!(value, Value::MaterialRef(_)),
        Type::WorldLine => matches!(value, Value::WorldLine(_)),
        Type::Enum(name) => matches!(value, Value::Enum(ev) if ev.enum_name == *name),
        Type::SpeciesRef => matches!(value, Value::SpeciesRef(_)),
        Type::Mixture => matches!(value, Value::Mixture(_)),
        Type::Receipt => matches!(value, Value::Receipt),
        Type::Option(inner) => matches!(value, Value::Null) || matches_type(value, inner),
        Type::Result(ok, err) => match value {
            Value::Ok(v) => matches_type(v, ok),
            Value::Err(e) => matches_type(e, err),
            _ => false,
        },
        Type::List(inner) => {
            matches!(value, Value::List(xs) if xs.iter().all(|v| matches_type(v, inner)))
        }
        Type::Record => matches!(value, Value::Record(_)),
        // Other 0.1 domain values (Bytes/Did/Continuant/Hash/Literal/Quin/Stream/Future/Named/Unknown)
        // keep their existing evaluator representation.
        _ => true,
    }
}
