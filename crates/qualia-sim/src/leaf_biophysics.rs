//! Procedural Leaf Biophysics & Dual-Graph Venation (`leaf_biophysics.rs`)
//!
//! Models leaves as dual-system biophysical structures:
//! - 1D Vascular Skeleton (midrib, secondary veins, reticulate anastomosing loops)
//! - Murray's Law radius tapering ($r_p^3 = r_1^3 + r_2^3$)
//! - Hagen-Poiseuille hydraulic conductance ($K_h = \frac{\pi r^4}{8 \eta L}$)
//! - Turgor-coupled mechanical stiffness ($E_{\text{eff}} = E_{\text{tissue}} + k_{\text{hydro}} P_t$)
//! - Leaf Economics Spectrum (LES) allometric parameterization
//!
//! Strictly zero heap in hot evaluation loops (`AGENTS.md` §0).

pub const MAX_VEIN_EDGES: usize = 96;
pub const NO_PARENT_VEIN: u16 = 0xFFFF;

/// A single vascular vein segment within the leaf dual-graph skeleton.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VeinEdge {
    /// Normalized 2D coordinate on leaf lamina [-1.0, 1.0]
    pub start: [f32; 2],
    /// Normalized 2D coordinate on leaf lamina [-1.0, 1.0]
    pub end: [f32; 2],
    /// Parent vein index (or `NO_PARENT_VEIN` for petiole insertion)
    pub parent_idx: u16,
    /// Vein radius in millimeters (governed by Murray's Law)
    pub radius_mm: f32,
    /// Hagen-Poiseuille hydraulic conductance ($m^4 \cdot s^{-1} \cdot Pa^{-1}$)
    pub conductance: f32,
    /// Vascular hierarchy order: 1 = primary midrib, 2 = secondary, 3 = reticulate loop
    pub order: u8,
    /// Anastomosing fusion flag (true if this edge closes a structural loop)
    pub is_loop_closure: bool,
}

impl Default for VeinEdge {
    fn default() -> Self {
        Self {
            start: [0.0, 0.0],
            end: [0.0, 0.0],
            parent_idx: NO_PARENT_VEIN,
            radius_mm: 0.1,
            conductance: 1e-12,
            order: 1,
            is_loop_closure: false,
        }
    }
}

/// Continuous Leaf Economics Spectrum (LES) profile.
///
/// Drives leaf morphology, venation density, and mechanics without hardcoding species.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeafEconomicsProfile {
    /// Leaf Mass per Area (LMA) in $g/m^2$ (e.g. 40.0 for deciduous, 220.0 for sclerophyllous eucalyptus)
    pub lma_g_m2: f32,
    /// Vein Length per Area (VLA) in $mm / mm^2$ (typically 5.0 to 20.0)
    pub vein_density_mm_per_mm2: f32,
    /// Loopiness $\alpha$-index $[0.0, 1.0]$: 0.0 = open tree-branching, 1.0 = fully closed reticulate loops
    pub loopiness_alpha: f32,
    /// Secondary vein branching angle $\theta$ in radians (e.g. $0.65 \approx 37^\circ$)
    pub branching_angle_rad: f32,
    /// Bulk tissue Young's modulus without water in MPa (e.g. 5.0 - 50.0 MPa)
    pub tissue_elastic_modulus_mpa: f32,
    /// Turgor coupling coefficient $k_{\text{hydro}}$ (dimensionless stiffness multiplier per MPa turgor)
    pub turgor_coupling_k: f32,
    /// Nominal leaf length along midrib in meters
    pub leaf_length_m: f32,
    /// Nominal leaf maximum width in meters
    pub leaf_width_m: f32,
}

impl LeafEconomicsProfile {
    /// Archetype: Sclerophyllous Australian Eucalyptus (thick, tough, reticulate, drought-tolerant).
    pub const fn eucalyptus_sclerophyll() -> Self {
        Self {
            lma_g_m2: 210.0,
            vein_density_mm_per_mm2: 16.5,
            loopiness_alpha: 0.85,
            branching_angle_rad: 0.55,
            tissue_elastic_modulus_mpa: 45.0,
            turgor_coupling_k: 22.0,
            leaf_length_m: 0.16,
            leaf_width_m: 0.035,
        }
    }

