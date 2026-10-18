//! Atmospheric Physics, Boundary-Layer Wind & Aeroelastic Coupling (`atmosphere.rs`)
//!
//! Provides zero-heap physical modeling of:
//! - Synoptic barometric state: Pressure, temperature, humidity, and moist air density
//! - Orographic lifting condensation level (LCL) and inverse barometer sea surface bulge
//! - Prandtl–von Kármán atmospheric boundary layer (ABL) logarithmic wind profile
//! - Hierarchical aeroelastic vegetation mechanics:
//!   * Level 0/1: Trunk & primary bough cantilever harmonic bending
//!   * Level 2: Secondary branch Vogel crown streamlining ($F_{\text{drag}} \propto v^{2+\mathcal{V}}$)
//!   * Level 3: Leaf Strouhal vortex shedding flutter ($f = \text{St} \cdot v / L$) & petiole abscission detachment
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, deterministic).

/// Gas constant for dry air in J/(kg·K).
pub const GAS_CONSTANT_DRY_AIR: f32 = 287.058;
/// Gas constant for water vapor in J/(kg·K).
pub const GAS_CONSTANT_WATER_VAPOR: f32 = 461.495;
/// von Kármán constant for boundary-layer turbulence.
pub const VON_KARMAN_KAPPA: f32 = 0.41;
/// Standard atmospheric pressure at sea level in hPa.
pub const STANDARD_MSLP_HPA: f32 = 1013.25;

/// Aerodynamic roughness length $z_0$ presets (in meters).
pub mod roughness {
    pub const CALM_WATER_Z0: f32 = 0.0002;
    pub const MOWN_GRASS_Z0: f32 = 0.008;
    pub const FARMLAND_CROPS_Z0: f32 = 0.05;
    pub const DENSE_EUCALYPT_FOREST_Z0: f32 = 1.25;
}

/// Synoptic thermodynamic and barometric state of the air column.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarometricState {
    /// Mean Sea Level Pressure in hPa (e.g., 1013.25).
    pub mslp_hpa: f32,
    /// Ambient air temperature at 2m in degrees Celsius.
    pub temperature_c: f32,
    /// Dew point temperature at 2m in degrees Celsius.
    pub dew_point_c: f32,
    /// Relative humidity in [0.0, 1.0].
    pub relative_humidity: f32,
    /// Horizontal geostrophic wind vector [u, v] at 10m in m/s.
    pub wind_vector_10m: [f32; 2],
}

impl Default for BarometricState {
    fn default() -> Self {
        Self {
            mslp_hpa: STANDARD_MSLP_HPA,
            temperature_c: 15.0,
            dew_point_c: 9.0,
            relative_humidity: 0.65,
            wind_vector_10m: [4.5, 0.0],
        }
    }
}

impl BarometricState {
    /// Calculates air density $\rho_{\text{air}}$ (in kg/m³) using the ideal gas law for moist air:
    /// $\rho = \frac{P_d}{R_d T} + \frac{P_v}{R_v T}$
    pub fn air_density(&self, altitude_m: f32) -> f32 {
        // Barometric formula for pressure at altitude
        let temp_k = self.temperature_c + 273.15;
        let pressure_pa = self.mslp_hpa * 100.0 * (1.0 - 0.0065 * altitude_m / temp_k).powf(5.255);

        // Saturation vapor pressure via Tetens equation in hPa
        let e_sat_hpa = 6.1078 * (17.27 * self.temperature_c / (self.temperature_c + 237.3)).exp();
        let e_hpa = e_sat_hpa * self.relative_humidity;
        let p_v_pa = e_hpa * 100.0;
        let p_d_pa = pressure_pa - p_v_pa;

        let rho_d = p_d_pa / (GAS_CONSTANT_DRY_AIR * temp_k);
        let rho_v = p_v_pa / (GAS_CONSTANT_WATER_VAPOR * temp_k);
        (rho_d + rho_v).max(0.1)
    }

    /// Evaluates the Lifting Condensation Level (LCL) height above ground in meters.
    /// Air forced over mountains condenses and triggers precipitation when $z \ge z_{\text{LCL}}$.
    #[inline]
    pub fn lifting_condensation_level_m(&self) -> f32 {
        // Espy's approximation: z_LCL ≈ 125 * (T - T_dew)
        (125.0 * (self.temperature_c - self.dew_point_c)).max(0.0)
    }

    /// Inverse Barometer Effect: Evaluates the static sea surface elevation anomaly (in meters).
    /// Low pressure centers (storms) cause water levels to bulge upward by ~1 cm per 1 hPa drop.
    #[inline]
    pub fn inverse_barometer_offset_m(&self) -> f32 {
        (STANDARD_MSLP_HPA - self.mslp_hpa) * 0.0101
    }

