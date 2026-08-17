//! Black–Scholes — `specialized_libs::computational_economics::derivatives`.

use super::super::args;
use crate::specialized_libs::computational_economics::derivatives::{
    black_scholes_price_and_greeks, OptionKind,
};
use poet_vibe::{Diagnostic, Span, Value};

pub fn price(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let kind = match args::rec_str(args_v, "kind").unwrap_or("call") {
        "put" | "Put" => OptionKind::Put,
        _ => OptionKind::Call,
    };
    let r = black_scholes_price_and_greeks(
        kind,
        args::rec_f64(args_v, "spot").ok_or_else(|| args::bad(span, "black_scholes needs spot"))?,
        args::rec_f64(args_v, "strike").ok_or_else(|| args::bad(span, "black_scholes needs strike"))?,
        args::rec_f64(args_v, "rate").unwrap_or(0.0),
        args::rec_f64(args_v, "dividend").unwrap_or(0.0),
        args::rec_f64(args_v, "vol").ok_or_else(|| args::bad(span, "black_scholes needs vol"))?,
        args::rec_f64(args_v, "time").ok_or_else(|| args::bad(span, "black_scholes needs time"))?,
    )
    .map_err(|e| args::bad(span, format!("black_scholes: {e:?}")))?;
    Ok(args::record([
        ("price", Value::F64(r.price)),
        ("delta", Value::F64(r.delta)),
        ("gamma", Value::F64(r.gamma)),
        ("vega", Value::F64(r.vega)),
        ("theta", Value::F64(r.theta)),
        ("rho", Value::F64(r.rho)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn atm_call_positive() {
        let mut m = BTreeMap::new();
        m.insert("spot".into(), Value::F64(100.0));
        m.insert("strike".into(), Value::F64(100.0));
        m.insert("rate".into(), Value::F64(0.01));
        m.insert("vol".into(), Value::F64(0.2));
        m.insert("time".into(), Value::F64(1.0));
        match price(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("price") {
                Some(Value::F64(p)) => assert!(*p > 0.0),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
