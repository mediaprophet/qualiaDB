//! Fourier transforms — `solvers::transforms::fourier`.
//!
//! Wraps the engine's wasm-clean DFT reference. The solver owns the exact f64
//! math; this seam only marshals `Value` ↔ `Cplx` and packs the spectrum into a
//! record (`re`, `im`, `magnitude`, `n`). Same code the WASM export and the
//! solver unit tests exercise.

use super::super::args;
use crate::solvers::transforms::fourier::{dft as fourier_dft, Cplx};
use vibe::{Diagnostic, Span, Value};

/// Forward Discrete Fourier Transform of a real signal.
///
/// Input: record `{ data: [f64, ..] }` (a real-valued signal; each sample is
/// lifted to the complex plane as `(x, 0.0)`).
/// Output: record `{ re: [f64], im: [f64], magnitude: [f64], n: usize }` where
/// `magnitude[k] = sqrt(re[k]^2 + im[k]^2)`.
pub fn dft(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec(args_v, "data")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "dft needs data: a number list"))?;
    let x: Vec<Cplx> = data.iter().map(|&r| (r, 0.0)).collect();
    let spectrum = fourier_dft(&x);
    let n = spectrum.len();
    let re: Vec<f64> = spectrum.iter().map(|c| c.0).collect();
    let im: Vec<f64> = spectrum.iter().map(|c| c.1).collect();
    let magnitude: Vec<f64> = spectrum
        .iter()
        .map(|c| (c.0 * c.0 + c.1 * c.1).sqrt())
        .collect();
    Ok(args::record([
        ("re", args::f64_list_value(re)),
        ("im", args::f64_list_value(im)),
        ("magnitude", args::f64_list_value(magnitude)),
        ("n", Value::U64(n as u64)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn dft_of_constant_is_an_impulse() {
        // DFT([1,1,1,1]) = [4,0,0,0] — bin 0 holds the DC sum, rest zero.
        let mut m = BTreeMap::new();
        m.insert("data".into(), args::f64_list_value([1.0, 1.0, 1.0, 1.0]));
        let v = dft(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => {
                let re = match r.get("re") {
                    Some(Value::List(xs)) => xs,
                    other => panic!("re: {other:?}"),
                };
                let im = match r.get("im") {
                    Some(Value::List(xs)) => xs,
                    other => panic!("im: {other:?}"),
                };
                let mag = match r.get("magnitude") {
                    Some(Value::List(xs)) => xs,
                    other => panic!("magnitude: {other:?}"),
                };
                assert_eq!(
                    match r.get("n") {
                        Some(Value::U64(n)) => *n,
                        other => panic!("n: {other:?}"),
                    },
                    4
                );
                assert!(match &re[0] {
                    Value::F64(x) => (x - 4.0).abs() < 1e-9,
                    _ => false,
                });
                assert!(match &im[0] {
                    Value::F64(x) => x.abs() < 1e-9,
                    _ => false,
                });
                assert!(match &mag[0] {
                    Value::F64(x) => (x - 4.0).abs() < 1e-9,
                    _ => false,
                });
                for k in 1..4 {
                    assert!(match &re[k] {
                        Value::F64(x) => x.abs() < 1e-9,
                        _ => false,
                    });
                    assert!(match &mag[k] {
                        Value::F64(x) => x.abs() < 1e-9,
                        _ => false,
                    });
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dft_needs_data() {
        let v = Value::Record(BTreeMap::new());
        assert!(dft(&v, Span { start: 0, end: 0 }).is_err());
    }
}
