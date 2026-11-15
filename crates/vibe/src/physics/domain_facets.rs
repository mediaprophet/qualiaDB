//! Cross-Domain Constitutive Facets and Bond-Graph Ports.
//!
//! Provides biomechanical (Hill muscle), botanical/hydraulic (Allometric branch),
//! mechanical structural (Fatigue & corrosion), and port-Hamiltonian energy
//! conservation primitives for VibeScript physics.
//!
//! Reference: `AGENTS.md` Workstream W5 (§6.1–§6.2).

use std::f64::consts::PI;

/// Biomechanical muscle actuator facet based on Hill's three-element muscle model.
///
/// $$F_{\text{muscle}}(t) = F_{\max}\left[a(t)\cdot f_L(\tilde{l})\cdot f_V(\tilde{v}) + f_P(\tilde{l})\right]\cos\alpha_{\text{penn}}$$
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HillMuscleFacet {
    /// Maximum isometric force at optimal fiber length (N).
    pub max_isometric_force_n: f64,
    /// Optimal muscle fiber length $L_0$ (m).
    pub optimal_fiber_length_m: f64,
    /// Elastic tendon slack length $L_s^T$ (m).
    pub tendon_slack_length_m: f64,
    /// Maximum contraction velocity normalized to $L_0$ ($s^{-1}$).
    pub max_contraction_velocity_l0_s: f64,
    /// Muscle fiber pennation angle with respect to tendon line of action (radians).
    pub pennation_angle_rad: f64,
}

impl HillMuscleFacet {
    /// Constructs a new Hill muscle facet.
    pub const fn new(
        max_isometric_force_n: f64,
        optimal_fiber_length_m: f64,
        tendon_slack_length_m: f64,
        max_contraction_velocity_l0_s: f64,
        pennation_angle_rad: f64,
    ) -> Self {
        Self {
            max_isometric_force_n,
            optimal_fiber_length_m,
            tendon_slack_length_m,
            max_contraction_velocity_l0_s,
            pennation_angle_rad,
        }
    }

    /// Evaluates the active normalized force-length relationship $f_L(\tilde{l})$.
    ///
    /// $$f_L(\tilde{l}) = \exp\left( - \left(\frac{\tilde{l} - 1.0}{0.45}\right)^2 \right)$$
    #[inline]
    pub fn active_force_length(l_norm: f64) -> f64 {
        let delta = (l_norm - 1.0) / 0.45;
        (-delta * delta).exp()
    }

    /// Evaluates the normalized force-velocity relationship $f_V(\tilde{v})$.
    ///
    /// Uses Hill's hyperbola for concentric shortening ($\tilde{v} < 0$) and an
    /// eccentric plateau saturating up to 1.4 for lengthening ($\tilde{v} > 0$).
    #[inline]
    pub fn force_velocity(v_norm: f64) -> f64 {
        if v_norm <= -1.0 {
            0.0
        } else if v_norm < 0.0 {
            // Concentric shortening: Hill hyperbola (c = 0.25)
            (1.0 + v_norm) / (1.0 - v_norm / 0.25)
        } else {
            // Eccentric lengthening: asymptotic plateau up to 1.4
            1.0 + 0.4 * (v_norm / (v_norm + 0.1))
        }
    }

    /// Evaluates the passive normalized force-length relationship $f_P(\tilde{l})$.
    ///
    /// Exponential tension curve engaged when fiber length exceeds optimal length ($\tilde{l} > 1.0$).
    #[inline]
    pub fn passive_force_length(l_norm: f64) -> f64 {
        if l_norm <= 1.0 {
            0.0
        } else {
            let dl = l_norm - 1.0;
            (4.0 * dl).exp() - 1.0
        }
    }

