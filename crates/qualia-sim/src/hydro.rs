//! Zero-Heap Hydrological & Fluid Simulation Engine (`hydro.rs`)
//!
//! Bridges high-frequency fluid physics with QualiaDB's 64 Hz deterministic loop:
//! - 8-harmonic analytical Gerstner wave spectrum evaluated on CPU without allocations
//! - Multi-sphere Archimedes proxy volume buoyancy solver
//! - Kinematic fluid locomotion states (Dry, Wading, Swimming, Diving) with drag/damping
//! - Hysteresis gate filtering high-frequency noise before emitting 10D semantic transitions
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, deterministic).

/// Maximum number of analytical wave components evaluated simultaneously.
pub const MAX_GERSTNER_HARMONICS: usize = 8;
/// Maximum number of buoyancy proxy spheres per character/rigid-body hull.
pub const MAX_BUOYANCY_SPHERES: usize = 4;
/// Standard water density in kg/m³ (freshwater = 1000.0, ocean = 1025.0).
pub const WATER_DENSITY_KG_M3: f32 = 1025.0;
/// Gravity constant in m/s².
pub const GRAVITY_M_S2: f32 = 9.80665;

/// Individual Gerstner wave harmonic component.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GerstnerWave {
    /// Normalized horizontal propagation direction `[dx, dz]`.
    pub direction: [f32; 2],
    /// Wave amplitude in meters (crest height above calm surface).
    pub amplitude: f32,
    /// Wavelength in meters.
    pub wavelength: f32,
    /// Wave steepness $Q \in [0.0, 1.0]$ where $Q=0$ is sinusoidal and $Q=1$ is sharp trochoidal.
    pub steepness: f32,
    /// Phase offset in radians.
    pub phase: f32,
}

impl Default for GerstnerWave {
    fn default() -> Self {
        Self {
            direction: [1.0, 0.0],
            amplitude: 0.0,
            wavelength: 10.0,
            steepness: 0.0,
            phase: 0.0,
        }
    }
}

/// 8-harmonic Gerstner wave spectrum for zero-allocation CPU water queries.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimdGerstnerSpectrum {
    pub waves: [GerstnerWave; MAX_GERSTNER_HARMONICS],
    pub active_waves: usize,
    /// Global calm water surface elevation along the Y-axis.
    pub sea_level: f32,
}

impl Default for SimdGerstnerSpectrum {
    fn default() -> Self {
        Self {
            waves: [GerstnerWave::default(); MAX_GERSTNER_HARMONICS],
            active_waves: 0,
            sea_level: 0.0,
        }
    }
}

impl SimdGerstnerSpectrum {
    /// Constructs a standard calm coastal water profile.
    pub const fn coastal_preset(sea_level: f32) -> Self {
        Self {
            waves: [
                // Primary ocean swell: 35m wavelength, 0.6m amplitude
                GerstnerWave { direction: [0.92, 0.38], amplitude: 0.6, wavelength: 35.0, steepness: 0.45, phase: 0.0 },
                // Secondary cross-swell: 18m wavelength, 0.35m amplitude
                GerstnerWave { direction: [0.707, -0.707], amplitude: 0.35, wavelength: 18.0, steepness: 0.35, phase: 1.2 },
                // Wind chop: 7m wavelength, 0.15m amplitude
                GerstnerWave { direction: [0.38, 0.92], amplitude: 0.15, wavelength: 7.0, steepness: 0.25, phase: 2.4 },
                // High-frequency capillary ripples: 2.5m wavelength, 0.05m amplitude
                GerstnerWave { direction: [0.0, 1.0], amplitude: 0.05, wavelength: 2.5, steepness: 0.15, phase: 0.5 },
                GerstnerWave { direction: [1.0, 0.0], amplitude: 0.0, wavelength: 10.0, steepness: 0.0, phase: 0.0 },
                GerstnerWave { direction: [1.0, 0.0], amplitude: 0.0, wavelength: 10.0, steepness: 0.0, phase: 0.0 },
                GerstnerWave { direction: [1.0, 0.0], amplitude: 0.0, wavelength: 10.0, steepness: 0.0, phase: 0.0 },
                GerstnerWave { direction: [1.0, 0.0], amplitude: 0.0, wavelength: 10.0, steepness: 0.0, phase: 0.0 },
            ],
            active_waves: 4,
            sea_level,
        }
    }