    /// Archetype: Herbaceous Broadleaf (thin lamina, high turgor dependency, wilts quickly).
    pub const fn herbaceous_broadleaf() -> Self {
        Self {
            lma_g_m2: 45.0,
            vein_density_mm_per_mm2: 7.0,
            loopiness_alpha: 0.40,
            branching_angle_rad: 0.78,
            tissue_elastic_modulus_mpa: 4.5,
            turgor_coupling_k: 85.0,
            leaf_length_m: 0.09,
            leaf_width_m: 0.065,
        }
    }

    /// Archetype: Linear Monocot Grass Blade.
    pub const fn monocot_grass() -> Self {
        Self {
            lma_g_m2: 60.0,
            vein_density_mm_per_mm2: 12.0,
            loopiness_alpha: 0.05,
            branching_angle_rad: 0.08, // Parallel venation
            tissue_elastic_modulus_mpa: 18.0,
            turgor_coupling_k: 60.0,
            leaf_length_m: 0.25,
            leaf_width_m: 0.012,
        }
    }
}

/// Zero-heap planar leaf vascular skeleton with coupled biophysics.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LeafVascularSkeleton {
    pub edges: [VeinEdge; MAX_VEIN_EDGES],
    pub edge_count: usize,
    pub profile: LeafEconomicsProfile,
    /// Current internal turgor pressure in MPa (0.0 = completely flaccid/wilted, ~1.5 MPa = fully turgid)
    pub turgor_pressure_mpa: f32,
    /// Terminal vein tip base radius in mm
    pub tip_radius_mm: f32,
}

impl LeafVascularSkeleton {
    /// Constructs a procedural vascular skeleton based on an LES profile.
    pub fn new(profile: LeafEconomicsProfile, turgor_pressure_mpa: f32) -> Self {
        let mut skeleton = Self {
            edges: [VeinEdge::default(); MAX_VEIN_EDGES],
            edge_count: 0,
            profile,
            turgor_pressure_mpa,
            tip_radius_mm: 0.08,
        };
        skeleton.generate_venation();
        skeleton.propagate_murray_law();
        skeleton.evaluate_hydraulics(1.002e-3); // Water viscosity at 20°C in Pa·s
        skeleton
    }

