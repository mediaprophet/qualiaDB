//! Wave-16 Host binds: pure EngineeringAnalysis vibration / buckling / reliability.
//!
//! No CFD / CUDA / forge paths — scalar closed-form and Acklam Φ⁻¹ only.

use super::super::args;
use crate::specialized_libs::engineering_analysis as eng;
use vibe::{Diagnostic, Span, Value};

/// `EngineeringAnalysis.analyze_harmonic_sdof` — SDOF FRF amplitude + phase.
/// Args: `{ mass, damping, stiffness, force_amplitude, excitation_freqs: [f64] }`.
/// Out: `{ excitation_frequencies, response_amplitudes, phase_angles }`.
pub fn analyze_harmonic_sdof(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mass = args::rec_f64(args_v, "mass")
        .ok_or_else(|| args::bad(span, "analyze_harmonic_sdof needs mass"))?;
    let damping = args::rec_f64(args_v, "damping")
        .ok_or_else(|| args::bad(span, "analyze_harmonic_sdof needs damping"))?;
    let stiffness = args::rec_f64(args_v, "stiffness")
        .ok_or_else(|| args::bad(span, "analyze_harmonic_sdof needs stiffness"))?;
    let force_amplitude = args::rec_f64(args_v, "force_amplitude")
        .ok_or_else(|| args::bad(span, "analyze_harmonic_sdof needs force_amplitude"))?;
    let freqs = args::rec_f64_list(args_v, "excitation_freqs").ok_or_else(|| {
        args::bad(span, "analyze_harmonic_sdof needs excitation_freqs: [f64]")
    })?;
    let mut vib = eng::VibrationAnalysis::new();
    match vib.analyze_harmonic_sdof(mass, damping, stiffness, force_amplitude, &freqs) {
        Ok(fv) => Ok(args::record([
            (
                "excitation_frequencies",
                Value::List(
                    fv.excitation_frequencies
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            (
                "response_amplitudes",
                Value::List(
                    fv.response_amplitudes
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            (
                "phase_angles",
                Value::List(fv.phase_angles.iter().map(|v| Value::F64(*v)).collect()),
            ),
        ])),
        Err(e) => Err(args::bad(span, format!("analyze_harmonic_sdof: {e:?}"))),
    }
}

/// `EngineeringAnalysis.analyze_euler` — Euler column buckling critical loads.
/// Args: `{ youngs_modulus, moment_of_inertia, length, effective_length_factor, num_modes }`.
/// Out: `{ critical_loads: [f64] }`.
pub fn analyze_euler(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let e = args::rec_f64(args_v, "youngs_modulus")
        .ok_or_else(|| args::bad(span, "analyze_euler needs youngs_modulus"))?;
    let i = args::rec_f64(args_v, "moment_of_inertia")
        .ok_or_else(|| args::bad(span, "analyze_euler needs moment_of_inertia"))?;
    let length = args::rec_f64(args_v, "length")
        .ok_or_else(|| args::bad(span, "analyze_euler needs length"))?;
    let k = args::rec_f64(args_v, "effective_length_factor")
        .ok_or_else(|| args::bad(span, "analyze_euler needs effective_length_factor"))?;
    let num_modes = args::rec_u64(args_v, "num_modes")
        .ok_or_else(|| args::bad(span, "analyze_euler needs num_modes"))? as usize;
    let mut ba = eng::BucklingAnalysis::new();
    match ba.analyze_euler(e, i, length, k, num_modes) {
        Ok(eb) => Ok(args::record([(
            "critical_loads",
            Value::List(eb.critical_loads.iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("analyze_euler: {e:?}"))),
    }
}

/// `EngineeringAnalysis.compute_reliability_index` — β = −Φ⁻¹(p_f).
/// Args: `{ failure_prob }`. Out: `{ beta }`.
pub fn compute_reliability_index(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64(args_v, "failure_prob")
        .ok_or_else(|| args::bad(span, "compute_reliability_index needs failure_prob"))?;
    let analyzer = eng::ReliabilityAnalyzer::new();
    let beta = analyzer.compute_reliability_index(p);
    Ok(args::record([("beta", Value::F64(beta))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave16_analyze_harmonic_sdof_static() {
        let mut m = BTreeMap::new();
        m.insert("mass".into(), Value::F64(1.0));
        m.insert("damping".into(), Value::F64(0.0));
        m.insert("stiffness".into(), Value::F64(4.0));
        m.insert("force_amplitude".into(), Value::F64(2.0));
        m.insert(
            "excitation_freqs".into(),
            Value::List(vec![Value::F64(0.0)]),
        );
        let out = analyze_harmonic_sdof(&Value::Record(m), span()).unwrap();
        let amps = args::rec_f64_list(&out, "response_amplitudes").unwrap();
        // X(0) = F₀/k = 2/4 = 0.5
        assert!((amps[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn wave16_analyze_euler_pinned() {
        let mut m = BTreeMap::new();
        m.insert("youngs_modulus".into(), Value::F64(1.0));
        m.insert("moment_of_inertia".into(), Value::F64(1.0));
        m.insert("length".into(), Value::F64(1.0));
        m.insert("effective_length_factor".into(), Value::F64(1.0));
        m.insert("num_modes".into(), Value::U64(1));
        let out = analyze_euler(&Value::Record(m), span()).unwrap();
        let loads = args::rec_f64_list(&out, "critical_loads").unwrap();
        let expected = std::f64::consts::PI.powi(2);
        assert!((loads[0] - expected).abs() < 1e-9);
    }

    #[test]
    fn wave16_compute_reliability_index_half() {
        let mut m = BTreeMap::new();
        m.insert("failure_prob".into(), Value::F64(0.5));
        let out = compute_reliability_index(&Value::Record(m), span()).unwrap();
        // Φ⁻¹(0.5)=0 → β=0
        assert!(args::rec_f64(&out, "beta").unwrap().abs() < 1e-6);
    }
}
