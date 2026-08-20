//! Number-theory kernels already in `solvers::number_theory::modular`.

use crate::solvers::number_theory::modular::{gcd as nt_gcd, lcm as nt_lcm};
use crate::solvers::number_theory::primes::is_prime as nt_is_prime;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

fn pair(args: &Value, span: Span) -> Result<(u64, u64), Diagnostic> {
    let Value::List(xs) = args else {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "NumberTheory needs [a, b]",
        ));
    };
    let a = match xs.first() {
        Some(Value::U64(n)) => *n,
        Some(Value::I64(n)) if *n >= 0 => *n as u64,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "gcd/lcm needs two integers",
            ))
        }
    };
    let b = match xs.get(1) {
        Some(Value::U64(n)) => *n,
        Some(Value::I64(n)) if *n >= 0 => *n as u64,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "gcd/lcm needs two integers",
            ))
        }
    };
    Ok((a, b))
}

pub fn gcd(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (a, b) = pair(args, span)?;
    Ok(Value::U64(nt_gcd(a, b)))
}

pub fn lcm(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (a, b) = pair(args, span)?;
    Ok(Value::U64(nt_lcm(a, b)))
}

pub fn is_prime(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = match args {
        Value::U64(n) => *n,
        Value::I64(n) if *n >= 0 => *n as u64,
        Value::List(xs) => match xs.first() {
            Some(Value::U64(n)) => *n,
            Some(Value::I64(n)) if *n >= 0 => *n as u64,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "is_prime needs an integer",
                ))
            }
        },
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "is_prime needs an integer",
            ))
        }
    };
    Ok(Value::Bool(nt_is_prime(n)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_48_18() {
        let args = Value::List(vec![Value::U64(48), Value::U64(18)]);
        assert_eq!(
            gcd(&args, Span { start: 0, end: 0 }).unwrap(),
            Value::U64(6)
        );
    }
}
