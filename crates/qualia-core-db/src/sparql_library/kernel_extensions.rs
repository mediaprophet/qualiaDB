//! Qualia kernel SPARQL extension functions (UE-052).
//!
//! Scalar numeric helpers callable as `q42:mean`, `q42:gini`, … from FILTER /
//! BIND. Fail closed on empty / non-finite input. Not a substitute for the
//! receipted WASM welfare surface — these are query-local scalars.

use crate::q_hash;

/// Recognised kernel extension functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelFn {
    Mean,
    Min,
    Max,
    Sum,
    Gini,
    Rawlsian,
}

const IRIS: &[(&str, KernelFn)] = &[
    ("q42:mean", KernelFn::Mean),
    ("https://webizen.org/ns/q42#mean", KernelFn::Mean),
    ("q42:min", KernelFn::Min),
    ("https://webizen.org/ns/q42#min", KernelFn::Min),
    ("q42:max", KernelFn::Max),
    ("https://webizen.org/ns/q42#max", KernelFn::Max),
    ("q42:sum", KernelFn::Sum),
    ("https://webizen.org/ns/q42#sum", KernelFn::Sum),
    ("q42:gini", KernelFn::Gini),
    ("https://webizen.org/ns/q42#gini", KernelFn::Gini),
    ("q42:rawlsian", KernelFn::Rawlsian),
    ("https://webizen.org/ns/q42#rawlsian", KernelFn::Rawlsian),
];

/// Resolve a function IRI hash to a kernel op.
pub fn kernel_fn_for_hash(iri_hash: u64) -> Option<KernelFn> {
    for &(iri, op) in IRIS {
        if q_hash(iri) == iri_hash {
            return Some(op);
        }
    }
    None
}

/// Evaluate a kernel extension over float arguments.
pub fn eval_kernel_fn(op: KernelFn, args: &[f64]) -> Result<f64, String> {
    if args.is_empty() {
        return Err("q42 kernel function requires at least one numeric argument".into());
    }
    if args.iter().any(|x| !x.is_finite()) {
        return Err("q42 kernel function: non-finite argument".into());
    }
    match op {
        KernelFn::Mean => Ok(args.iter().sum::<f64>() / args.len() as f64),
        KernelFn::Min => Ok(args.iter().copied().fold(f64::INFINITY, f64::min)),
        KernelFn::Max => Ok(args.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        KernelFn::Sum => Ok(args.iter().sum()),
        KernelFn::Rawlsian => Ok(args.iter().copied().fold(f64::INFINITY, f64::min)),
        KernelFn::Gini => gini(args),
    }
}

fn gini(incomes: &[f64]) -> Result<f64, String> {
    if incomes.iter().any(|v| *v < 0.0) {
        return Err("q42:gini: negative income".into());
    }
    let n = incomes.len();
    let mut sum = 0.0f64;
    let mut total = 0.0f64;
    for &yi in incomes {
        total += yi;
        for &yj in incomes {
            sum += (yi - yj).abs();
        }
    }
    if total <= 0.0 {
        return Err("q42:gini: all-zero incomes".into());
    }
    let mean = total / n as f64;
    Ok(sum / (2.0 * n as f64 * n as f64 * mean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_gini_basic() {
        assert!((eval_kernel_fn(KernelFn::Mean, &[1.0, 2.0, 3.0]).unwrap() - 2.0).abs() < 1e-12);
        let g = eval_kernel_fn(KernelFn::Gini, &[0.0, 1.0]).unwrap();
        assert!((g - 0.5).abs() < 1e-12);
    }

    #[test]
    fn hash_lookup_stable() {
        assert_eq!(
            kernel_fn_for_hash(q_hash("q42:mean")),
            Some(KernelFn::Mean)
        );
        assert_eq!(kernel_fn_for_hash(q_hash("q42:nope")), None);
    }
}
