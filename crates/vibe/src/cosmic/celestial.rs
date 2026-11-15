//! Celestial body taxonomy, relativistic gravito-electromagnetic propagation,
//! and pulsar triangulation (OCS §2, §6.2.5.2).
//!
//! Reference: OCS Specification v2.2.0 §2 & Playbook v4 §6.2.5.2.

use crate::value::Value;
use std::collections::BTreeMap;

/// Exact SI speed of light in vacuum $c_0$ (m/s).
///
/// Used exclusively to define the invariant Lorentzian causal cone boundary.
pub const C0_VACUUM_M_S: f64 = 299_792_458.0;

/// Speed of light constant alias (m/s).
const C: f64 = C0_VACUUM_M_S;

/// Cold-plasma pulsar dispersion constant $\mathcal{D} \approx 4.148808\text{ ms}\cdot\text{GHz}^2\cdot\text{pc}^{-1}\cdot\text{cm}^3$.
pub const PULSAR_DISPERSION_CONSTANT_MS_GHZ2: f64 = 4.148808;

/// Celestial body classification (OCS §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CelestialBodyClass {
    TerrestrialPlanet,
    GasGiant,
    IceGiant,
    DwarfPlanet,
    NaturalSatellite,
    MinorBodyIrregular,
    Star,
    CompactRelativistic,
    BlackHole,
    Megastructure,
    FictionalWorld,
}

impl CelestialBodyClass {
    /// `str` form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TerrestrialPlanet => "TerrestrialPlanet",
            Self::GasGiant => "GasGiant",
            Self::IceGiant => "IceGiant",
            Self::DwarfPlanet => "DwarfPlanet",
            Self::NaturalSatellite => "NaturalSatellite",
            Self::MinorBodyIrregular => "MinorBodyIrregular",
            Self::Star => "Star",
            Self::CompactRelativistic => "CompactRelativistic",
            Self::BlackHole => "BlackHole",
            Self::Megastructure => "Megastructure",
            Self::FictionalWorld => "FictionalWorld",
        }
    }

    /// Constructs from str.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "TerrestrialPlanet" => Some(Self::TerrestrialPlanet),
            "GasGiant" => Some(Self::GasGiant),
            "IceGiant" => Some(Self::IceGiant),
            "DwarfPlanet" => Some(Self::DwarfPlanet),
            "NaturalSatellite" => Some(Self::NaturalSatellite),
            "MinorBodyIrregular" => Some(Self::MinorBodyIrregular),
            "Star" => Some(Self::Star),
            "CompactRelativistic" => Some(Self::CompactRelativistic),
            "BlackHole" => Some(Self::BlackHole),
            "Megastructure" => Some(Self::Megastructure),
            "FictionalWorld" => Some(Self::FictionalWorld),
            _ => None,
        }
    }

    /// Whether this body type has a solid surface.
    pub fn has_solid_surface(&self) -> bool {
        matches!(
            self,
            Self::TerrestrialPlanet
                | Self::DwarfPlanet
                | Self::NaturalSatellite
                | Self::MinorBodyIrregular
                | Self::Megastructure
                | Self::FictionalWorld
        )
    }

    /// Whether this body type requires relativistic metric handling.
    pub fn is_relativistic(&self) -> bool {
        matches!(self, Self::CompactRelativistic | Self::BlackHole)
    }
}

/// Reference ellipsoid type (OCS §2.2).
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceSurface {
    /// Biaxial ellipsoid (a = b ≠ c) for rotationally flattened bodies.
    Biaxial { a: f64, c: f64 },
    /// Triaxial ellipsoid (a > b > c) for non-axisymmetric bodies.
    Triaxial { a: f64, b: f64, c: f64 },
    /// Equipotential surface (geoid, areoid, 1-bar level).
    Equipotential { name: String },
    /// 3D polygonal mesh for irregular bodies.
    Mesh { name: String },
}

/// Relativistic causal cone classification separated from medium signal arrival (§6.2.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CausalConeRelation {
    /// Event lies on or inside the past vacuum causal cone ($\Delta t < 0$, $|\Delta t| \ge \Delta t_{\text{vacuum+Shapiro}}$).
    PastCausalEnvelope,
    /// Event lies on or inside the future vacuum causal cone ($\Delta t \ge \Delta t_{\text{vacuum+Shapiro}}$).
    FutureCausalEnvelope,
    /// Event is causally reachable in vacuum, and signal arrives after dispersive/refractive medium group delay.
    DispersedMediumArrival,
    /// Event is outside the vacuum + Shapiro causal cone ($|\Delta t| < \Delta t_{\text{vacuum+Shapiro}}$).
    SpacelikeElsewhere,
}

#[allow(non_upper_case_globals)]
impl CausalConeRelation {
    /// Alias for `PastCausalEnvelope` for light-cone callers.
    pub const PastLightCone: Self = Self::PastCausalEnvelope;
    /// Alias for `FutureCausalEnvelope` for light-cone callers.
    pub const FutureLightCone: Self = Self::FutureCausalEnvelope;
}

/// Electromagnetic medium specification for signal propagation vs causal geometry (§6.2.5.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmfMediumSpec {
    /// Group refractive index $n_g \ge 1.0$ ($v_g = c_0 / n_g \le c_0$).
    pub group_refractive_index: f64,
    /// Phase refractive index $n_p > 0.0$ ($v_p = c_0 / n_p$; in cold plasma $n_p < 1 \implies v_p > c_0$ while $v_p v_g = c_0^2$).
    pub phase_refractive_index: f64,
    /// Carrier frequency in Hz (0.0 if unspecified / broadband vacuum).
    pub carrier_freq_hz: f64,
    /// Integrated electron column density / dispersion measure ($\text{pc}\cdot\text{cm}^{-3}$).
    pub dispersion_measure_pc_cm3: f64,
}

