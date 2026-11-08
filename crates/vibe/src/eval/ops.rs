//! Literal evaluation, binary arithmetic/comparison, and pattern matching helpers for `eval`.

use crate::ast::*;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::Env;

/// Literal expression.
pub(crate) fn lit(l: &Literal) -> Value {
    match l {
        Literal::Null => Value::Null,
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Int(n) => Value::I64(*n),
        Literal::UInt(n) => Value::U64(*n),
        Literal::Float(bits) => Value::F64(f64::from_bits(*bits)),
        Literal::Quantity { value, unit } => Value::Quantity(crate::value::Quantity {
            value: f64::from_bits(*value),
            unit: unit.clone(),
        }),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Color(c) => {
            let mut rec = std::collections::BTreeMap::new();
            rec.insert("space".into(), Value::String("srgb".into()));
            rec.insert("r".into(), Value::U64(c.r as u64));
            rec.insert("g".into(), Value::U64(c.g as u64));
            rec.insert("b".into(), Value::U64(c.b as u64));
            rec.insert("a".into(), Value::U64(c.a as u64));
            rec.insert(
                "hex".into(),
                Value::String(format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a)),
            );
            Value::Record(rec)
        }
    }
}

/// Binary operation.
pub(super) fn binop(op: BinOp, l: &Value, r: &Value, span: Span) -> Result<Value, Diagnostic> {
    if matches!(op, BinOp::Eq | BinOp::Ne) {
        let eq = l == r;
        return Ok(Value::Bool(if op == BinOp::Eq { eq } else { !eq }));
    }

    if let (Value::I64(a), Value::I64(b)) = (l, r) {
        return match op {
            BinOp::Add => a.checked_add(*b).map(Value::I64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")
            }),
            BinOp::Sub => a.checked_sub(*b).map(Value::I64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")
            }),
            BinOp::Mul => a.checked_mul(*b).map(Value::I64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")
            }),
            BinOp::Div => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_div(*b).map(Value::I64).ok_or_else(|| {
                        Diagnostic::new(DiagCode::E600, span, "integer overflow on division")
                    })
                }
            }
            BinOp::Rem => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_rem(*b).map(Value::I64).ok_or_else(|| {
                        Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder")
                    })
                }
            }
            BinOp::Lt => Ok(Value::Bool(a < b)),
            BinOp::Le => Ok(Value::Bool(a <= b)),
            BinOp::Gt => Ok(Value::Bool(a > b)),
            BinOp::Ge => Ok(Value::Bool(a >= b)),
            _ => Ok(Value::I64(*a)),
        };
    }

    if let (Value::U64(a), Value::U64(b)) = (l, r) {
        return match op {
            BinOp::Add => a.checked_add(*b).map(Value::U64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")
            }),
            BinOp::Sub => a.checked_sub(*b).map(Value::U64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")
            }),
            BinOp::Mul => a.checked_mul(*b).map(Value::U64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")
            }),
            BinOp::Div => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_div(*b).map(Value::U64).ok_or_else(|| {
                        Diagnostic::new(DiagCode::E600, span, "integer overflow on division")
                    })
                }
            }
            BinOp::Rem => {
                if *b == 0 {
                    Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                } else {
                    a.checked_rem(*b).map(Value::U64).ok_or_else(|| {
                        Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder")
                    })
                }
            }
            BinOp::Lt => Ok(Value::Bool(a < b)),
            BinOp::Le => Ok(Value::Bool(a <= b)),
            BinOp::Gt => Ok(Value::Bool(a > b)),
            BinOp::Ge => Ok(Value::Bool(a >= b)),
            _ => Ok(Value::U64(*a)),
        };
    }

    if let (Value::U64(a), Value::I64(b)) = (l, r) {
        if *b >= 0 {
            let b_u64 = *b as u64;
            return match op {
                BinOp::Add => a.checked_add(b_u64).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")
                }),
                BinOp::Sub => a.checked_sub(b_u64).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")
                }),
                BinOp::Mul => a.checked_mul(b_u64).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")
                }),
                BinOp::Div => {
                    if b_u64 == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a.checked_div(b_u64).map(Value::U64).ok_or_else(|| {
                            Diagnostic::new(DiagCode::E600, span, "integer overflow on division")
                        })
                    }
                }
                BinOp::Rem => {
                    if b_u64 == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a.checked_rem(b_u64).map(Value::U64).ok_or_else(|| {
                            Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder")
                        })
                    }
                }
                BinOp::Lt => Ok(Value::Bool(*a < b_u64)),
                BinOp::Le => Ok(Value::Bool(*a <= b_u64)),
                BinOp::Gt => Ok(Value::Bool(*a > b_u64)),
                BinOp::Ge => Ok(Value::Bool(*a >= b_u64)),
                _ => Ok(Value::U64(*a)),
            };
        } else {
            let neg_b = (-*b) as u64;
            return match op {
                BinOp::Add => a.checked_sub(neg_b).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer underflow on addition")
                }),
                BinOp::Sub => a.checked_add(neg_b).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")
                }),
                _ => Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "unsupported op on unsigned and negative integer",
                )),
            };
        }
    }

    if let (Value::I64(a), Value::U64(b)) = (l, r) {
        if *a >= 0 {
            let a_u64 = *a as u64;
            return match op {
                BinOp::Add => a_u64.checked_add(*b).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on addition")
                }),
                BinOp::Sub => a_u64.checked_sub(*b).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on subtraction")
                }),
                BinOp::Mul => a_u64.checked_mul(*b).map(Value::U64).ok_or_else(|| {
                    Diagnostic::new(DiagCode::E600, span, "integer overflow on multiplication")
                }),
                BinOp::Div => {
                    if *b == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a_u64.checked_div(*b).map(Value::U64).ok_or_else(|| {
                            Diagnostic::new(DiagCode::E600, span, "integer overflow on division")
                        })
                    }
                }
                BinOp::Rem => {
                    if *b == 0 {
                        Err(Diagnostic::new(DiagCode::E600, span, "division by zero"))
                    } else {
                        a_u64.checked_rem(*b).map(Value::U64).ok_or_else(|| {
                            Diagnostic::new(DiagCode::E600, span, "integer overflow on remainder")
                        })
                    }
                }
                BinOp::Lt => Ok(Value::Bool(a_u64 < *b)),
                BinOp::Le => Ok(Value::Bool(a_u64 <= *b)),
                BinOp::Gt => Ok(Value::Bool(a_u64 > *b)),
                BinOp::Ge => Ok(Value::Bool(a_u64 >= *b)),
                _ => Ok(Value::U64(a_u64)),
            };
        } else {
            return Err(Diagnostic::new(
                DiagCode::E600,
                span,
                "unsupported op on negative integer and unsigned",
            ));
        }
    }

    if let (Value::Quantity(a), Value::Quantity(b)) = (l, r) {
        if a.unit != b.unit {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!(
                    "unit mismatch: cannot {op:?} quantities of `{}` and `{}`",
                    a.unit, b.unit
                ),
            ));
        }
        if matches!(op, BinOp::Div | BinOp::Rem) && b.value == 0.0 {
            return Err(Diagnostic::new(DiagCode::E600, span, "division by zero"));
        }
        return match op {
            BinOp::Add => Ok(Value::Quantity(crate::value::Quantity {
                value: a.value + b.value,
                unit: a.unit.clone(),
            })),
            BinOp::Sub => Ok(Value::Quantity(crate::value::Quantity {
                value: a.value - b.value,
                unit: a.unit.clone(),
            })),
            BinOp::Mul => Ok(Value::Quantity(crate::value::Quantity {
                value: a.value * b.value,
                unit: a.unit.clone(),
            })),
            BinOp::Div => Ok(Value::Quantity(crate::value::Quantity {
                value: a.value / b.value,
                unit: a.unit.clone(),
            })),
            BinOp::Rem => Ok(Value::Quantity(crate::value::Quantity {
                value: a.value % b.value,
                unit: a.unit.clone(),
            })),
            BinOp::Lt => Ok(Value::Bool(a.value < b.value)),
            BinOp::Le => Ok(Value::Bool(a.value <= b.value)),
            BinOp::Gt => Ok(Value::Bool(a.value > b.value)),
            BinOp::Ge => Ok(Value::Bool(a.value >= b.value)),
            _ => Ok(Value::Quantity(a.clone())),
        };
    }

    if let Value::Quantity(a) = l {
        if let Some(b) = r.as_f64() {
            if matches!(op, BinOp::Div | BinOp::Rem) && b == 0.0 {
                return Err(Diagnostic::new(DiagCode::E600, span, "division by zero"));
            }
            return match op {
                BinOp::Mul => Ok(Value::Quantity(crate::value::Quantity {
                    value: a.value * b,
                    unit: a.unit.clone(),
                })),
                BinOp::Div => Ok(Value::Quantity(crate::value::Quantity {
                    value: a.value / b,
                    unit: a.unit.clone(),
                })),
                _ => Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "quantity and number only support * and /",
                )),
            };
        }
    }

    if let Value::Quantity(b) = r {
        if let Some(a) = l.as_f64() {
            return match op {
                BinOp::Mul => Ok(Value::Quantity(crate::value::Quantity {
                    value: a * b.value,
                    unit: b.unit.clone(),
                })),
                _ => Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "number and quantity only support *",
                )),
            };
        }
    }

    let a = l
        .as_f64()
        .or_else(|| match l {
            Value::List(xs) => xs.first().and_then(|v| v.as_f64()),
            _ => None,
        })
        .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "numeric operator on non-number"))?;
    let b = r
        .as_f64()
        .or_else(|| match r {
            Value::List(xs) => xs.first().and_then(|v| v.as_f64()),
            _ => None,
        })
        .ok_or_else(|| Diagnostic::new(DiagCode::E600, span, "numeric operator on non-number"))?;
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
    Ok(Value::F64(n))
}

