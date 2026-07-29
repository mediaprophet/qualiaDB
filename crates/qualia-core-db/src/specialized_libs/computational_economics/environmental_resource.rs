//! Environmental and resource economics: carbon/social cost, pollution
//! damage, optimal extraction under stock externality, and cost-benefit of
//! abatement.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Carbon/social cost of carbon is a linear marginal damage function.
//! - Optimal pollution satisfies marginal abatement cost = marginal damage.
//! - Extraction with stock externality: higher cumulative extraction raises
//!   marginal extraction cost.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentalError {
    InvalidInput,
    NonFinite,
    InsufficientData,
    BufferTooSmall,
}

fn require_finite(x: f64) -> Result<(), EnvironmentalError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(EnvironmentalError::NonFinite)
    }
}

/// Social cost of carbon: `SCC = damage_per_ton * emissions_tons`.
///
/// `damage_per_ton` is the marginal damage per ton of CO2 equivalent.
pub fn social_cost_of_carbon(
    emissions_tons: f64,
    damage_per_ton: f64,
) -> Result<f64, EnvironmentalError> {
    require_finite(emissions_tons)?;
    require_finite(damage_per_ton)?;
    if emissions_tons < 0.0 || damage_per_ton < 0.0 {
        return Err(EnvironmentalError::InvalidInput);
    }
    Ok(emissions_tons * damage_per_ton)
}

/// Total pollution damage: `D = 0.5 * damage_coeff * emissions^2`.
///
/// Quadratic damage function (standard in environmental economics).
pub fn pollution_damage(emissions: f64, damage_coeff: f64) -> Result<f64, EnvironmentalError> {
    require_finite(emissions)?;
    require_finite(damage_coeff)?;
    if emissions < 0.0 || damage_coeff < 0.0 {
        return Err(EnvironmentalError::InvalidInput);
    }
    Ok(0.5 * damage_coeff * emissions * emissions)
}

/// Marginal damage: `MD = damage_coeff * emissions`.
pub fn marginal_damage(emissions: f64, damage_coeff: f64) -> Result<f64, EnvironmentalError> {
    require_finite(emissions)?;
    require_finite(damage_coeff)?;
    if emissions < 0.0 || damage_coeff < 0.0 {
        return Err(EnvironmentalError::InvalidInput);
    }
    Ok(damage_coeff * emissions)
}

/// Optimal pollution level: where marginal abatement cost = marginal damage.
///
/// Abatement cost: `AC = 0.5 * abatement_coeff * (E0 - E)^2` where `E0` is
/// baseline emissions and `E` is actual emissions. Marginal abatement cost:
/// `MAC = abatement_coeff * (E0 - E)`. Setting `MAC = MD`:
/// `abatement_coeff * (E0 - E) = damage_coeff * E`
/// → `E* = abatement_coeff * E0 / (abatement_coeff + damage_coeff)`.
pub fn optimal_pollution(
    baseline_emissions: f64,
    abatement_coeff: f64,
    damage_coeff: f64,
) -> Result<f64, EnvironmentalError> {
    require_finite(baseline_emissions)?;
    require_finite(abatement_coeff)?;
    require_finite(damage_coeff)?;
    if baseline_emissions < 0.0 || abatement_coeff <= 0.0 || damage_coeff <= 0.0 {
        return Err(EnvironmentalError::InvalidInput);
    }
    Ok(abatement_coeff * baseline_emissions / (abatement_coeff + damage_coeff))
}

/// Optimal abatement level: `A* = E0 - E*`.
pub fn optimal_abatement(
    baseline_emissions: f64,
    abatement_coeff: f64,
    damage_coeff: f64,
) -> Result<f64, EnvironmentalError> {
    let e_star = optimal_pollution(baseline_emissions, abatement_coeff, damage_coeff)?;
    Ok(baseline_emissions - e_star)
}

/// Cost-benefit of abatement: net benefit = avoided damage - abatement cost.
///
/// `NB = 0.5 * damage_coeff * (E0^2 - E^2) - 0.5 * abatement_coeff * (E0 - E)^2`.
pub fn abatement_net_benefit(
    baseline_emissions: f64,
    actual_emissions: f64,
    abatement_coeff: f64,
    damage_coeff: f64,
) -> Result<f64, EnvironmentalError> {
    require_finite(baseline_emissions)?;
    require_finite(actual_emissions)?;
    if baseline_emissions < 0.0 || actual_emissions < 0.0 || actual_emissions > baseline_emissions {
        return Err(EnvironmentalError::InvalidInput);
    }
    let avoided_damage =
        0.5 * damage_coeff * (baseline_emissions.powi(2) - actual_emissions.powi(2));
    let abatement_cost = 0.5 * abatement_coeff * (baseline_emissions - actual_emissions).powi(2);
    Ok(avoided_damage - abatement_cost)
}