impl EmfMediumSpec {
    /// Exact vacuum propagation ($n_g = n_p = 1$, $\text{DM} = 0$).
    pub const fn vacuum() -> Self {
        Self {
            group_refractive_index: 1.0,
            phase_refractive_index: 1.0,
            carrier_freq_hz: 0.0,
            dispersion_measure_pc_cm3: 0.0,
        }
    }

    /// Cold ionized plasma from plasma frequency $f_p$ and carrier frequency $f > f_p$.
    ///
    /// Satisfies $\varepsilon(\omega) = 1 - \omega_p^2 / \omega^2$, so
    /// $n_p = \sqrt{1 - (f_p/f)^2} < 1$, $n_g = 1 / n_p > 1$, and $v_p v_g = c_0^2$.
    pub fn cold_plasma(
        carrier_freq_hz: f64,
        plasma_freq_hz: f64,
        dispersion_measure_pc_cm3: f64,
    ) -> Result<Self, &'static str> {
        if carrier_freq_hz <= 0.0 || plasma_freq_hz < 0.0 || carrier_freq_hz <= plasma_freq_hz {
            return Err("carrier_freq_hz must exceed plasma_freq_hz >= 0");
        }
        let ratio_sq = (plasma_freq_hz / carrier_freq_hz) * (plasma_freq_hz / carrier_freq_hz);
        let n_p = (1.0 - ratio_sq).sqrt();
        let n_g = 1.0 / n_p;
        Ok(Self {
            group_refractive_index: n_g,
            phase_refractive_index: n_p,
            carrier_freq_hz,
            dispersion_measure_pc_cm3,
        })
    }
}

/// Medium propagation estimate paired with invariant causal cone classification (§6.2.5.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MediumPropagationEstimate {
    /// Euclidean spatial separation $|\mathbf{r}_2 - \mathbf{r}_1|$ (m).
    pub euclidean_distance_m: f64,
    /// Vacuum light-time $|\mathbf{r}_2 - \mathbf{r}_1| / c_0$ (s).
    pub vacuum_light_time_s: f64,
    /// First-order post-Newtonian Shapiro gravitational time delay $\Delta t_{\text{Shapiro}}$ (s).
    pub shapiro_delay_s: f64,
    /// Extra group delay from refractive index $n_g > 1$ and cold-plasma dispersion $\Delta t_{\text{DM}}$ (s).
    pub medium_group_delay_s: f64,
    /// Total minimum vacuum+gravitational causal time $\Delta t_{\text{vacuum}} + \Delta t_{\text{Shapiro}}$ (s).
    pub causal_envelope_time_s: f64,
    /// Total expected signal arrival time $\Delta t_{\text{causal}} + \Delta t_{\text{medium}}$ (s).
    pub total_signal_arrival_time_s: f64,
    /// Group velocity $v_g = c_0 / n_g \le c_0$ (m/s).
    pub group_velocity_m_s: f64,
    /// Phase velocity $v_p = c_0 / n_p$ (m/s; may exceed $c_0$ in cold plasma without violating causality).
    pub phase_velocity_m_s: f64,
    /// Lorentzian invariant spacetime interval squared $ds^2 = -c_0^2 \Delta t_{\text{eff}}^2 + \Delta r^2$ ($\text{m}^2$).
    pub spacetime_interval_m2: f64,
}

/// Gravitational field model (OCS §2.3).
#[derive(Debug, Clone, PartialEq)]
pub struct GravitationalField {
    /// Standard gravitational parameter μ = GM (m³/s²).
    pub mu: f64,
    /// Reference radius for harmonic expansion (m).
    pub r_ref: f64,
    /// Zonal harmonic coefficients J₂, J₃, ...
    pub jn: Vec<f64>,
    /// Schwarzschild radius r_s = 2GM/c² (m). Zero for non-relativistic bodies.
    pub schwarzschild_r: f64,
    /// Kerr spin parameter a = J/(Mc) (m). Zero for non-spinning bodies.
    pub kerr_spin_a: f64,
}

impl GravitationalField {
    /// Create a simple Newtonian field (no relativistic terms).
    pub fn newtonian(mu: f64, r_ref: f64) -> Self {
        Self {
            mu,
            r_ref,
            jn: Vec::new(),
            schwarzschild_r: 0.0,
            kerr_spin_a: 0.0,
        }
    }

    /// Kerr event horizon radius r₊ (OCS §2.3).
    /// r₊ = r_s/2 + sqrt((r_s/2)² - a²)
    pub fn kerr_horizon_r(&self) -> f64 {
        if self.schwarzschild_r == 0.0 {
            return 0.0;
        }
        let half_rs = self.schwarzschild_r / 2.0;
        let disc = half_rs * half_rs - self.kerr_spin_a * self.kerr_spin_a;
        if disc < 0.0 {
            return 0.0; // No horizon (naked singularity — unphysical)
        }
        half_rs + disc.sqrt()
    }

    /// Kerr ergosphere boundary r_E(θ) at θ = π/2 (equator) (OCS §2.3).
    /// r_E(θ) = r_s/2 + sqrt((r_s/2)² - a²cos²θ)
    pub fn kerr_ergosphere_r(&self, theta: f64) -> f64 {
        if self.schwarzschild_r == 0.0 {
            return 0.0;
        }
        let half_rs = self.schwarzschild_r / 2.0;
        let cos_theta = theta.cos();
        let disc = half_rs * half_rs - self.kerr_spin_a * self.kerr_spin_a * cos_theta * cos_theta;
        if disc < 0.0 {
            return 0.0;
        }
        half_rs + disc.sqrt()
    }