/// Match pattern.
pub(super) fn match_pat(p: &Pattern, v: &Value, env: &mut Env) -> bool {
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
        Pattern::Record(fields) => match v {
            Value::Record(rec) => {
                for (name, pat) in fields {
                    if let Some(val) = rec.get(name) {
                        if !match_pat(pat, val, env) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            _ => false,
        },
        Pattern::List(elements) => match v {
            Value::List(items) => {
                if items.len() == elements.len() {
                    elements
                        .iter()
                        .zip(items.iter())
                        .all(|(pat, val)| match_pat(pat, val, env))
                } else {
                    false
                }
            }
            _ => false,
        },
        Pattern::Constructor { name, args } => match v {
            Value::List(items)
                if matches!(name.as_str(), "vec2" | "vec3" | "vec4" | "mat3" | "mat4") =>
            {
                if items.len() == args.len() {
                    args.iter()
                        .zip(items.iter())
                        .all(|(pat, val)| match_pat(pat, val, env))
                } else {
                    false
                }
            }
            _ => false,
        },
        Pattern::Variant {
            enum_name,
            variant_name,
            args,
        } => match v {
            Value::Enum(e) if e.enum_name == *enum_name && e.variant_name == *variant_name => {
                if args.is_empty() {
                    e.payload.is_empty()
                } else if args.len() == e.payload.len() {
                    args.iter()
                        .zip(e.payload.iter())
                        .all(|(pat, val)| match_pat(pat, val, env))
                } else {
                    false
                }
            }
            _ => false,
        },
    }
}

/// Collects pattern identifiers.
pub(super) fn collect_pat_idents(pat: &Pattern, out: &mut Vec<String>) {
    match pat {
        Pattern::Ident(n) => out.push(n.clone()),
        Pattern::Record(fields) => {
            for (_, p) in fields {
                collect_pat_idents(p, out);
            }
        }
        Pattern::List(elements) => {
            for p in elements {
                collect_pat_idents(p, out);
            }
        }
        Pattern::Constructor { args, .. } | Pattern::Variant { args, .. } => {
            for p in args {
                collect_pat_idents(p, out);
            }
        }
        Pattern::Ok(p) | Pattern::Err(p) | Pattern::Some(p) => {
            collect_pat_idents(p, out);
        }
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::None => {}
    }
}