    /// Computes total muscle force transmitted to the tendon (N).
    pub fn compute_tendon_force(
        &self,
        activation: f64,
        fiber_length_m: f64,
        fiber_velocity_m_s: f64,
    ) -> f64 {
        let a = activation.clamp(0.0, 1.0);
        let l_opt = self.optimal_fiber_length_m.max(1e-9);
        let l_norm = fiber_length_m / l_opt;
        let v_max = self.max_contraction_velocity_l0_s * l_opt;
        let v_norm = fiber_velocity_m_s / v_max.max(1e-9);

        let f_l = Self::active_force_length(l_norm);
        let f_v = Self::force_velocity(v_norm);
        let f_p = Self::passive_force_length(l_norm);

        let active_passive = (a * f_l * f_v + f_p).max(0.0);
        let force = self.max_isometric_force_n * active_passive * self.pennation_angle_rad.cos();
        force.max(0.0)
    }
}

/// Botanical morphogenesis allometric scaling laws and xylem transpiration hydraulics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllometricBranchLaw {
    /// Allometric branching exponent $\gamma \in [2.0, 3.0]$ (Murray's Law default: 2.49).
    pub gamma: f64,
    /// West-Brown-Enquist (WBE) radius scaling exponent $a \approx 1/3$.
    pub wbe_radius_exp: f64,
    /// West-Brown-Enquist (WBE) length scaling exponent $b \approx 2/3$.
    pub wbe_length_exp: f64,
}

impl Default for AllometricBranchLaw {
    fn default() -> Self {
        Self {
            gamma: 2.49,
            wbe_radius_exp: 1.0 / 3.0,
            wbe_length_exp: 2.0 / 3.0,
        }
    }
}

impl AllometricBranchLaw {
    /// Computes child branch radii given a parent radius and split ratio $\eta \in [0.0, 1.0]$.
    ///
    /// $$r_{\text{parent}}^\gamma = r_{\text{child},1}^\gamma + r_{\text{child},2}^\gamma$$
    pub fn child_radii_from_split(&self, parent_radius: f64, split_ratio: f64) -> (f64, f64) {
        let eta = split_ratio.clamp(0.0, 1.0);
        let inv_gamma = 1.0 / self.gamma.max(1e-6);
        let r_p = parent_radius.max(0.0);
        let r1 = r_p * eta.powf(inv_gamma);
        let r2 = r_p * (1.0 - eta).powf(inv_gamma);
        (r1, r2)
    }

    /// Verifies Da Vinci / Murray conservation residual: $|r_{\text{parent}}^\gamma - \sum r_{\text{child}}^\gamma|$.
    pub fn verify_da_vinci_conservation(&self, parent_radius: f64, child_radii: &[f64]) -> f64 {
        let sum_children: f64 = child_radii.iter().map(|&r| r.max(0.0).powf(self.gamma)).sum();
        let parent_pow = parent_radius.max(0.0).powf(self.gamma);
        (parent_pow - sum_children).abs()
    }

    /// Evaluates golden-angle phyllotaxis direction vector for primordium / leaf node $n$.
    ///
    /// $$\theta_n = n \cdot 137.50776405^\circ, \quad r_n = c \sqrt{n}$$
    pub fn phyllotaxis_direction(index: u32, scale_c: f64) -> [f64; 3] {
        let n_f = index as f64;
        let golden_angle_rad = 137.50776405 * (PI / 180.0);
        let theta = n_f * golden_angle_rad;
        let r = scale_c * n_f.sqrt();
        [r * theta.cos(), r * theta.sin(), 0.0]
    }

    /// Evaluates Darcy-Hagen-Poiseuille xylem transpiration flux density ($m \cdot s^{-1}$).
    ///
    /// $$J_w = -K_h \left(\frac{\Delta\Psi}{\Delta z} + \rho g\right)$$
    pub fn darcy_transpiration_flux(
        conductance_k: f64,
        psi_soil_mpa: f64,
        psi_leaf_mpa: f64,
        height_m: f64,
    ) -> f64 {
        let dz = height_m.max(1e-4);
        let delta_psi_pa = (psi_leaf_mpa - psi_soil_mpa) * 1e6;
        let rho_g = 1000.0 * 9.80665; // Pa / m
        let gradient = (delta_psi_pa / dz) + rho_g;
        -conductance_k * gradient
    }
}