    /// Horizontal wind speed magnitude at 10m in m/s.
    #[inline]
    pub fn wind_speed_10m(&self) -> f32 {
        (self.wind_vector_10m[0] * self.wind_vector_10m[0] + self.wind_vector_10m[1] * self.wind_vector_10m[1]).sqrt()
    }
}

/// Atmospheric Boundary Layer (ABL) logarithmic wind profile.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryLayerProfile {
    /// Aerodynamic roughness length $z_0$ in meters.
    pub roughness_length_z0: f32,
    /// Zero-plane displacement height $d$ in meters (for forest/canopy lift).
    pub zero_plane_displacement_d: f32,
    /// Friction velocity $u_*$ in m/s (momentum shear stress transferred into surface).
    pub friction_velocity_u_star: f32,
}

impl Default for BoundaryLayerProfile {
    fn default() -> Self {
        Self {
            roughness_length_z0: roughness::DENSE_EUCALYPT_FOREST_Z0,
            zero_plane_displacement_d: 6.0,
            friction_velocity_u_star: 0.35,
        }
    }
}

impl BoundaryLayerProfile {
    /// Evaluates horizontal wind speed $u(z)$ at elevation $z$ above ground (in meters):
    /// $u(z) = \frac{u_*}{\kappa} \ln\left(\frac{z - d}{z_0}\right)$
    pub fn sample_wind_speed(&self, height_z: f32) -> f32 {
        let eff_z = height_z - self.zero_plane_displacement_d;
        if eff_z <= self.roughness_length_z0 {
            // Below canopy / roughness threshold: attenuated quadratic damping
            let sub_ratio = (height_z / (self.zero_plane_displacement_d + self.roughness_length_z0)).clamp(0.0, 1.0);
            (self.friction_velocity_u_star / VON_KARMAN_KAPPA) * sub_ratio * 0.5
        } else {
            let log_ratio = (eff_z / self.roughness_length_z0).max(1.0).ln();
            (self.friction_velocity_u_star / VON_KARMAN_KAPPA) * log_ratio
        }
    }

    /// Evaluates dynamic wind pressure $q = \frac{1}{2} \rho_{\text{air}} v^2$ (in Pascals/N·m⁻²).
    #[inline]
    pub fn dynamic_pressure(&self, air_density: f32, wind_speed: f32) -> f32 {
        0.5 * air_density * wind_speed * wind_speed
    }
}

/// Aeroelastic branch oscillator with Vogel crown streamlining reconfiguration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AeroelasticBranch {
    /// Branch length in meters.
    pub length_m: f32,
    /// Natural resonant frequency in Hz ($0.3\text{--}1.5\text{ Hz}$).
    pub natural_frequency_hz: f32,
    /// Damping ratio $\zeta$ ($0.08\text{--}0.25$ viscous damping).
    pub damping_ratio: f32,
    /// Vogel streamlining reconfiguration exponent $\mathcal{V} \in [-0.8, -0.4]$.
    /// Negative exponent reduces frontal drag area as flexible branches bend leeward.
    pub vogel_exponent: f32,
    /// Current angular deflection in radians.
    pub deflection_rad: f32,
    /// Angular velocity in rad/s.
    pub angular_velocity: f32,
}

impl Default for AeroelasticBranch {
    fn default() -> Self {
        Self {
            length_m: 2.2,
            natural_frequency_hz: 0.65,
            damping_ratio: 0.15,
            vogel_exponent: -0.6,
            deflection_rad: 0.0,
            angular_velocity: 0.0,
        }
    }
}

impl AeroelasticBranch {
    /// Advances the branch oscillator by one fixed timestep $\Delta t$.
    /// Uses Vogel's drag reduction formula: $F_{\text{aero}} = q \cdot A_0 \cdot (1 + |\theta|)^{\mathcal{V}}$
    pub fn step(&mut self, dynamic_pressure: f32, dt: f32) {
        let omega_n = 2.0 * core::f32::consts::PI * self.natural_frequency_hz;
        let k_spring = omega_n * omega_n;
        let c_damper = 2.0 * self.damping_ratio * omega_n;

        // Vogel drag scaling: bending leeward streamlines the branch and sheds load
        let streamlining_factor = (1.0 + self.deflection_rad.abs()).powf(self.vogel_exponent);
        let torque_aero = dynamic_pressure * self.length_m * 0.12 * streamlining_factor;
        let torque_restoring = -k_spring * self.deflection_rad;
        let torque_damping = -c_damper * self.angular_velocity;

        let angular_accel = torque_aero + torque_restoring + torque_damping;
        self.angular_velocity += angular_accel * dt;
        self.deflection_rad += self.angular_velocity * dt;
    }
}