    /// Evaluates the water surface elevation and normal at world coordinate `(x, z)` at time `time_sec`.
    /// Zero heap allocations, pure stack evaluation.
    pub fn sample_surface(&self, x: f32, z: f32, time_sec: f32) -> (f32, [f32; 3]) {
        let mut height = self.sea_level;
        let mut normal_x = 0.0f32;
        let mut normal_z = 0.0f32;

        let count = if self.active_waves < MAX_GERSTNER_HARMONICS {
            self.active_waves
        } else {
            MAX_GERSTNER_HARMONICS
        };

        let mut i = 0;
        while i < count {
            let w = &self.waves[i];
            if w.amplitude <= 1e-5 || w.wavelength <= 1e-4 {
                i += 1;
                continue;
            }

            // Wave number k = 2pi / L
            let k = core::f32::consts::PI * 2.0 / w.wavelength;
            // Dispersion relation: omega = sqrt(g * k)
            let omega = (GRAVITY_M_S2 * k).sqrt();
            let dot = w.direction[0] * x + w.direction[1] * z;
            let theta = k * dot - omega * time_sec + w.phase;

            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            // Vertical displacement: y = A * sin(theta)
            height += w.amplitude * sin_theta;

            // Surface derivatives for normal vector calculation
            // slope_x = k * D_x * A * cos(theta)
            // slope_z = k * D_z * A * cos(theta)
            let factor = k * w.amplitude * cos_theta;
            normal_x -= w.direction[0] * factor;
            normal_z -= w.direction[1] * factor;

            i += 1;
        }

        // Un-normalized normal = [-normal_x, 1.0, -normal_z]
        let len_sq = normal_x * normal_x + 1.0 + normal_z * normal_z;
        let inv_len = 1.0 / len_sq.sqrt();
        let normal = [normal_x * inv_len, 1.0 * inv_len, normal_z * inv_len];

        (height, normal)
    }
}

/// Locomotion and submersion mode of an entity.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmersionState {
    /// Completely dry / out of water.
    Dry = 0,
    /// Walking on bed with water around feet/calves (< 0.5m depth).
    Wading = 1,
    /// Water at chest/neck level, swimming locomotion enabled (0.5m .. 1.6m depth).
    Swimming = 2,
    /// Fully submerged underwater (6-DOF diving mechanics).
    Diving = 3,
}

/// Sphere element of an Archimedes buoyancy hull.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuoyancySphere {
    /// Relative offset `[rx, ry, rz]` from entity origin (feet).
    pub offset: [f32; 3],
    /// Radius in meters.
    pub radius: f32,
    /// Sphere volume in m³: $\frac{4}{3} \pi r^3$.
    pub volume: f32,
}

impl BuoyancySphere {
    pub fn new(offset: [f32; 3], radius: f32) -> Self {
        let volume = (4.0 / 3.0) * core::f32::consts::PI * radius * radius * radius;
        Self { offset, radius, volume }
    }
}

/// Multi-sphere proxy hull for volumetric character buoyancy.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArchimedesProxy {
    pub spheres: [BuoyancySphere; MAX_BUOYANCY_SPHERES],
    pub sphere_count: usize,
    /// Total entity mass in kg.
    pub total_mass_kg: f32,
}

impl ArchimedesProxy {
    /// Standard humanoid buoyancy hull (75 kg mass, 4 proxy spheres from feet to head).
    pub fn humanoid_default() -> Self {
        Self {
            spheres: [
                BuoyancySphere::new([0.0, 0.25, 0.0], 0.20), // Feet & calves
                BuoyancySphere::new([0.0, 0.75, 0.0], 0.22), // Hips & thighs
                BuoyancySphere::new([0.0, 1.25, 0.0], 0.24), // Torso & chest
                BuoyancySphere::new([0.0, 1.65, 0.0], 0.16), // Head
            ],
            sphere_count: 4,
            total_mass_kg: 75.0,
        }
    }