    /// Lense-Thirring frame-dragging precession frequency Ω_LT (OCS §2.3, Packet OCS-12).
    ///
    /// Since $\mu = GM$ ($\text{m}^3/\text{s}^2$) and $a = J / (Mc)$ ($\text{m}$),
    /// $GJ = \mu c a$, so $\Omega_{\text{LT}} = \frac{2GJ}{c^2 r^3} = \frac{2\mu a}{c r^3}$ ($\text{rad/s}$).
    pub fn frame_dragging(&self, r: f64) -> f64 {
        if self.schwarzschild_r == 0.0 || self.kerr_spin_a == 0.0 || r <= 0.0 {
            return 0.0;
        }
        2.0 * self.mu * self.kerr_spin_a / (C0_VACUUM_M_S * r * r * r)
    }

    /// First-order post-Newtonian (1PN) Shapiro gravitational time delay $\Delta t_{\text{Shapiro}}$ (s)
    /// for a signal propagating from `r1_m` to `r2_m` past this gravitating body at the origin:
    ///
    /// $$\Delta t_{\text{Shapiro}} = \frac{2\mu}{c_0^3} \ln\!\left(\frac{r_1 + r_2 + R}{r_1 + r_2 - R}\right)$$
    pub fn shapiro_delay_s(&self, r1_m: [f64; 3], r2_m: [f64; 3]) -> f64 {
        if self.mu <= 0.0 {
            return 0.0;
        }
        let r1 = (r1_m[0] * r1_m[0] + r1_m[1] * r1_m[1] + r1_m[2] * r1_m[2]).sqrt();
        let r2 = (r2_m[0] * r2_m[0] + r2_m[1] * r2_m[1] + r2_m[2] * r2_m[2]).sqrt();
        let dx = r2_m[0] - r1_m[0];
        let dy = r2_m[1] - r1_m[1];
        let dz = r2_m[2] - r1_m[2];
        let big_r = (dx * dx + dy * dy + dz * dz).sqrt();
        let denom = (r1 + r2 - big_r).max(1e-9);
        let numer = (r1 + r2 + big_r).max(denom);
        let c3 = C0_VACUUM_M_S * C0_VACUUM_M_S * C0_VACUUM_M_S;
        (2.0 * self.mu / c3) * (numer / denom).ln()
    }

    /// Evaluate the relativistic causal interval and dispersive medium propagation estimate
    /// between two events $(\mathbf{r}_1, t_1)$ and $(\mathbf{r}_2, t_2 = t_1 + \Delta t)$ (§6.2.5.2).
    ///
    /// Enforces:
    /// 1. Vacuum speed $c_0 = 299\,792\,458\text{ m/s}$ defines the invariant causal cone.
    /// 2. Signal group velocity satisfies $v_g \le c_0$ ($n_g \ge 1.0$).
    /// 3. Phase velocity $v_p = c_0 / n_p$ may exceed $c_0$ in cold plasma ($n_p < 1$) without
    ///    corrupting causal ordering.
    pub fn emf_manifold_causal_interval_s(
        &self,
        r1_m: [f64; 3],
        r2_m: [f64; 3],
        coord_dt_s: f64,
        medium: &EmfMediumSpec,
    ) -> Result<(CausalConeRelation, MediumPropagationEstimate), &'static str> {
        if medium.group_refractive_index < 1.0 || !medium.group_refractive_index.is_finite() {
            return Err("Superluminal group velocity forbidden: group_refractive_index must be >= 1.0");
        }
        if medium.phase_refractive_index <= 0.0 || !medium.phase_refractive_index.is_finite() {
            return Err("phase_refractive_index must be positive and finite");
        }

        let dx = r2_m[0] - r1_m[0];
        let dy = r2_m[1] - r1_m[1];
        let dz = r2_m[2] - r1_m[2];
        let dist_m = (dx * dx + dy * dy + dz * dz).sqrt();

        let vacuum_light_time_s = dist_m / C0_VACUUM_M_S;
        let shapiro_s = self.shapiro_delay_s(r1_m, r2_m);
        let causal_envelope_time_s = vacuum_light_time_s + shapiro_s;

        let refractive_delay_s = vacuum_light_time_s * (medium.group_refractive_index - 1.0);
        let dm_delay_s = if medium.dispersion_measure_pc_cm3 > 0.0 && medium.carrier_freq_hz > 0.0 {
            pulsar_dispersion_delay_s(medium.dispersion_measure_pc_cm3, medium.carrier_freq_hz)?
        } else {
            0.0
        };
        let medium_group_delay_s = refractive_delay_s + dm_delay_s;
        let total_signal_arrival_time_s = causal_envelope_time_s + medium_group_delay_s;

        let group_velocity_m_s = C0_VACUUM_M_S / medium.group_refractive_index;
        let phase_velocity_m_s = C0_VACUUM_M_S / medium.phase_refractive_index;

        let abs_dt = coord_dt_s.abs();
        let eff_vacuum_dt = (abs_dt - shapiro_s).max(0.0);
        let spacetime_interval_m2 =
            -(C0_VACUUM_M_S * eff_vacuum_dt) * (C0_VACUUM_M_S * eff_vacuum_dt) + dist_m * dist_m;

        let relation = if abs_dt + 1e-12 < causal_envelope_time_s {
            CausalConeRelation::SpacelikeElsewhere
        } else if coord_dt_s < 0.0 {
            CausalConeRelation::PastCausalEnvelope
        } else if medium_group_delay_s > 0.0 && abs_dt + 1e-12 >= total_signal_arrival_time_s {
            CausalConeRelation::DispersedMediumArrival
        } else {
            CausalConeRelation::FutureCausalEnvelope
        };

        Ok((
            relation,
            MediumPropagationEstimate {
                euclidean_distance_m: dist_m,
                vacuum_light_time_s,
                shapiro_delay_s: shapiro_s,
                medium_group_delay_s,
                causal_envelope_time_s,
                total_signal_arrival_time_s,
                group_velocity_m_s,
                phase_velocity_m_s,
                spacetime_interval_m2,
            },
        ))
    }
}

