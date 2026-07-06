//! Hamiltonian mechanics primitives with invariant diagnostics.

use super::analysis::{AnalysisError, Vector};

pub fn canonical_poisson_bracket<const N: usize>(
    df_dq: Vector<N>,
    df_dp: Vector<N>,
    dg_dq: Vector<N>,
    dg_dp: Vector<N>,
) -> Result<f64, AnalysisError> {
    Ok(df_dq.dot(dg_dp)? - df_dp.dot(dg_dq)?)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseState<const N: usize> {
    pub q: Vector<N>,
    pub p: Vector<N>,
}

pub fn stormer_verlet_step<const N: usize, F, G>(
    state: &mut PhaseState<N>,
    step: f64,
    potential_gradient: F,
    kinetic_gradient: G,
) -> Result<(), AnalysisError>
where
    F: Fn(Vector<N>) -> Vector<N>,
    G: Fn(Vector<N>) -> Vector<N>,
{
    if !step.is_finite() || step == 0.0 {
        return Err(AnalysisError::InvalidDomain);
    }
    state.q.validate()?;
    state.p.validate()?;
    let half_force = potential_gradient(state.q).scale(-0.5 * step)?;
    state.p = state.p.add(half_force);
    state.q = state.q.add(kinetic_gradient(state.p).scale(step)?);
    state.p = state.p.add(potential_gradient(state.q).scale(-0.5 * step)?);
    state.q.validate()?;
    state.p.validate()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InvariantDrift {
    pub initial: f64,
    pub final_value: f64,
    pub absolute_drift: f64,
    pub relative_drift: f64,
}

pub fn invariant_drift(initial: f64, final_value: f64) -> Result<InvariantDrift, AnalysisError> {
    if !initial.is_finite() || !final_value.is_finite() {
        return Err(AnalysisError::NonFinite);
    }
    let absolute_drift = (final_value - initial).abs();
    Ok(InvariantDrift {
        initial,
        final_value,
        absolute_drift,
        relative_drift: absolute_drift / initial.abs().max(f64::MIN_POSITIVE),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oscillator_energy(state: PhaseState<1>) -> f64 {
        0.5 * (state.q.data[0].powi(2) + state.p.data[0].powi(2))
    }

    #[test]
    fn poisson_bracket_is_antisymmetric() {
        let fq = Vector::new([1.0, 2.0]);
        let fp = Vector::new([3.0, 4.0]);
        let gq = Vector::new([-2.0, 1.0]);
        let gp = Vector::new([0.5, 3.0]);
        let fg = canonical_poisson_bracket(fq, fp, gq, gp).unwrap();
        let gf = canonical_poisson_bracket(gq, gp, fq, fp).unwrap();
        assert_eq!(fg, -gf);
    }

    #[test]
    fn stormer_verlet_is_time_reversible_and_bounds_energy_drift() {
        let initial = PhaseState {
            q: Vector::new([1.0]),
            p: Vector::new([0.0]),
        };
        let mut state = initial;
        for _ in 0..10_000 {
            stormer_verlet_step(&mut state, 0.01, |q| q, |p| p).unwrap();
        }
        let drift = invariant_drift(oscillator_energy(initial), oscillator_energy(state)).unwrap();
        assert!(drift.absolute_drift < 2e-5);

        for _ in 0..10_000 {
            stormer_verlet_step(&mut state, -0.01, |q| q, |p| p).unwrap();
        }
        assert!(state.q.distance(initial.q).unwrap() < 1e-12);
        assert!(state.p.distance(initial.p).unwrap() < 1e-12);
    }
}