/// Mechanical structural fatigue and galvanic corrosion constitutive law.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FatigueCorrosionLaw {
    /// Fatigue strength coefficient $\sigma_f'$ (MPa).
    pub fatigue_strength_coeff_mpa: f64,
    /// Basquin fatigue exponent $b$ (typically $-0.05$ to $-0.15$).
    pub basquin_exponent_b: f64,
    /// Paris-Erdogan crack growth coefficient $C$ ($m \cdot \text{cycle}^{-1} \cdot (\text{MPa}\sqrt{m})^{-m}$).
    pub paris_coefficient_c: f64,
    /// Paris-Erdogan crack growth exponent $m$ (typically $2.0$ to $4.0$).
    pub paris_exponent_m: f64,
}

impl FatigueCorrosionLaw {
    /// Constructs a new fatigue and corrosion law instance.
    pub const fn new(
        fatigue_strength_coeff_mpa: f64,
        basquin_exponent_b: f64,
        paris_coefficient_c: f64,
        paris_exponent_m: f64,
    ) -> Self {
        Self {
            fatigue_strength_coeff_mpa,
            basquin_exponent_b,
            paris_coefficient_c,
            paris_exponent_m,
        }
    }

    /// Evaluates Basquin S-N high-cycle fatigue cycles to failure $N_f$.
    ///
    /// $$\sigma_a = \sigma_f' (2 N_f)^b \implies N_f = \frac{1}{2}\left(\frac{\sigma_a}{\sigma_f'}\right)^{1/b}$$
    pub fn cycles_to_failure_basquin(&self, stress_amplitude_mpa: f64) -> f64 {
        if stress_amplitude_mpa <= 0.0 {
            return f64::INFINITY;
        }
        let ratio = stress_amplitude_mpa / self.fatigue_strength_coeff_mpa.max(1e-9);
        let inv_b = 1.0 / self.basquin_exponent_b;
        0.5 * ratio.powf(inv_b)
    }

    /// Evaluates Paris-Erdogan subcritical crack growth per cycle $\frac{da}{dN}$ (m/cycle).
    ///
    /// $$\frac{da}{dN} = C (\Delta K)^m$$
    pub fn crack_growth_per_cycle(&self, delta_k_mpa_sqrt_m: f64) -> f64 {
        if delta_k_mpa_sqrt_m <= 0.0 {
            0.0
        } else {
            self.paris_coefficient_c * delta_k_mpa_sqrt_m.powf(self.paris_exponent_m)
        }
    }

    /// Evaluates Faraday-Arrhenius galvanic corrosion mass loss rate (kg/s).
    ///
    /// $$\frac{dm}{dt} = \frac{j \cdot A \cdot M}{z F} \cdot \exp\left(-\frac{E_a}{R}\left(\frac{1}{T} - \frac{1}{T_0}\right)\right)$$
    pub fn corrosion_mass_loss_rate_kg_s(
        &self,
        current_density_a_m2: f64,
        area_m2: f64,
        molar_mass_kg_mol: f64,
        valence_z: f64,
        temp_k: f64,
    ) -> f64 {
        const FARADAY_C_PER_MOL: f64 = 96485.33212;
        const GAS_CONSTANT_R: f64 = 8.314462618;
        const ACTIVATION_ENERGY_J_PER_MOL: f64 = 35000.0;
        const T_REF_K: f64 = 298.15;

        let current_a = current_density_a_m2.max(0.0) * area_m2.max(0.0);
        let z = valence_z.max(1.0);
        let faraday_rate = (current_a * molar_mass_kg_mol.max(0.0)) / (z * FARADAY_C_PER_MOL);

        let t = temp_k.max(1.0);
        let arrhenius =
            (-(ACTIVATION_ENERGY_J_PER_MOL / GAS_CONSTANT_R) * (1.0 / t - 1.0 / T_REF_K)).exp();
        faraday_rate * arrhenius
    }
}

