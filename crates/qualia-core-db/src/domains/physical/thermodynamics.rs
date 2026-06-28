//! Thermodynamics & Statistical Ensembles
//! Implements pure Rust Markov Chain Monte Carlo (MCMC) sampling for macroscopic properties.

/// State of a thermodynamic ensemble
#[derive(Clone)]
pub struct EnsembleState {
    pub temperature: f64,
    pub particles: usize,
    pub total_energy: f64,
}

/// Computes thermodynamic macroscopic properties from discrete structures via MCMC.
pub struct ThermodynamicSampler {
    pub current_state: EnsembleState,
}

impl ThermodynamicSampler {
    pub fn new(initial_temp: f64, particles: usize) -> Self {
        Self {
            current_state: EnsembleState {
                temperature: initial_temp,
                particles,
                total_energy: 0.0,
            },
        }
    }

    /// Performs a Metropolis-Hastings MCMC step
    pub fn metropolis_step(&mut self, proposed_energy: f64, random_uniform: f64) -> bool {
        let delta_e = proposed_energy - self.current_state.total_energy;

        // Accept if energy decreases or strictly probabilistically according to Boltzmann distribution
        let k_b = 8.617333262145e-5; // Boltzmann constant in eV/K
        let beta = 1.0 / (k_b * self.current_state.temperature);

        let acceptance_probability = if delta_e < 0.0 {
            1.0
        } else {
            (-beta * delta_e).exp()
        };

        if random_uniform < acceptance_probability {
            self.current_state.total_energy = proposed_energy;
            true // Accepted
        } else {
            false // Rejected
        }
    }

    /// Calculates macroscopic Gibbs Free Energy approximation
    pub fn calculate_gibbs_free_energy(&self, enthalpy: f64, entropy: f64) -> f64 {
        // G = H - TS
        enthalpy - (self.current_state.temperature * entropy)
    }
}

// ─── Off-grid energy: battery / solar / heat-transfer models ─────────────────────
//
// The resilience-energy scope: rigorous thermal/electrical state for off-grid energy
// storage (lithium banks under fluctuating loads), dynamic solar harvesting (MPP
// tracking across mixed arrays), and heat-transfer efficiency in constrained/mobile
// infrastructure. All zero-heap (pure-scalar equivalent-circuit / I-V / U·A·ΔT models).

/// An off-grid lithium pack as an `S`×`P` array of prismatic cells, modelled as a
/// first-order equivalent circuit (OCV − I·R sag). Captures the "down to the internal
/// 4-prismatic-cell architecture" requirement: terminal voltage and deliverable power
/// depend on the series/parallel topology, per-cell internal resistance, and SoC under
/// a fluctuating load.
#[derive(Debug, Clone, Copy)]
pub struct LithiumPack {
    pub cells_series: u32,
    pub cells_parallel: u32,
    pub cell_internal_resistance_ohm: f64,
    pub cell_capacity_ah: f64,
}

impl LithiumPack {
    /// Open-circuit voltage of ONE cell at a state-of-charge (0..1): a linearised
    /// Li-ion curve from ~3.0 V (empty) to ~4.2 V (full).
    pub fn cell_ocv(&self, soc: f64) -> f64 {
        3.0 + (4.2 - 3.0) * soc.clamp(0.0, 1.0)
    }

    /// Pack open-circuit voltage = series count × cell OCV.
    pub fn pack_ocv(&self, soc: f64) -> f64 {
        self.cells_series as f64 * self.cell_ocv(soc)
    }

    /// Pack internal resistance = R_cell × S / P (series adds, parallel divides).
    pub fn pack_resistance(&self) -> f64 {
        self.cell_internal_resistance_ohm * self.cells_series as f64
            / self.cells_parallel.max(1) as f64
    }

    /// Terminal voltage under a load current (A): OCV minus the internal-resistance sag.
    pub fn terminal_voltage(&self, soc: f64, load_current_a: f64) -> f64 {
        self.pack_ocv(soc) - load_current_a * self.pack_resistance()
    }

    /// Power (W) actually delivered to the load at a given SoC and current.
    pub fn deliverable_power(&self, soc: f64, load_current_a: f64) -> f64 {
        self.terminal_voltage(soc, load_current_a).max(0.0) * load_current_a
    }

    /// Total pack capacity (Ah) = per-cell capacity × parallel count.
    pub fn pack_capacity_ah(&self) -> f64 {
        self.cell_capacity_ah * self.cells_parallel as f64
    }
}

/// A solar panel on a simplified single-knee I-V curve (`I_sc` scales with irradiance;
/// `fill_factor` sets the knee sharpness ∝ panel/impedance quality).
#[derive(Debug, Clone, Copy)]
pub struct SolarPanel {
    pub short_circuit_current_a: f64,
    pub open_circuit_voltage_v: f64,
    pub fill_factor: f64,
}

impl SolarPanel {
    /// Current (A) at an operating voltage on the I-V curve `I = I_sc·(1 − (V/V_oc)^p)`,
    /// with `p` derived from the fill factor (higher FF ⇒ squarer knee).
    pub fn current_at(&self, v: f64) -> f64 {
        if v <= 0.0 {
            return self.short_circuit_current_a;
        }
        if v >= self.open_circuit_voltage_v {
            return 0.0;
        }
        let p = (1.0 / (1.0 - self.fill_factor.clamp(0.05, 0.95))).max(1.0);
        self.short_circuit_current_a * (1.0 - (v / self.open_circuit_voltage_v).powf(p))
    }

