//! Input-output and supply-shock propagation.

/// Maximum sectors in a bounded input-output (Leontief) model.
pub const MAX_SECTORS: usize = 32;

/// Propagate a supply/geopolitical shock through an inter-sector input-output
/// (Leontief) coupling matrix to its total downstream impact.
pub fn propagate_supply_shock(
    coupling: &[f64],
    shock: &[f64],
    n: usize,
    max_rounds: u32,
    tolerance: f64,
    impact_out: &mut [f64],
) -> usize {
    if n == 0
        || n > MAX_SECTORS
        || coupling.len() < n * n
        || shock.len() < n
        || impact_out.len() < n
    {
        return 0;
    }
    let mut term = [0.0f64; MAX_SECTORS];
    let mut next = [0.0f64; MAX_SECTORS];
    for i in 0..n {
        term[i] = shock[i];
        impact_out[i] = shock[i];
    }
    let mut rounds = 0usize;
    for _ in 0..max_rounds {
        rounds += 1;
        let mut l1 = 0.0f64;
        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += coupling[i * n + j] * term[j];
            }
            next[i] = acc;
            l1 += acc.abs();
        }
        for i in 0..n {
            impact_out[i] += next[i];
            term[i] = next[i];
        }
        if l1 < tolerance {
            break;
        }
    }
    rounds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_shock_propagates_to_dependent_sectors() {
        let a = [0.0, 0.5, 0.5, 0.0];
        let shock = [1.0, 0.0];
        let mut impact = [0.0f64; 2];
        let rounds = propagate_supply_shock(&a, &shock, 2, 100, 1e-9, &mut impact);
        assert!(rounds > 1);
        assert!((impact[0] - 4.0 / 3.0).abs() < 1e-6);
        assert!(impact[1] > 0.6 && impact[1] < 0.7);
    }

    #[test]
    fn supply_shock_rejects_bad_dimensions() {
        let mut out = [0.0f64; 2];
        assert_eq!(
            propagate_supply_shock(&[0.0], &[1.0], 0, 10, 1e-9, &mut out),
            0
        );
        assert_eq!(
            propagate_supply_shock(&[0.0; 4], &[1.0, 0.0], 3, 10, 1e-9, &mut out),
            0
        );
    }
}
