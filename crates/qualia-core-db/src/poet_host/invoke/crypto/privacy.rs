//! Differential privacy invoke seams.
//!
//! Exposes `specialized_libs::linear_algebra::privacy` functions through
//! VibeScript invoke IDs in the `Privacy.*` namespace.

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `Privacy.gaussian_sigma` — compute the Gaussian noise sigma for (ε, δ)-DP.
/// Args: { sensitivity: f64, epsilon: f64, delta: f64 }
pub fn gaussian_sigma(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sensitivity = args::rec_f64(args, "sensitivity")
        .ok_or_else(|| args::bad(span, "Privacy.gaussian_sigma needs sensitivity"))?;
    let epsilon = args::rec_f64(args, "epsilon")
        .ok_or_else(|| args::bad(span, "Privacy.gaussian_sigma needs epsilon"))?;
    let delta = args::rec_f64(args, "delta")
        .ok_or_else(|| args::bad(span, "Privacy.gaussian_sigma needs delta"))?;
    match crate::specialized_libs::linear_algebra::privacy::gaussian_sigma(
        sensitivity,
        epsilon,
        delta,
    ) {
        Ok(sigma) => Ok(args::record([("gaussian_sigma", Value::F64(sigma))])),
        Err(e) => Err(args::bad(span, format!("gaussian_sigma: {e:?}"))),
    }
}