/// Hotelling extraction path with stock externality: marginal extraction cost
/// rises with cumulative extraction.
///
/// `cost_t = c0 + externality_coeff * cumulative_t`. Extraction declines as
/// cost rises. Simple myopic rule: extract while price > marginal cost.
/// Writes extraction quantities into `out[..n_periods]`.
pub fn extraction_with_externality_into(
    initial_stock: f64,
    price: f64,
    base_cost: f64,
    externality_coeff: f64,
    n_periods: usize,
    out: &mut [f64],
) -> Result<usize, EnvironmentalError> {
    if initial_stock <= 0.0 || price <= 0.0 || n_periods == 0 || out.len() < n_periods {
        return Err(EnvironmentalError::InvalidInput);
    }
    require_finite(initial_stock)?;
    require_finite(price)?;
    require_finite(base_cost)?;
    require_finite(externality_coeff)?;
    if base_cost < 0.0 || externality_coeff < 0.0 {
        return Err(EnvironmentalError::InvalidInput);
    }
    let mut remaining = initial_stock;
    let mut cumulative = 0.0;
    for t in 0..n_periods {
        out[t] = 0.0;
        if remaining <= 0.0 {
            continue;
        }
        let marginal_cost = base_cost + externality_coeff * cumulative;
        if price > marginal_cost {
            // Myopic: extract a fixed fraction of remaining.
            let extract = (remaining / (n_periods - t) as f64).min(remaining);
            out[t] = extract;
            remaining -= extract;
            cumulative += extract;
        }
    }
    Ok(n_periods)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn social_cost_basic() {
        let scc = social_cost_of_carbon(100.0, 50.0).unwrap();
        assert!(approx(scc, 5000.0, 1e-9));
    }

    #[test]
    fn pollution_damage_quadratic() {
        // D = 0.5 * 2 * 10^2 = 100
        let d = pollution_damage(10.0, 2.0).unwrap();
        assert!(approx(d, 100.0, 1e-9));
    }

    #[test]
    fn marginal_damage_linear() {
        // MD = 2 * 10 = 20
        let md = marginal_damage(10.0, 2.0).unwrap();
        assert!(approx(md, 20.0, 1e-9));
    }

    #[test]
    fn optimal_pollution_formula() {
        // E0=100, abatement=2, damage=3 → E* = 2*100/(2+3) = 40
        let e = optimal_pollution(100.0, 2.0, 3.0).unwrap();
        assert!(approx(e, 40.0, 1e-9));
    }

    #[test]
    fn optimal_abatement_level() {
        // E0=100, E*=40 → A*=60
        let a = optimal_abatement(100.0, 2.0, 3.0).unwrap();
        assert!(approx(a, 60.0, 1e-9));
    }

    #[test]
    fn abatement_net_benefit_positive() {
        // At optimal: NB should be positive.
        let e_star = optimal_pollution(100.0, 2.0, 3.0).unwrap();
        let nb = abatement_net_benefit(100.0, e_star, 2.0, 3.0).unwrap();
        assert!(nb > 0.0);
    }

    #[test]
    fn abatement_net_benefit_zero_at_baseline() {
        // No abatement → NB = 0
        let nb = abatement_net_benefit(100.0, 100.0, 2.0, 3.0).unwrap();
        assert!(approx(nb, 0.0, 1e-9));
    }

    #[test]
    fn extraction_with_externality_depletes() {
        let mut path = [0.0f64; 10];
        extraction_with_externality_into(100.0, 10.0, 1.0, 0.01, 10, &mut path).unwrap();
        let total: f64 = path.iter().sum();
        assert!(total <= 100.0 + 1e-6);
        assert!(total > 0.0);
    }

    #[test]
    fn invalid_inputs_rejected() {
        assert_eq!(
            social_cost_of_carbon(-1.0, 50.0).unwrap_err(),
            EnvironmentalError::InvalidInput
        );
        assert_eq!(
            optimal_pollution(-1.0, 2.0, 3.0).unwrap_err(),
            EnvironmentalError::InvalidInput
        );
    }
}