/// Convenience wrapper for [`GravitationalField::emf_manifold_causal_interval_s`] (§6.2.5.2).
pub fn emf_manifold_causal_interval_s(
    field: &GravitationalField,
    r1_m: [f64; 3],
    r2_m: [f64; 3],
    coord_dt_s: f64,
    medium: &EmfMediumSpec,
) -> Result<(CausalConeRelation, MediumPropagationEstimate), &'static str> {
    field.emf_manifold_causal_interval_s(r1_m, r2_m, coord_dt_s, medium)
}

/// Millisecond pulsar timing anchor for deep-space BCRS navigation (OCS-12, §6.2.5.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulsarAnchor {
    /// Deterministic identifier hash of the pulsar (e.g. PSR B1937+21).
    pub id_hash: u64,
    /// Unit direction vector $\hat{n}_i$ from BCRS origin toward the pulsar.
    pub direction_unit: [f64; 3],
    /// Spin period $P_0$ at reference epoch (s).
    pub period_s: f64,
    /// Spin-down derivative $\dot{P}$ (dimensionless s/s).
    pub period_derivative: f64,
    /// Reference epoch $t_0$ in BCRS TDB seconds.
    pub epoch_t0_s: f64,
    /// Observation carrier frequency $f$ in Hz (0.0 for infinite-frequency / X-ray band).
    pub carrier_freq_hz: f64,
    /// Interstellar dispersion measure $\text{DM} = \int n_e\,ds$ ($\text{pc}\cdot\text{cm}^{-3}$).
    pub dispersion_measure_pc_cm3: f64,
}

impl PulsarAnchor {
    /// Construct an X-ray pulsar anchor where cold-plasma dispersion is negligible.
    pub fn x_ray(
        id_hash: u64,
        direction_unit: [f64; 3],
        period_s: f64,
        period_derivative: f64,
        epoch_t0_s: f64,
    ) -> Self {
        Self {
            id_hash,
            direction_unit,
            period_s,
            period_derivative,
            epoch_t0_s,
            carrier_freq_hz: 0.0,
            dispersion_measure_pc_cm3: 0.0,
        }
    }

    /// Cold-plasma dispersion delay $\Delta t_{\text{DM}}$ (s) at this anchor's carrier frequency.
    pub fn dispersion_delay_s(&self) -> Result<f64, &'static str> {
        if self.dispersion_measure_pc_cm3 <= 0.0 || self.carrier_freq_hz <= 0.0 {
            return Ok(0.0);
        }
        pulsar_dispersion_delay_s(self.dispersion_measure_pc_cm3, self.carrier_freq_hz)
    }
}

/// Single-band cold-plasma dispersion delay (s) relative to infinite frequency (§6.2.5.2):
///
/// $$\Delta t_{\text{DM}}(f) = 4.148808 \times 10^{-3}\text{ s} \cdot \text{DM} \cdot \left(\frac{f}{10^9\text{ Hz}}\right)^{-2}$$
pub fn pulsar_dispersion_delay_s(dm_pc_cm3: f64, freq_hz: f64) -> Result<f64, &'static str> {
    if dm_pc_cm3 < 0.0 || !dm_pc_cm3.is_finite() {
        return Err("dispersion_measure_pc_cm3 must be non-negative and finite");
    }
    if freq_hz <= 0.0 || !freq_hz.is_finite() {
        return Err("carrier frequency freq_hz must be positive and finite");
    }
    let freq_ghz = freq_hz * 1.0e-9;
    Ok((PULSAR_DISPERSION_CONSTANT_MS_GHZ2 * 1.0e-3) * dm_pc_cm3 / (freq_ghz * freq_ghz))
}

/// Differential cold-plasma dispersion delay (s) between two radio carrier frequencies $f_1, f_2$ (§6.2.5.2):
///
/// $$\Delta t_{\text{DM}}(f_1, f_2) = 4.148808\text{ ms} \cdot \text{DM} \cdot (f_{1,\text{GHz}}^{-2} - f_{2,\text{GHz}}^{-2})$$
pub fn dual_band_dedisperse_delay_s(
    dm_pc_cm3: f64,
    freq_1_hz: f64,
    freq_2_hz: f64,
) -> Result<f64, &'static str> {
    let dt1 = pulsar_dispersion_delay_s(dm_pc_cm3, freq_1_hz)?;
    let dt2 = pulsar_dispersion_delay_s(dm_pc_cm3, freq_2_hz)?;
    Ok(dt1 - dt2)
}

/// Infer both the interstellar Dispersion Measure ($\text{pc}\cdot\text{cm}^{-3}$) and the
/// infinite-frequency vacuum time of arrival $t_{\infty}$ (s) from dual-band TOAs $(t_1, f_1)$ and $(t_2, f_2)$.
pub fn infer_dispersion_measure_dual_band(
    toa_1_s: f64,
    freq_1_hz: f64,
    toa_2_s: f64,
    freq_2_hz: f64,
) -> Result<(f64, f64), &'static str> {
    if freq_1_hz <= 0.0 || freq_2_hz <= 0.0 || (freq_1_hz - freq_2_hz).abs() < 1.0 {
        return Err("dual-band frequencies must be distinct positive values in Hz");
    }
    let f1_ghz = freq_1_hz * 1.0e-9;
    let f2_ghz = freq_2_hz * 1.0e-9;
    let inv_diff = 1.0 / (f1_ghz * f1_ghz) - 1.0 / (f2_ghz * f2_ghz);
    let k_s = PULSAR_DISPERSION_CONSTANT_MS_GHZ2 * 1.0e-3;
    let dm = (toa_1_s - toa_2_s) / (k_s * inv_diff);
    if dm < 0.0 || !dm.is_finite() {
        return Err("inferred dispersion measure is negative or non-finite");
    }
    let dt1 = pulsar_dispersion_delay_s(dm, freq_1_hz)?;
    let vacuum_toa_s = toa_1_s - dt1;
    Ok((dm, vacuum_toa_s))
}