/// Physical energy domain of a port-Hamiltonian bond graph port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDomain {
    MechanicalTranslational,
    MechanicalRotational,
    Electrical,
    Hydraulic,
    Thermal,
    Chemical,
}

/// Bond-graph energy-conjugated port ($e, f$) for port-Hamiltonian conservation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BondGraphPort {
    pub domain: PortDomain,
    pub effort: f64,
    pub flow: f64,
}

impl BondGraphPort {
    /// Constructs a new bond-graph energy port.
    pub const fn new(domain: PortDomain, effort: f64, flow: f64) -> Self {
        Self {
            domain,
            effort,
            flow,
        }
    }

    /// Evaluates the instantaneous energy power flow $P = e \cdot f$ (Watts).
    #[inline]
    pub fn instantaneous_power(&self) -> f64 {
        self.effort * self.flow
    }

    /// Verifies Dirac structure junction power conservation: $\sum_k e_k f_k = 0$.
    pub fn verify_dirac_junction_power_balance(
        ports: &[BondGraphPort],
        tolerance: f64,
    ) -> Result<f64, f64> {
        let net_power: f64 = ports.iter().map(|p| p.instantaneous_power()).sum();
        if net_power.abs() <= tolerance.abs() {
            Ok(net_power)
        } else {
            Err(net_power)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hill_muscle_facet_mechanics() {
        let muscle = HillMuscleFacet::new(1000.0, 0.1, 0.2, 10.0, 0.0);

        // Optimal fiber length at rest should give max active force
        let f_opt = HillMuscleFacet::active_force_length(1.0);
        assert!((f_opt - 1.0).abs() < 1e-6);

        // Isometric active contraction at L0
        let force_isometric = muscle.compute_tendon_force(1.0, 0.1, 0.0);
        assert!((force_isometric - 1000.0).abs() < 1e-4);

        // Concentric shortening reduces active force
        let force_shortening = muscle.compute_tendon_force(1.0, 0.1, -0.5);
        assert!(force_shortening < force_isometric);

        // Eccentric lengthening increases force up to plateau
        let force_eccentric = muscle.compute_tendon_force(1.0, 0.1, 0.5);
        assert!(force_eccentric > force_isometric);
    }

    #[test]
    fn test_allometric_branching_and_da_vinci() {
        let law = AllometricBranchLaw::default();
        let (r1, r2) = law.child_radii_from_split(0.10, 0.5);
        let res = law.verify_da_vinci_conservation(0.10, &[r1, r2]);
        assert!(res < 1e-9);

        let phyll = AllometricBranchLaw::phyllotaxis_direction(5, 0.02);
        assert!(phyll[0] != 0.0 || phyll[1] != 0.0);
    }

    #[test]
    fn test_fatigue_and_corrosion() {
        let law = FatigueCorrosionLaw::new(800.0, -0.10, 1e-11, 3.0);
        let cycles = law.cycles_to_failure_basquin(400.0);
        assert!(cycles > 1e2);

        let crack_rate = law.crack_growth_per_cycle(25.0);
        assert!(crack_rate > 0.0);

        let loss_rate = law.corrosion_mass_loss_rate_kg_s(10.0, 0.05, 0.055845, 2.0, 298.15);
        assert!(loss_rate > 0.0);
    }

    #[test]
    fn test_bond_graph_dirac_junction() {
        let p_in = BondGraphPort::new(PortDomain::Electrical, 100.0, 2.0); // +200 W
        let p_mech = BondGraphPort::new(PortDomain::MechanicalTranslational, 50.0, -4.0); // -200 W
        let ports = [p_in, p_mech];

        let balance = BondGraphPort::verify_dirac_junction_power_balance(&ports, 1e-6);
        assert!(balance.is_ok());
    }
}