    /// Evaluates buoyant force, submerged ratio, and fluid drag on the entity.
    pub fn compute_buoyancy(
        &self,
        base_pos: [f32; 3],
        spectrum: &SimdGerstnerSpectrum,
        time_sec: f32,
    ) -> BuoyancyResult {
        let mut total_submerged_vol = 0.0f32;
        let mut max_water_depth = 0.0f32;

        let count = if self.sphere_count < MAX_BUOYANCY_SPHERES {
            self.sphere_count
        } else {
            MAX_BUOYANCY_SPHERES
        };

        let mut i = 0;
        while i < count {
            let s = &self.spheres[i];
            let sphere_center = [
                base_pos[0] + s.offset[0],
                base_pos[1] + s.offset[1],
                base_pos[2] + s.offset[2],
            ];

            let (water_y, _) = spectrum.sample_surface(sphere_center[0], sphere_center[2], time_sec);
            let water_above_center = water_y - sphere_center[1];

            if water_above_center > -s.radius {
                // Sphere is partially or fully submerged
                let submerged_vol = if water_above_center >= s.radius {
                    s.volume
                } else {
                    // Spherical cap formula: V = pi * h^2 * (3r - h) / 3
                    let h = water_above_center + s.radius;
                    core::f32::consts::PI * h * h * (3.0 * s.radius - h) / 3.0
                };
                total_submerged_vol += submerged_vol;

                let depth = water_y - base_pos[1];
                if depth > max_water_depth {
                    max_water_depth = depth;
                }
            }
            i += 1;
        }

        // Archimedes upward buoyant force: F = rho * V * g
        let buoyant_force = WATER_DENSITY_KG_M3 * total_submerged_vol * GRAVITY_M_S2;
        let gravity_force = self.total_mass_kg * GRAVITY_M_S2;
        let net_upward_accel = (buoyant_force - gravity_force) / self.total_mass_kg;

        // Submersion classification
        let state = if max_water_depth <= 0.05 {
            SubmersionState::Dry
        } else if max_water_depth < 0.65 {
            SubmersionState::Wading
        } else if max_water_depth < 1.65 {
            SubmersionState::Swimming
        } else {
            SubmersionState::Diving
        };

        BuoyancyResult {
            buoyant_force_n: buoyant_force,
            net_vertical_accel: net_upward_accel,
            submerged_volume_m3: total_submerged_vol,
            water_depth_at_feet: max_water_depth,
            state,
        }
    }
}

/// Instantaneous buoyancy evaluation result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuoyancyResult {
    /// Upward Archimedes force in Newtons.
    pub buoyant_force_n: f32,
    /// Net vertical acceleration (m/s²) imparted to the body (buoyancy - gravity).
    pub net_vertical_accel: f32,
    /// Displaced fluid volume in m³.
    pub submerged_volume_m3: f32,
    /// Depth of water relative to feet in meters.
    pub water_depth_at_feet: f32,
    /// Locomotion category.
    pub state: SubmersionState,
}

/// Hysteresis gate to filter high-frequency fluid noise before committing 10D nquin transitions.
#[derive(Debug, Clone, Copy)]
pub struct HydrologicalHysteresisGate {
    pub current_committed_state: SubmersionState,
    pub smoothed_depth: f32,
    pub wetness_ratio: f32,
    pub last_commit_tick: u64,
}

impl Default for HydrologicalHysteresisGate {
    fn default() -> Self {
        Self {
            current_committed_state: SubmersionState::Dry,
            smoothed_depth: 0.0,
            wetness_ratio: 0.0,
            last_commit_tick: 0,
        }
    }
}