/// Triangulate 3D BCRS position $\mathbf{r} = [x, y, z]$ (m) from 4 pulsar phase observations (OCS-12, §6.2.5.2).
///
/// Each pulsar $i \in \{0, 1, 2, 3\}$ yields an observed phase offset $\Delta\phi_i$ (cycles)
/// relative to the BCRS origin reference model:
///
/// $$\Delta t_i = \Delta\phi_i \cdot P_i - \Delta t_{\text{DM},i} = -\frac{\hat{n}_i \cdot \mathbf{r}}{c_0} + \delta t_{\text{clock}}$$
///
/// Differencing anchors $i = 1, 2, 3$ against anchor $0$ cancels the unknown local clock bias
/// $\delta t_{\text{clock}}$ and produces a $3\times 3$ linear system $(\hat{n}_0 - \hat{n}_i) \cdot \mathbf{r} = c_0 (\Delta t_{\text{vac},i} - \Delta t_{\text{vac},0})$.
pub fn triangulate_from_pulsars(
    anchors: &[PulsarAnchor; 4],
    observed_phases: &[f64; 4],
) -> Result<[f64; 3], &'static str> {
    let mut vac_dt = [0.0_f64; 4];
    let mut unit_dirs = [[0.0_f64; 3]; 4];

    for i in 0..4 {
        let a = &anchors[i];
        if a.period_s <= 0.0 || !a.period_s.is_finite() {
            return Err("pulsar period_s must be positive and finite");
        }
        let norm = (a.direction_unit[0] * a.direction_unit[0]
            + a.direction_unit[1] * a.direction_unit[1]
            + a.direction_unit[2] * a.direction_unit[2])
            .sqrt();
        if norm < 1e-12 {
            return Err("pulsar direction_unit must be non-zero");
        }
        unit_dirs[i] = [
            a.direction_unit[0] / norm,
            a.direction_unit[1] / norm,
            a.direction_unit[2] / norm,
        ];
        let dm_delay = a.dispersion_delay_s()?;
        // Dedisperse the phase arrival time before geometric projection
        vac_dt[i] = observed_phases[i] * a.period_s - dm_delay;
    }

    // Build 3x3 system: M * r = rhs, where row (i-1) is (n_0 - n_i) · r = c_0 * (vac_dt[i] - vac_dt[0])
    let mut m = [[0.0_f64; 3]; 3];
    let mut rhs = [0.0_f64; 3];
    for row in 0..3 {
        let i = row + 1;
        m[row] = [
            unit_dirs[0][0] - unit_dirs[i][0],
            unit_dirs[0][1] - unit_dirs[i][1],
            unit_dirs[0][2] - unit_dirs[i][2],
        ];
        rhs[row] = C0_VACUUM_M_S * (vac_dt[i] - vac_dt[0]);
    }

    solve_3x3(m, rhs)
}

fn solve_3x3(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Result<[f64; 3], &'static str> {
    for col in 0..3 {
        let mut pivot_row = col;
        let mut max_abs = a[col][col].abs();
        for r in (col + 1)..3 {
            let v = a[r][col].abs();
            if v > max_abs {
                max_abs = v;
                pivot_row = r;
            }
        }
        if max_abs < 1e-12 {
            return Err("degenerate pulsar geometry: direction differences are coplanar");
        }
        if pivot_row != col {
            a.swap(col, pivot_row);
            b.swap(col, pivot_row);
        }
        let pivot = a[col][col];
        for r in (col + 1)..3 {
            let factor = a[r][col] / pivot;
            for c in col..3 {
                a[r][c] -= factor * a[col][c];
            }
            b[r] -= factor * b[col];
        }
    }

    let mut x = [0.0_f64; 3];
    for row in (0..3).rev() {
        let mut sum = b[row];
        for col in (row + 1)..3 {
            sum -= a[row][col] * x[col];
        }
        x[row] = sum / a[row][row];
    }
    Ok(x)
}

/// A celestial body profile (OCS §2).
#[derive(Debug, Clone)]
pub struct CelestialBodyProfile {
    pub name: String,
    pub class: CelestialBodyClass,
    pub usri: String,
    pub mass_kg: f64,
    pub equatorial_radius_m: f64,
    pub rotation_period_s: f64,
    pub surface: ReferenceSurface,
    pub gravity: GravitationalField,
}

impl CelestialBodyProfile {
    /// Surface gravity g = GM/R² (m/s²).
    pub fn surface_gravity(&self) -> f64 {
        self.gravity.mu / (self.equatorial_radius_m * self.equatorial_radius_m)
    }

    /// Convert to Value::Record.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("class".into(), Value::String(self.class.as_str().into()));
        rec.insert("usri".into(), Value::String(self.usri.clone()));
        rec.insert("mass_kg".into(), Value::F64(self.mass_kg));
        rec.insert(
            "equatorial_radius_m".into(),
            Value::F64(self.equatorial_radius_m),
        );
        rec.insert(
            "rotation_period_s".into(),
            Value::F64(self.rotation_period_s),
        );
        rec.insert("surface_gravity".into(), Value::F64(self.surface_gravity()));
        Value::Record(rec)
    }
}