    /// Maximum power point: scan the I-V curve for the voltage maximising `P = V·I`.
    /// Returns `(v_mp, i_mp, p_mp)` — the dynamic-harvesting operating point an MPPT
    /// controller would seek as irradiance/impedance shift.
    pub fn max_power_point(&self, scan_steps: u32) -> (f64, f64, f64) {
        let mut best = (0.0f64, 0.0f64, 0.0f64);
        let n = scan_steps.max(2);
        for k in 1..n {
            let v = self.open_circuit_voltage_v * k as f64 / n as f64;
            let i = self.current_at(v);
            let p = v * i;
            if p > best.2 {
                best = (v, i, p);
            }
        }
        best
    }
}

/// Total MPP power (W) harvested from a mixed-topology array — the sum of each panel's
/// independently-tracked maximum power point. Zero-heap (slice in, scalar out).
pub fn array_mppt_power(panels: &[SolarPanel], scan_steps: u32) -> f64 {
    panels.iter().map(|p| p.max_power_point(scan_steps).2).sum()
}

/// Conductive/convective heat-loss rate (W) through an envelope: `Q = U·A·ΔT`.
pub fn heat_loss_rate(u_value_w_m2k: f64, area_m2: f64, delta_t_k: f64) -> f64 {
    u_value_w_m2k * area_m2 * delta_t_k
}

/// Latent heat (J) of a phase change for `mass_kg` at specific latent heat
/// `latent_heat_j_kg` — the multi-phase (boiling/condensing/melting) energy term that
/// sits alongside the sensible `Q = U·A·ΔT` loss.
pub fn phase_change_energy(mass_kg: f64, latent_heat_j_kg: f64) -> f64 {
    mass_kg * latent_heat_j_kg
}

/// Thermal efficiency of a heating/cooling system delivering `useful_power_w` against
/// an envelope loss `U·A·ΔT`: `η = useful / (useful + loss)`, in `0..1`. Models the
/// efficiency drop-off in constrained / mobile / pop-up infrastructure (thin envelope
/// ⇒ high U ⇒ low η).
pub fn thermal_efficiency(
    useful_power_w: f64,
    u_value_w_m2k: f64,
    area_m2: f64,
    delta_t_k: f64,
) -> f64 {
    let loss = heat_loss_rate(u_value_w_m2k, area_m2, delta_t_k).abs();
    let useful = useful_power_w.max(0.0);
    if useful + loss <= 0.0 {
        return 0.0;
    }
    useful / (useful + loss)
}

#[cfg(test)]
mod offgrid_energy_tests {
    use super::*;

    #[test]
    fn lithium_pack_sags_under_load() {
        // 4S2P pack, 5 mΩ per cell, 100 Ah cells.
        let pack = LithiumPack {
            cells_series: 4,
            cells_parallel: 2,
            cell_internal_resistance_ohm: 0.005,
            cell_capacity_ah: 100.0,
        };
        assert!((pack.pack_ocv(1.0) - 16.8).abs() < 1e-9); // 4 × 4.2
        assert!((pack.pack_capacity_ah() - 200.0).abs() < 1e-9); // 100 × 2
        let rest = pack.terminal_voltage(0.5, 0.0);
        let loaded = pack.terminal_voltage(0.5, 50.0);
        assert!(loaded < rest, "terminal voltage must sag under load");
        // sag = I·R_pack = 50 × (0.005×4/2) = 0.5 V
        assert!((rest - loaded - 0.5).abs() < 1e-9);
        assert!(pack.deliverable_power(0.5, 50.0) > 0.0);
    }

    #[test]
    fn solar_mpp_is_between_zero_and_isc_voc() {
        let panel = SolarPanel {
            short_circuit_current_a: 8.0,
            open_circuit_voltage_v: 40.0,
            fill_factor: 0.75,
        };
        let (v_mp, i_mp, p_mp) = panel.max_power_point(256);
        assert!(p_mp > 0.0);
        assert!(v_mp > 0.0 && v_mp < panel.open_circuit_voltage_v);
        assert!(i_mp > 0.0 && i_mp < panel.short_circuit_current_a);
        // MPP power < the I_sc·V_oc rectangle (fill factor < 1).
        assert!(p_mp < panel.short_circuit_current_a * panel.open_circuit_voltage_v);
        // An array of two identical panels harvests ~2× one panel.
        let arr = array_mppt_power(&[panel, panel], 256);
        assert!((arr - 2.0 * p_mp).abs() < 1e-6);
    }

    #[test]
    fn thermal_efficiency_drops_with_worse_insulation() {
        // Same delivered heat, thicker vs thinner envelope (lower vs higher U).
        let good = thermal_efficiency(1000.0, 0.5, 10.0, 20.0); // U=0.5 → loss 100 W
        let bad = thermal_efficiency(1000.0, 3.0, 10.0, 20.0); // U=3.0 → loss 600 W
        assert!(
            good > bad,
            "lower U (better insulation) ⇒ higher efficiency"
        );
        assert!(good > 0.0 && good < 1.0 && bad > 0.0 && bad < 1.0);
        // Multi-phase latent term is additive and sane.
        assert!((phase_change_energy(2.0, 334_000.0) - 668_000.0).abs() < 1.0); // ice→water
    }
}