    /// Procedurally seeds primary midrib, secondary ribs, and reticulate loops.
    pub fn generate_venation(&mut self) {
        self.edge_count = 0;

        // 1. Primary midrib: 5 segments along the central axis from petiole (y=0) to apex (y=1)
        let midrib_segments = 5;
        let mut prev_idx = NO_PARENT_VEIN;

        for i in 0..midrib_segments {
            if self.edge_count >= MAX_VEIN_EDGES {
                break;
            }
            let y0 = (i as f32) / (midrib_segments as f32);
            let y1 = ((i + 1) as f32) / (midrib_segments as f32);

            let idx = self.edge_count;
            self.edges[idx] = VeinEdge {
                start: [0.0, y0],
                end: [0.0, y1],
                parent_idx: prev_idx,
                radius_mm: self.tip_radius_mm,
                conductance: 0.0,
                order: 1,
                is_loop_closure: false,
            };
            self.edge_count += 1;
            prev_idx = idx as u16;
        }

        let midrib_end_idx = self.edge_count;

        // 2. Secondary lateral veins branching off midrib nodes
        let angle = self.profile.branching_angle_rad;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let max_w = self.profile.leaf_width_m / self.profile.leaf_length_m.max(1e-3);

        for m_idx in 1..midrib_end_idx {
            let base_pt = self.edges[m_idx].start;
            let len = (1.0 - base_pt[1] * 0.6) * max_w * 0.9;

            // Right secondary branch
            if self.edge_count < MAX_VEIN_EDGES {
                let r_idx = self.edge_count;
                let end_r = [base_pt[0] + sin_a * len, base_pt[1] + cos_a * len * 0.7];
                self.edges[r_idx] = VeinEdge {
                    start: base_pt,
                    end: end_r,
                    parent_idx: m_idx as u16,
                    radius_mm: self.tip_radius_mm,
                    conductance: 0.0,
                    order: 2,
                    is_loop_closure: false,
                };
                self.edge_count += 1;
            }

            // Left secondary branch
            if self.edge_count < MAX_VEIN_EDGES {
                let l_idx = self.edge_count;
                let end_l = [base_pt[0] - sin_a * len, base_pt[1] + cos_a * len * 0.7];
                self.edges[l_idx] = VeinEdge {
                    start: base_pt,
                    end: end_l,
                    parent_idx: m_idx as u16,
                    radius_mm: self.tip_radius_mm,
                    conductance: 0.0,
                    order: 2,
                    is_loop_closure: false,
                };
                self.edge_count += 1;
            }
        }

        // 3. Reticulate loop closures (anastomosing network) if loopiness_alpha > 0.3
        if self.profile.loopiness_alpha > 0.3 && self.edge_count + 2 < MAX_VEIN_EDGES {
            let count_before_loops = self.edge_count;
            for i in (midrib_end_idx..count_before_loops).step_by(2) {
                if i + 2 < count_before_loops && self.edge_count < MAX_VEIN_EDGES {
                    // Bridge adjacent secondary vein tips
                    let loop_idx = self.edge_count;
                    self.edges[loop_idx] = VeinEdge {
                        start: self.edges[i].end,
                        end: self.edges[i + 2].end,
                        parent_idx: i as u16,
                        radius_mm: self.tip_radius_mm * 0.7,
                        conductance: 0.0,
                        order: 3,
                        is_loop_closure: true,
                    };
                    self.edge_count += 1;
                }
            }
        }
    }

    /// Propagates Murray's Law from leaf tips backward to the petiole:
    /// $r_p^3 = \sum r_{\text{child}}^3 \implies r_p = (\sum r_c^3)^{1/3}$
    pub fn propagate_murray_law(&mut self) {
        if self.edge_count == 0 {
            return;
        }

        // Reset radii to base tip radius
        for i in 0..self.edge_count {
            self.edges[i].radius_mm = self.tip_radius_mm;
        }

        // Bottom-up pass in reverse index order
        for i in (0..self.edge_count).rev() {
            let p_idx = self.edges[i].parent_idx;
            if p_idx != NO_PARENT_VEIN && (p_idx as usize) < self.edge_count {
                let r_child = self.edges[i].radius_mm;
                let r_parent = self.edges[p_idx as usize].radius_mm;

                // Murray's law accumulation: r_new = (r_p^3 + r_c^3)^(1/3)
                let r_cubed_sum = r_parent.powi(3) + r_child.powi(3);
                self.edges[p_idx as usize].radius_mm = r_cubed_sum.cbrt();
            }
        }
    }

    /// Evaluates Hagen-Poiseuille hydraulic conductance for every vascular edge:
    /// $K_h = \frac{\pi r^4}{8 \eta L}$
    pub fn evaluate_hydraulics(&mut self, dynamic_viscosity_pa_s: f32) {
        let eta = dynamic_viscosity_pa_s.max(1e-6);
        let scale_l = self.profile.leaf_length_m;

        for i in 0..self.edge_count {
            let edge = &mut self.edges[i];
            let dx = (edge.end[0] - edge.start[0]) * self.profile.leaf_width_m;
            let dy = (edge.end[1] - edge.start[1]) * scale_l;
            let length_m = (dx * dx + dy * dy).sqrt().max(1e-4);

            let radius_m = (edge.radius_mm * 1e-3).max(1e-6);
            let r4 = radius_m.powi(4);

            // Poiseuille formula: (pi * r^4) / (8 * eta * L)
            edge.conductance = (std::f32::consts::PI * r4) / (8.0 * eta * length_m);
        }
    }