/// Aeroelastic leaf oscillator with Strouhal vortex-shedding flutter and petiole abscission.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeafFlutterState {
    /// Characteristic leaf length in meters (e.g. 0.12m for eucalyptus lanceolate leaf).
    pub leaf_length_m: f32,
    /// Strouhal number ($\text{St} \approx 0.18$ for elliptical leaves).
    pub strouhal_number: f32,
    /// Torsional flutter angle in radians.
    pub torsional_angle_rad: f32,
    /// Tensile petiole abscission limit in Newtons. When drag exceeds this, leaf detaches.
    pub abscission_limit_n: f32,
    /// True if the leaf has detached and become a tumbling airborne particle.
    pub is_detached: bool,
}

impl Default for LeafFlutterState {
    fn default() -> Self {
        Self {
            leaf_length_m: 0.14,
            strouhal_number: 0.18,
            torsional_angle_rad: 0.0,
            abscission_limit_n: 1.85,
            is_detached: false,
        }
    }
}

impl LeafFlutterState {
    /// Computes Strouhal vortex shedding flutter frequency $f = \frac{\text{St} \cdot v}{L}$ in Hz.
    #[inline]
    pub fn flutter_frequency_hz(&self, wind_speed: f32) -> f32 {
        (self.strouhal_number * wind_speed / self.leaf_length_m).clamp(1.0, 35.0)
    }

    /// Advances leaf flutter state. Returns `true` if petiole tensile failure causes leaf detachment on this tick.
    pub fn step(&mut self, wind_speed: f32, dynamic_pressure: f32, dt: f32) -> bool {
        if self.is_detached {
            return false;
        }

        // Petiole drag force: F_drag = C_d * A * q
        let leaf_area = self.leaf_length_m * 0.035; // ~50 cm²
        let drag_force = 1.15 * leaf_area * dynamic_pressure;

        // Abscission detachment check: storm gust snaps petiole
        if drag_force > self.abscission_limit_n {
            self.is_detached = true;
            return true;
        }

        // Torsional flutter driven by Strouhal alternating shed vortices
        let freq = self.flutter_frequency_hz(wind_speed);
        let flutter_amp = (dynamic_pressure * 0.04).clamp(0.05, 0.75);
        let phase_advance = 2.0 * core::f32::consts::PI * freq * dt;
        let base_phase = (self.torsional_angle_rad + phase_advance) % (2.0 * core::f32::consts::PI);
        self.torsional_angle_rad = base_phase * flutter_amp;

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_air_density_standard_conditions() {
        let state = BarometricState::default(); // 1013.25 hPa, 15°C, 65% RH
        let density = state.air_density(0.0);
        // Standard sea level air density is ~1.225 kg/m³
        assert!((density - 1.225).abs() < 0.02);
    }

    #[test]
    fn test_lifting_condensation_level() {
        let mut state = BarometricState::default();
        state.temperature_c = 22.0;
        state.dew_point_c = 14.0;
        let lcl = state.lifting_condensation_level_m();
        // 125 * (22 - 14) = 1000m
        assert!((lcl - 1000.0).abs() < 1e-3);
    }

    #[test]
    fn test_inverse_barometer_sea_elevation() {
        let mut storm = BarometricState::default();
        storm.mslp_hpa = 980.0; // Deep low pressure center
        let sea_bulge = storm.inverse_barometer_offset_m();
        // (1013.25 - 980) * 0.0101 ≈ +0.335m storm surge bulge
        assert!(sea_bulge > 0.30 && sea_bulge < 0.36);
    }

    #[test]
    fn test_boundary_layer_wind_profile() {
        let profile = BoundaryLayerProfile::default(); // Forest canopy d = 6m, z0 = 1.25m
        let speed_canopy = profile.sample_wind_speed(5.0); // Inside canopy
        let speed_top = profile.sample_wind_speed(15.0);   // Above canopy
        let speed_high = profile.sample_wind_speed(40.0);  // High boundary layer

        assert!(speed_canopy < speed_top);
        assert!(speed_top < speed_high);
    }

    #[test]
    fn test_aeroelastic_branch_vogel_streamlining() {
        let mut branch = AeroelasticBranch::default();
        let dt = 0.015625; // 64 Hz

        // Step under moderate dynamic pressure (10 m/s wind -> q ≈ 60 Pa)
        for _ in 0..128 {
            branch.step(60.0, dt);
        }

        // Branch must be deflected leeward and stable
        assert!(branch.deflection_rad > 0.05);
        assert!(branch.deflection_rad < 1.2);
    }

    #[test]
    fn test_leaf_abscission_detachment_in_storm() {
        let mut leaf = LeafFlutterState::default();
        let dt = 0.015625;

        // Mild breeze (3 m/s, q ≈ 5 Pa) should flutter without detaching
        let detached_breeze = leaf.step(3.0, 5.0, dt);
        assert!(!detached_breeze);
        assert!(!leaf.is_detached);

        // Gale storm gust (35 m/s, q ≈ 750 Pa) must snap petiole and detach
        let detached_gale = leaf.step(35.0, 750.0, dt);
        assert!(detached_gale);
        assert!(leaf.is_detached);
    }
}
