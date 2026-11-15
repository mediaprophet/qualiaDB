//! Hydrodynamic & Geomorphological Homogenization (Kozeny-Carman, Exner, van Genuchten).
//!
//! Evaluates hydraulic conductivity, riverbed sediment deposition/erosion, and unsaturated capillary flow.

/// Evaluates intrinsic permeability $k$ in m² using the semi-empirical Kozeny-Carman equation.
///
/// $k = \frac{d_{\text{eff}}^2}{180}\frac{\phi^3}{(1 - \phi)^2}$
///
/// `d_eff_meters`: Effective harmonic mean particle grain diameter in meters.
/// `porosity_phi`: Total connected volumetric porosity $\phi \in (0, 1)$.
pub fn kozeny_carman_permeability_m2(d_eff_meters: f64, porosity_phi: f64) -> Option<f64> {
    if d_eff_meters <= 0.0 || porosity_phi <= 0.0 || porosity_phi >= 1.0 {
        return None;
    }

    let phi_3 = porosity_phi * porosity_phi * porosity_phi;
    let one_minus_phi = 1.0 - porosity_phi;
    let denom = 180.0 * (one_minus_phi * one_minus_phi);

    Some((d_eff_meters * d_eff_meters * phi_3) / denom)
}

/// Evaluates the Exner sediment conservation equation to update riverbed/terrain elevation.
///
/// $(1 - \lambda_p)\frac{\partial z_{\text{bed}}}{\partial t} = -\nabla \cdot \mathbf{q}_s$
///
/// `dt_seconds`: Simulation time delta $\Delta t$ in seconds.
/// `porosity_lambda_p`: Bed deposit porosity $\lambda_p \in [0, 1)$.
/// `div_qs`: Sediment volumetric flux divergence $\nabla \cdot \mathbf{q}_s = \frac{\partial q_{sx}}{\partial x} + \frac{\partial q_{sy}}{\partial y}$ in m/s.
pub fn exner_elevation_change_m(
    dt_seconds: f64,
    porosity_lambda_p: f64,
    div_qs: f64,
) -> Option<f64> {
    if porosity_lambda_p < 0.0 || porosity_lambda_p >= 1.0 || dt_seconds < 0.0 {
        return None;
    }

    let one_minus_lambda = 1.0 - porosity_lambda_p;
    Some(-dt_seconds * div_qs / one_minus_lambda)
}

/// Evaluates effective water saturation $S_e \in [0, 1]$ using the van Genuchten empirical formulation.
///
/// $S_e(\psi) = \left[1 + (\alpha |\psi|)^n\right]^{-(1 - 1/n)}$
///
/// `suction_head_psi_meters`: Matric suction head $|\psi|$ in meters of water column.
/// `alpha_inv_meters`: van Genuchten parameter $\alpha$ (inverse air-entry pressure) in 1/m.
/// `n_exponent`: van Genuchten pore-size distribution index $n > 1$.
pub fn van_genuchten_effective_saturation(
    suction_head_psi_meters: f64,
    alpha_inv_meters: f64,
    n_exponent: f64,
) -> Option<f64> {
    if suction_head_psi_meters < 0.0 || alpha_inv_meters <= 0.0 || n_exponent <= 1.0 {
        return None;
    }

    let m = 1.0 - (1.0 / n_exponent);
    let term = 1.0 + (alpha_inv_meters * suction_head_psi_meters).powf(n_exponent);
    Some(term.powf(-m))
}