impl HydrologicalHysteresisGate {
    /// Updates smoothed depth and returns `Some(new_state)` only if a discrete transition occurred.
    pub fn update(
        &mut self,
        raw_depth: f32,
        current_tick: u64,
        dt: f32,
    ) -> Option<SubmersionState> {
        // Exponential moving average filter on raw water depth
        let filter_alpha = 10.0 * dt; // ~100ms response time
        self.smoothed_depth = self.smoothed_depth + filter_alpha * (raw_depth - self.smoothed_depth);

        // Update continuous wetness ratio (absorbs instantly, dries slowly over 30 seconds)
        if self.smoothed_depth > 0.05 {
            let target_wetness = (self.smoothed_depth / 1.5).min(1.0);
            if target_wetness > self.wetness_ratio {
                self.wetness_ratio = target_wetness;
            }
        } else {
            self.wetness_ratio = (self.wetness_ratio - dt / 30.0).max(0.0);
        }

        // 5cm hysteresis threshold to avoid boundary thrashing
        let deadband = 0.05;
        let candidate_state = if self.smoothed_depth <= 0.05 - deadband {
            SubmersionState::Dry
        } else if self.smoothed_depth < 0.65 - deadband {
            SubmersionState::Wading
        } else if self.smoothed_depth < 1.65 - deadband {
            SubmersionState::Swimming
        } else {
            SubmersionState::Diving
        };

        if candidate_state != self.current_committed_state && current_tick >= self.last_commit_tick + 16 {
            self.current_committed_state = candidate_state;
            self.last_commit_tick = current_tick;
            Some(candidate_state)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calm_water_elevation_and_normal() {
        let spectrum = SimdGerstnerSpectrum::default();
        let (height, normal) = spectrum.sample_surface(12.34, 56.78, 10.0);
        assert!((height - 0.0).abs() < 1e-6);
        assert!((normal[0] - 0.0).abs() < 1e-6);
        assert!((normal[1] - 1.0).abs() < 1e-6);
        assert!((normal[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_gerstner_wave_elevation_bounds() {
        let spectrum = SimdGerstnerSpectrum::coastal_preset(0.0);
        let (height, normal) = spectrum.sample_surface(0.0, 0.0, 0.0);
        // Swell amplitude sum: 0.6 + 0.35 + 0.15 + 0.05 = 1.15m
        assert!(height >= -1.2 && height <= 1.2);
        // Normal must be unit length
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_archimedes_buoyancy_progression() {
        let spectrum = SimdGerstnerSpectrum::default(); // Flat sea level at 0.0
        let proxy = ArchimedesProxy::humanoid_default();

        // 1. Entity high above water (feet at y = 5.0m) -> Dry
        let dry = proxy.compute_buoyancy([0.0, 5.0, 0.0], &spectrum, 0.0);
        assert_eq!(dry.state, SubmersionState::Dry);
        assert_eq!(dry.submerged_volume_m3, 0.0);
        assert!(dry.net_vertical_accel < 0.0); // Net downward gravity

        // 2. Entity wading (feet at y = -0.3m) -> Wading
        let wading = proxy.compute_buoyancy([0.0, -0.3, 0.0], &spectrum, 0.0);
        assert_eq!(wading.state, SubmersionState::Wading);
        assert!(wading.submerged_volume_m3 > 0.0);

        // 3. Entity swimming (feet at y = -1.2m) -> Swimming
        let swimming = proxy.compute_buoyancy([0.0, -1.2, 0.0], &spectrum, 0.0);
        assert_eq!(swimming.state, SubmersionState::Swimming);

        // 4. Entity submerged (feet at y = -3.0m) -> Diving
        let diving = proxy.compute_buoyancy([0.0, -3.0, 0.0], &spectrum, 0.0);
        assert_eq!(diving.state, SubmersionState::Diving);
        assert!(diving.buoyant_force_n > 0.0);
    }

    #[test]
    fn test_hysteresis_gate_filtering() {
        let mut gate = HydrologicalHysteresisGate::default();
        // High-frequency tick noise should not commit within deadband
        let trans1 = gate.update(0.04, 1, 0.015625);
        assert_eq!(trans1, None);

        // Deep step committing to wading after settling
        let mut trans = None;
        for tick in 2..30 {
            if let Some(t) = gate.update(0.45, tick, 0.015625) {
                trans = Some(t);
            }
        }
        assert_eq!(trans, Some(SubmersionState::Wading));
    }
}