    /// Computes the effective Young's modulus governed by tissue structure + hydraulic turgor:
    /// $E_{\text{eff}} = E_{\text{tissue}} + k_{\text{hydro}} \cdot P_t$
    #[inline]
    pub fn effective_elastic_modulus_mpa(&self) -> f32 {
        self.profile.tissue_elastic_modulus_mpa
            + self.profile.turgor_coupling_k * self.turgor_pressure_mpa.max(0.0)
    }

    /// Evaluates leaf cantilever droop angle in radians due to gravity and wind pressure.
    ///
    /// When turgor drops, $E_{\text{eff}}$ decreases, causing the leaf to wilt downward.
    pub fn evaluate_droop_angle(&self, wind_speed_m_s: f32) -> f32 {
        let e_eff_pa = self.effective_elastic_modulus_mpa() * 1e6;
        let petiole_r_m = self.edges[0].radius_mm.max(1.2) * 1e-3;

        // Second moment of area for circular petiole: I = pi * r^4 / 4
        let second_moment_i = (std::f32::consts::PI * petiole_r_m.powi(4)) / 4.0;
        let flexural_rigidity = (e_eff_pa * second_moment_i).max(1e-12);

        // Total leaf mass in kg = Area * LMA
        let leaf_area_m2 = self.profile.leaf_length_m * self.profile.leaf_width_m * 0.7; // ~0.7 ellipse factor
        let dry_mass_kg = leaf_area_m2 * (self.profile.lma_g_m2 * 1e-3);
        let water_mass_kg = leaf_area_m2 * (self.turgor_pressure_mpa * 0.15); // Water content proportional to turgor
        let total_mass_kg = dry_mass_kg + water_mass_kg;

        // Gravitational downward bending moment M_g = m * g * (L / 2)
        let mg = total_mass_kg * 9.80665;
        let arm = self.profile.leaf_length_m * 0.5;
        let moment_gravity = mg * arm;

        // Dynamic wind drag torque
        let air_density = 1.204;
        let q_wind = 0.5 * air_density * wind_speed_m_s * wind_speed_m_s;
        let drag_force = q_wind * leaf_area_m2 * 0.6; // C_d ~ 0.6
        let moment_wind = drag_force * arm;

        let total_moment = (moment_gravity * moment_gravity + moment_wind * moment_wind).sqrt();

        // Tip deflection angular slope theta ~ M * L / (E * I)
        let angle_rad = (total_moment * self.profile.leaf_length_m) / flexural_rigidity;
        angle_rad.clamp(0.0, std::f32::consts::FRAC_PI_2 * 0.95)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_murray_law_propagation() {
        let profile = LeafEconomicsProfile::eucalyptus_sclerophyll();
        let skeleton = LeafVascularSkeleton::new(profile, 1.2);

        assert!(skeleton.edge_count > 5);
        let petiole = &skeleton.edges[0];
        let midrib_apex = &skeleton.edges[4];

        // Petiole must be substantially thicker than apex due to Murray's law accumulation
        assert!(
            petiole.radius_mm > midrib_apex.radius_mm,
            "Petiole radius {} must exceed apex radius {}",
            petiole.radius_mm,
            midrib_apex.radius_mm
        );
        assert!(petiole.conductance > midrib_apex.conductance);
    }

    #[test]
    fn test_turgor_loss_wilting() {
        let profile = LeafEconomicsProfile::herbaceous_broadleaf();
        let turgid = LeafVascularSkeleton::new(profile, 1.5);
        let wilted = LeafVascularSkeleton::new(profile, 0.0);

        // Effective modulus should drop dramatically without turgor
        assert!(turgid.effective_elastic_modulus_mpa() > wilted.effective_elastic_modulus_mpa() * 10.0);

        // Wilted leaf droop angle must be significantly higher
        let droop_turgid = turgid.evaluate_droop_angle(1.0);
        let droop_wilted = wilted.evaluate_droop_angle(1.0);
        assert!(
            droop_wilted > droop_turgid,
            "Wilted droop {} must exceed turgid droop {}",
            droop_wilted,
            droop_turgid
        );
    }
}