/// Known celestial body profiles (OCS §2 examples).
pub fn earth_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Earth".into(),
        class: CelestialBodyClass::TerrestrialPlanet,
        usri: "urn:omni:v1:physical:observable:standard:sol:earth".into(),
        mass_kg: 5.972e24,
        equatorial_radius_m: 6_378_137.0,
        rotation_period_s: 86_164.0905, // sidereal day
        surface: ReferenceSurface::Biaxial {
            a: 6_378_137.0,
            c: 6_356_752.3,
        },
        gravity: {
            let mut g = GravitationalField::newtonian(3.986004418e14, 6_378_137.0);
            g.jn = vec![-1.08263e-3, 2.57e-6]; // J2, J3
            g
        },
    }
}

/// Mars profile.
pub fn mars_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Mars".into(),
        class: CelestialBodyClass::TerrestrialPlanet,
        usri: "urn:omni:v1:physical:observable:standard:sol:mars".into(),
        mass_kg: 6.4171e23,
        equatorial_radius_m: 3_396_200.0,
        rotation_period_s: 88_642.66, // Martian sol
        surface: ReferenceSurface::Biaxial {
            a: 3_396_200.0,
            c: 3_376_200.0,
        },
        gravity: {
            let mut g = GravitationalField::newtonian(4.282837e13, 3_396_200.0);
            g.jn = vec![-1.96045e-3];
            g
        },
    }
}

/// Jupiter — gas giant (OCS §2.1).
pub fn jupiter_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Jupiter".into(),
        class: CelestialBodyClass::GasGiant,
        usri: "urn:omni:v1:physical:observable:standard:sol:jupiter".into(),
        mass_kg: 1.898e27,
        equatorial_radius_m: 71_492_000.0, // At 1-bar level
        rotation_period_s: 35_718.0,       // ~9h 55m
        surface: ReferenceSurface::Equipotential {
            name: "1-bar level".into(),
        },
        gravity: GravitationalField::newtonian(1.26686534e17, 71_492_000.0),
    }
}

/// Saturn — gas giant (OCS §2.1).
pub fn saturn_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Saturn".into(),
        class: CelestialBodyClass::GasGiant,
        usri: "urn:omni:v1:physical:observable:standard:sol:saturn".into(),
        mass_kg: 5.683e26,
        equatorial_radius_m: 60_268_000.0,
        rotation_period_s: 38_352.0, // ~10h 39m
        surface: ReferenceSurface::Equipotential {
            name: "1-bar level".into(),
        },
        gravity: GravitationalField::newtonian(3.7931187e16, 60_268_000.0),
    }
}

/// Earth's Moon — natural satellite (OCS §2.1).
pub fn moon_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Moon".into(),
        class: CelestialBodyClass::NaturalSatellite,
        usri: "urn:omni:v1:physical:observable:standard:sol:earth:moon".into(),
        mass_kg: 7.342e22,
        equatorial_radius_m: 1_737_400.0,
        rotation_period_s: 2_360_591.6, // Sidereal month
        surface: ReferenceSurface::Biaxial {
            a: 1_737_400.0,
            c: 1_735_970.0,
        },
        gravity: GravitationalField::newtonian(4.9048695e12, 1_737_400.0),
    }
}

/// Europa — icy moon with subsurface ocean (OCS §2.6).
pub fn europa_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Europa".into(),
        class: CelestialBodyClass::NaturalSatellite,
        usri: "urn:omni:v1:physical:observable:standard:sol:jupiter:europa".into(),
        mass_kg: 4.8e22,
        equatorial_radius_m: 1_560_800.0,
        rotation_period_s: 271_558.8, // Tidally locked
        surface: ReferenceSurface::Biaxial {
            a: 1_560_800.0,
            c: 1_560_300.0,
        },
        gravity: GravitationalField::newtonian(3.202e12, 1_560_800.0),
    }
}

/// Neutron star — compact relativistic object (OCS §2.1).
pub fn neutron_star_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Neutron Star (typical)".into(),
        class: CelestialBodyClass::CompactRelativistic,
        usri: "urn:omni:v1:physical:observable:standard:neutron-star".into(),
        mass_kg: 2.8e30,               // ~1.4 solar masses
        equatorial_radius_m: 12_000.0, // ~12 km
        rotation_period_s: 0.001,      // ~1ms pulsar
        surface: ReferenceSurface::Biaxial {
            a: 12_000.0,
            c: 11_990.0,
        },
        gravity: GravitationalField {
            mu: 1.867e20,
            r_ref: 12_000.0,
            jn: vec![],
            schwarzschild_r: 2.0 * 1.867e20 / (C * C), // ~4.16 km
            kerr_spin_a: 1000.0,                       // Spinning pulsar
        },
    }
}

/// Sagittarius A* — supermassive black hole (OCS §2.1, §2.3).
pub fn sgr_a_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Sagittarius A*".into(),
        class: CelestialBodyClass::BlackHole,
        usri: "urn:omni:v1:physical:observable:standard:milkyway:sgr-a".into(),
        mass_kg: 8.26e36,              // ~4.15 million solar masses
        equatorial_radius_m: 1.227e10, // Event horizon (~12.27 million km)
        rotation_period_s: 1.0e4,      // Slowly spinning SMBH
        surface: ReferenceSurface::Equipotential {
            name: "event horizon".into(),
        },
        gravity: GravitationalField {
            mu: 5.514e26,
            r_ref: 1.227e10,
            jn: vec![],
            schwarzschild_r: 1.227e10,         // r_s = 2GM/c²
            kerr_spin_a: 0.9 * 1.227e10 / 2.0, // High spin: a ≈ 0.9 * r_s/2
        },
    }
}

/// The Sun — main sequence star (OCS §2.1).
pub fn sun_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Sun".into(),
        class: CelestialBodyClass::Star,
        usri: "urn:omni:v1:physical:observable:standard:sol:sun".into(),
        mass_kg: 1.989e30,
        equatorial_radius_m: 696_340_000.0,
        rotation_period_s: 2_192_320.0, // ~25.4 days (equatorial)
        surface: ReferenceSurface::Equipotential {
            name: "photosphere".into(),
        },
        gravity: GravitationalField::newtonian(1.32712440018e20, 6_963_400_000.0),
    }
}

/// Look up a body profile by name (OCS §2).
pub fn body_profile_by_name(name: &str) -> Option<CelestialBodyProfile> {
    match name {
        "earth" => Some(earth_profile()),
        "mars" => Some(mars_profile()),
        "jupiter" => Some(jupiter_profile()),
        "saturn" => Some(saturn_profile()),
        "moon" => Some(moon_profile()),
        "europa" => Some(europa_profile()),
        "neutron-star" => Some(neutron_star_profile()),
        "sgr-a" => Some(sgr_a_profile()),
        "sun" => Some(sun_profile()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_surface_gravity() {
        let earth = earth_profile();
        let g = earth.surface_gravity();
        assert!((g - 9.81).abs() < 0.1, "got {} expected ~9.81", g);
    }

    #[test]
    fn mars_surface_gravity() {
        let mars = mars_profile();
        let g = mars.surface_gravity();
        assert!((g - 3.71).abs() < 0.1, "got {} expected ~3.71", g);
    }

    #[test]
    fn jupiter_surface_gravity() {
        let j = jupiter_profile();
        let g = j.surface_gravity();
        assert!((g - 24.79).abs() < 1.0, "got {} expected ~24.79", g);
    }

    #[test]
    fn moon_surface_gravity() {
        let m = moon_profile();
        let g = m.surface_gravity();
        assert!((g - 1.62).abs() < 0.1, "got {} expected ~1.62", g);
    }

    #[test]
    fn sun_surface_gravity() {
        let s = sun_profile();
        let g = s.surface_gravity();
        assert!((g - 274.0).abs() < 10.0, "got {} expected ~274", g);
    }

    #[test]
    fn neutron_star_has_schwarzschild() {
        let ns = neutron_star_profile();
        assert!(ns.gravity.schwarzschild_r > 0.0);
        assert!(ns.gravity.kerr_spin_a > 0.0);
        assert!(ns.class.is_relativistic());
    }

    #[test]
    fn sgr_a_is_black_hole() {
        let sgr = sgr_a_profile();
        assert_eq!(sgr.class, CelestialBodyClass::BlackHole);
        assert!(sgr.gravity.schwarzschild_r > 0.0);
        assert!(sgr.gravity.kerr_horizon_r() > 0.0);
    }

    #[test]
    fn body_profile_lookup() {
        assert!(body_profile_by_name("earth").is_some());
        assert!(body_profile_by_name("jupiter").is_some());
        assert!(body_profile_by_name("sgr-a").is_some());
        assert!(body_profile_by_name("pluto").is_none());
    }

    #[test]
    fn europa_is_natural_satellite() {
        let e = europa_profile();
        assert_eq!(e.class, CelestialBodyClass::NaturalSatellite);
        assert!(e.surface_gravity() < 2.0);
    }

    #[test]
    fn class_has_solid_surface() {
        assert!(CelestialBodyClass::TerrestrialPlanet.has_solid_surface());
        assert!(!CelestialBodyClass::GasGiant.has_solid_surface());
        assert!(CelestialBodyClass::Megastructure.has_solid_surface());
    }

    #[test]
    fn class_is_relativistic() {
        assert!(CelestialBodyClass::BlackHole.is_relativistic());
        assert!(CelestialBodyClass::CompactRelativistic.is_relativistic());
        assert!(!CelestialBodyClass::Star.is_relativistic());
    }

    #[test]
    fn class_round_trip() {
        for c in [
            CelestialBodyClass::TerrestrialPlanet,
            CelestialBodyClass::GasGiant,
            CelestialBodyClass::BlackHole,
            CelestialBodyClass::FictionalWorld,
        ] {
            assert_eq!(CelestialBodyClass::from_str(c.as_str()), Some(c));
        }
    }

    #[test]
    fn kerr_horizon_schwarzschild() {
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 0.0,
        };
        assert!((g.kerr_horizon_r() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_horizon_spinning() {
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 30.0,
        };
        assert!((g.kerr_horizon_r() - 90.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_ergosphere_at_equator() {
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 30.0,
        };
        let r_eq = g.kerr_ergosphere_r(std::f64::consts::PI / 2.0);
        assert!((r_eq - 100.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_ergosphere_at_pole() {
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 30.0,
        };
        let r_pole = g.kerr_ergosphere_r(0.0);
        assert!((r_pole - 90.0).abs() < 1e-10);
    }

    #[test]
    fn frame_dragging_nonzero_and_dimensionally_exact() {
        let g = GravitationalField {
            mu: 3.986e14,
            r_ref: 6_378_137.0,
            jn: vec![],
            schwarzschild_r: 0.00887,
            kerr_spin_a: 3.3, // Earth Kerr spin parameter a = J/(Mc) ≈ 3.3 m
        };
        let r = 6_800_000.0;
        let omega = g.frame_dragging(r);
        let expected = 2.0 * g.mu * g.kerr_spin_a / (C0_VACUUM_M_S * r * r * r);
        assert!(omega > 0.0);
        assert!((omega - expected).abs() < 1e-24);
        // Gravity Probe B order-of-magnitude check (~2.8e-14 rad/s at LEO, NOT divided by c^2)
        assert!(omega > 1e-15 && omega < 1e-12, "omega = {}", omega);
    }

    #[test]
    fn pulsar_dual_band_dedispersion_and_4_anchor_triangulation() {
        let dm = 25.0; // pc cm^-3
        let f1 = 1.4e9; // 1.4 GHz
        let f2 = 2.8e9; // 2.8 GHz
        let diff_s = dual_band_dedisperse_delay_s(dm, f1, f2).unwrap();
        let expected_s =
            4.148808e-3 * dm * (1.0 / (1.4 * 1.4) - 1.0 / (2.8 * 2.8));
        assert!((diff_s - expected_s).abs() < 1e-12);

        let true_vac_toa = 1000.123456;
        let toa1 = true_vac_toa + pulsar_dispersion_delay_s(dm, f1).unwrap();
        let toa2 = true_vac_toa + pulsar_dispersion_delay_s(dm, f2).unwrap();
        let (inferred_dm, inferred_vac_toa) =
            infer_dispersion_measure_dual_band(toa1, f1, toa2, f2).unwrap();
        assert!((inferred_dm - dm).abs() < 1e-9);
        assert!((inferred_vac_toa - true_vac_toa).abs() < 1e-12);

        // 4-pulsar triangulation with both clock bias and radio dispersion
        let anchors = [
            PulsarAnchor {
                id_hash: 1,
                direction_unit: [1.0, 0.0, 0.0],
                period_s: 0.001557,
                period_derivative: 1.0e-19,
                epoch_t0_s: 0.0,
                carrier_freq_hz: 1.4e9,
                dispersion_measure_pc_cm3: 15.0,
            },
            PulsarAnchor {
                id_hash: 2,
                direction_unit: [0.0, 1.0, 0.0],
                period_s: 0.005757,
                period_derivative: 1.0e-20,
                epoch_t0_s: 0.0,
                carrier_freq_hz: 1.4e9,
                dispersion_measure_pc_cm3: 32.0,
            },
            PulsarAnchor {
                id_hash: 3,
                direction_unit: [0.0, 0.0, 1.0],
                period_s: 0.002145,
                period_derivative: 1.0e-20,
                epoch_t0_s: 0.0,
                carrier_freq_hz: 2.0e9,
                dispersion_measure_pc_cm3: 18.5,
            },
            PulsarAnchor::x_ray(
                4,
                [-1.0 / 3.0_f64.sqrt(), -1.0 / 3.0_f64.sqrt(), -1.0 / 3.0_f64.sqrt()],
                0.003050,
                1.0e-20,
                0.0,
            ),
        ];

        let true_pos_m = [1.5e8, -2.2e8, 0.85e8];
        let clock_bias_s = 0.00042; // 420 us unknown local receiver clock offset

        let mut observed_phases = [0.0_f64; 4];
        for i in 0..4 {
            let n = anchors[i].direction_unit;
            let geom_dt =
                -(n[0] * true_pos_m[0] + n[1] * true_pos_m[1] + n[2] * true_pos_m[2]) / C0_VACUUM_M_S;
            let dm_dt = anchors[i].dispersion_delay_s().unwrap();
            let total_dt = geom_dt + clock_bias_s + dm_dt;
            observed_phases[i] = total_dt / anchors[i].period_s;
        }

        let solved_pos = triangulate_from_pulsars(&anchors, &observed_phases).unwrap();
        for axis in 0..3 {
            assert!(
                (solved_pos[axis] - true_pos_m[axis]).abs() < 1e-3,
                "axis {} mismatch: got {}, expected {}",
                axis,
                solved_pos[axis],
                true_pos_m[axis]
            );
        }
    }

    #[test]
    fn emf_manifold_causal_cone_and_cold_plasma_dispersion() {
        let sun = sun_profile();
        let r1 = [-1.5e11, 7.0e8, 0.0];
        let r2 = [1.5e11, 7.0e8, 0.0];

        // Cold plasma: f = 1.5 GHz, f_p = 15 MHz, DM = 40 pc cm^-3
        let plasma = EmfMediumSpec::cold_plasma(1.5e9, 1.5e7, 40.0).unwrap();
        // Verify phase velocity > c_0 while group velocity < c_0 and v_p * v_g == c_0^2
        let (_, est_spacelike) = sun
            .gravity
            .emf_manifold_causal_interval_s(r1, r2, 100.0, &plasma)
            .unwrap();
        assert!(est_spacelike.group_velocity_m_s < C0_VACUUM_M_S);
        assert!(est_spacelike.phase_velocity_m_s > C0_VACUUM_M_S);
        let prod = est_spacelike.group_velocity_m_s * est_spacelike.phase_velocity_m_s;
        let c0_sq = C0_VACUUM_M_S * C0_VACUUM_M_S;
        assert!((prod - c0_sq).abs() / c0_sq < 1e-12);
        assert!(est_spacelike.shapiro_delay_s > 0.0);

        let (rel_spacelike, _) = sun
            .gravity
            .emf_manifold_causal_interval_s(r1, r2, 100.0, &plasma)
            .unwrap();
        assert_eq!(rel_spacelike, CausalConeRelation::SpacelikeElsewhere);

        let (rel_arrival, est_arrival) = sun
            .gravity
            .emf_manifold_causal_interval_s(
                r1,
                r2,
                est_spacelike.total_signal_arrival_time_s + 1.0,
                &plasma,
            )
            .unwrap();
        assert_eq!(rel_arrival, CausalConeRelation::DispersedMediumArrival);
        assert!(est_arrival.spacetime_interval_m2 < 0.0);

        // Superluminal group velocity (n_g < 1.0) must be rejected
        let bad_medium = EmfMediumSpec {
            group_refractive_index: 0.95,
            phase_refractive_index: 1.0,
            carrier_freq_hz: 1.0e9,
            dispersion_measure_pc_cm3: 0.0,
        };
        assert!(sun
            .gravity
            .emf_manifold_causal_interval_s(r1, r2, 2000.0, &bad_medium)
            .is_err());
    }

    #[test]
    fn profile_to_value() {
        let earth = earth_profile();
        let v = earth.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("Earth".into())));
                assert!(r.contains_key("surface_gravity"));
            }
            _ => panic!("expected Record"),
        }
    }
}
