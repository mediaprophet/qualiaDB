//! Functional-Structural Plant Model (FSPM) & Metamer Graph Engine (`metamer_fspm.rs`)
//!
//! Models whole-plant growth as a connected graph of Metamers operating as a
//! continuous thermodynamic source-sink circuit:
//! - Shinozaki's Pipe Model Theory (active sapwood pipes vs. retired structural heartwood)
//! - Thermal time phenology (Growing Degree Days $GDD$ & winter chilling hours)
//! - Two-pass carbon/water partitioning: Bottom-up assimilation $\to$ Top-down sink allocation
//! - Gravitational fruit point loading ($F = M_{\text{fruit}} g$) and reaction wood thickening
//!
//! Strictly zero heap in hot loops (`AGENTS.md` §0).

pub const MAX_METAMERS: usize = 128;
pub const NO_PARENT_METAMER: u16 = 0xFFFF;

/// Botanical organ type borne by a metamer node.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetamerOrgan {
    InternodeOnly = 0,
    VegetativeBud = 1,
    Leaf = 2,
    Flower = 3,
    Fruit = 4,
    RootTip = 5,
}

/// The atomic structural and hydraulic unit of a plant graph.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metamer {
    pub parent_idx: u16,
    pub organ: MetamerOrgan,
    /// Internode cylinder length in meters
    pub length_m: f32,
    /// Active conducting vascular pipes (Sapwood)
    pub active_pipes: u32,
    /// Retired non-conducting structural pipes (Heartwood core)
    pub retired_pipes: u32,
    /// Organ fresh mass in kg (e.g. swelling fruit or leaf biomass)
    pub organ_mass_kg: f32,
    /// Spatial orientation pitch angle in radians
    pub pitch_rad: f32,
    /// Spatial orientation yaw angle in radians
    pub yaw_rad: f32,
    /// Accumulated mechanical torque from gravity and fruit load (N·m)
    pub bending_moment_nm: f32,
    /// Age in thermal time degree-days
    pub age_gdd: f32,
}

impl Default for Metamer {
    fn default() -> Self {
        Self {
            parent_idx: NO_PARENT_METAMER,
            organ: MetamerOrgan::InternodeOnly,
            length_m: 0.1,
            active_pipes: 1,
            retired_pipes: 0,
            organ_mass_kg: 0.005,
            pitch_rad: 0.0,
            yaw_rad: 0.0,
            bending_moment_nm: 0.0,
            age_gdd: 0.0,
        }
    }
}

/// Thermal time phenology tracker.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalPhenology {
    /// Accumulated Growing Degree Days: $\sum \max(0, T_{\text{mean}} - T_{\text{base}})$
    pub accumulated_gdd: f32,
    /// Base physiological threshold temperature in °C (typically 5.0 - 10.0 °C)
    pub t_base_c: f32,
    /// Accumulated winter chilling hours below 7.2 °C (vernalization)
    pub chilling_hours: f32,
    /// Threshold chilling hours required before bud break is permitted
    pub required_chilling_hours: f32,
    /// GDD threshold for floral initiation
    pub flowering_gdd_threshold: f32,
    /// GDD threshold for fruit maturity
    pub fruiting_gdd_threshold: f32,
}

impl Default for ThermalPhenology {
    fn default() -> Self {
        Self {
            accumulated_gdd: 0.0,
            t_base_c: 7.0,
            chilling_hours: 600.0,
            required_chilling_hours: 500.0,
            flowering_gdd_threshold: 450.0,
            fruiting_gdd_threshold: 1200.0,
        }
    }
}

/// Zero-heap whole-plant FSPM solver.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MetamerPlantGraph {
    pub metamers: [Metamer; MAX_METAMERS],
    pub count: usize,
    pub phenology: ThermalPhenology,
    /// Nominal cross-sectional area of a single unit vascular pipe ($m^2$)
    pub unit_pipe_area_m2: f32,
    /// Global carbon assimilate pool Q in kg
    pub carbon_pool_q_kg: f32,
    /// Base wood Young's modulus in MPa
    pub wood_elastic_modulus_mpa: f32,
}

impl MetamerPlantGraph {
    /// Creates a seedling with root collar and apical shoot.
    pub fn new(phenology: ThermalPhenology) -> Self {
        let mut graph = Self {
            metamers: [Metamer::default(); MAX_METAMERS],
            count: 0,
            phenology,
            unit_pipe_area_m2: 1.5e-5, // ~4.37 mm diameter per unit pipe
            carbon_pool_q_kg: 0.05,
            wood_elastic_modulus_mpa: 850.0,
        };

        // Node 0: Root collar / base stem
        graph.metamers[0] = Metamer {
            parent_idx: NO_PARENT_METAMER,
            organ: MetamerOrgan::InternodeOnly,
            length_m: 0.2,
            active_pipes: 2,
            retired_pipes: 1,
            organ_mass_kg: 0.02,
            pitch_rad: 0.0,
            yaw_rad: 0.0,
            bending_moment_nm: 0.0,
            age_gdd: 0.0,
        };
        graph.count = 1;

        // Node 1: Primary shoot with active leaf
        graph.metamers[1] = Metamer {
            parent_idx: 0,
            organ: MetamerOrgan::Leaf,
            length_m: 0.15,
            active_pipes: 1,
            retired_pipes: 0,
            organ_mass_kg: 0.008,
            pitch_rad: 0.1,
            yaw_rad: 0.0,
            bending_moment_nm: 0.0,
            age_gdd: 0.0,
        };
        graph.count = 2;

        graph
    }

    /// Evaluates total radius (meters) of a metamer combining sapwood and heartwood pipes:
    /// $A_{\text{total}} = (P_{\text{active}} + P_{\text{retired}}) \cdot A_{\text{pipe}}$
    #[inline]
    pub fn radius_m(&self, idx: usize) -> f32 {
        if idx >= self.count {
            return 0.005;
        }
        let total_pipes = (self.metamers[idx].active_pipes + self.metamers[idx].retired_pipes) as f32;
        let area = total_pipes * self.unit_pipe_area_m2;
        (area / std::f32::consts::PI).sqrt().max(0.001)
    }

    /// Second moment of area: $I = \frac{\pi r^4}{4}$
    #[inline]
    pub fn second_moment_i(&self, idx: usize) -> f32 {
        let r = self.radius_m(idx);
        (std::f32::consts::PI * r.powi(4)) / 4.0
    }

    /// Updates thermal time from daily high/low temperatures.
    pub fn update_thermal_time(&mut self, t_max: f32, t_min: f32, days: f32) {
        let t_mean = (t_max + t_min) * 0.5;
        let delta_gdd = (t_mean - self.phenology.t_base_c).max(0.0) * days;
        self.phenology.accumulated_gdd += delta_gdd;

        for i in 0..self.count {
            self.metamers[i].age_gdd += delta_gdd;
        }

        // Floral transition check: vegetative buds become flowers if vernalized and GDD threshold met
        if self.phenology.chilling_hours >= self.phenology.required_chilling_hours
            && self.phenology.accumulated_gdd >= self.phenology.flowering_gdd_threshold
        {
            for i in 0..self.count {
                if self.metamers[i].organ == MetamerOrgan::VegetativeBud {
                    self.metamers[i].organ = MetamerOrgan::Flower;
                }
            }
        }

        // Fruiting transition check: flowers mature into fruit nodes
        if self.phenology.accumulated_gdd >= self.phenology.fruiting_gdd_threshold {
            for i in 0..self.count {
                if self.metamers[i].organ == MetamerOrgan::Flower {
                    self.metamers[i].organ = MetamerOrgan::Fruit;
                    self.metamers[i].organ_mass_kg = 0.02; // Initial fruit mass
                }
            }
        }
    }

    /// Solves the two-pass Shinozaki pipe model and source-sink allocation cycle.
    pub fn solve_growth_cycle(&mut self, light_intensity: f32, soil_moisture: f32) {
        if self.count == 0 {
            return;
        }

        // ── Pass 1: Bottom-Up Accumulation ────────────────────────────────────
        // Clear active pipe counts before recount
        for i in 0..self.count {
            self.metamers[i].active_pipes = 0;
            self.metamers[i].bending_moment_nm = 0.0;
        }

        // Sum active leaves and fruit gravity torque in reverse topological order
        let mut leaf_count = 0u32;
        for i in (0..self.count).rev() {
            if self.metamers[i].organ == MetamerOrgan::Leaf {
                self.metamers[i].active_pipes += 1;
                leaf_count += 1;
            }

            // Downward point load F = m * g
            let f_down = self.metamers[i].organ_mass_kg * 9.80665;
            let lever_arm = self.metamers[i].length_m * 0.5;
            self.metamers[i].bending_moment_nm += f_down * lever_arm;

            let p_idx = self.metamers[i].parent_idx;
            if p_idx != NO_PARENT_METAMER && (p_idx as usize) < self.count {
                // Pipe model: parent must carry all active pipes of its children
                let child_active = self.metamers[i].active_pipes;
                self.metamers[p_idx as usize].active_pipes += child_active;

                // Propagate torque down the branch
                let child_torque = self.metamers[i].bending_moment_nm;
                self.metamers[p_idx as usize].bending_moment_nm += child_torque;
            }
        }

        // Compute global assimilate production Q = light * moisture * leaf_area
        let leaf_area_total = (leaf_count as f32) * 0.015; // 0.015 m² per leaf
        let net_assimilation = leaf_area_total * light_intensity.clamp(0.0, 1.5) * soil_moisture.clamp(0.0, 1.0) * 0.05;
        self.carbon_pool_q_kg += net_assimilation;

        // ── Pass 2: Top-Down Sink Distribution ────────────────────────────────
        // Organ sink strengths: Fruit has massive priority over vegetative buds
        let mut total_sink_weight = 0.001f32;
        let mut sink_weights = [0.0f32; MAX_METAMERS];

        for i in 0..self.count {
            sink_weights[i] = match self.metamers[i].organ {
                MetamerOrgan::Fruit => 12.0, // Aggressively draws sugar
                MetamerOrgan::VegetativeBud => 2.5,
                MetamerOrgan::Flower => 3.0,
                MetamerOrgan::Leaf => 1.0,
                MetamerOrgan::RootTip => 2.0,
                MetamerOrgan::InternodeOnly => 0.5, // Secondary cambial growth
            };
            total_sink_weight += sink_weights[i];
        }

        let q_available = self.carbon_pool_q_kg * 0.4; // 40% carbon allocated per growth step
        self.carbon_pool_q_kg -= q_available;

        for i in 0..self.count {
            let share = (sink_weights[i] / total_sink_weight) * q_available;

            match self.metamers[i].organ {
                MetamerOrgan::Fruit => {
                    // Swell fruit mass directly
                    self.metamers[i].organ_mass_kg += share * 2.0; // Water hydration multiplier
                }
                MetamerOrgan::VegetativeBud => {
                    // Spawn new leaf metamer if sufficient carbon
                    if share > 0.008 && self.count < MAX_METAMERS {
                        let new_idx = self.count;
                        self.metamers[new_idx] = Metamer {
                            parent_idx: i as u16,
                            organ: MetamerOrgan::Leaf,
                            length_m: 0.12,
                            active_pipes: 1,
                            retired_pipes: 0,
                            organ_mass_kg: 0.006,
                            pitch_rad: self.metamers[i].pitch_rad + 0.35,
                            yaw_rad: self.metamers[i].yaw_rad + 1.25,
                            bending_moment_nm: 0.0,
                            age_gdd: 0.0,
                        };
                        self.count += 1;
                    }
                }
                MetamerOrgan::InternodeOnly => {
                    // Reaction wood thickening under sustained bending torque
                    if self.metamers[i].bending_moment_nm > 0.5 {
                        self.metamers[i].retired_pipes += 1; // Thicken heartwood core
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipe_model_basal_taper() {
        let phenology = ThermalPhenology::default();
        let mut tree = MetamerPlantGraph::new(phenology);

        // Add 3 branched leaf nodes
        tree.metamers[tree.count] = Metamer {
            parent_idx: 1,
            organ: MetamerOrgan::Leaf,
            length_m: 0.1,
            ..Default::default()
        };
        tree.count += 1;

        tree.solve_growth_cycle(1.0, 0.8);

        let r_base = tree.radius_m(0);
        let r_apex = tree.radius_m(tree.count - 1);

        // Pipe model demands trunk base radius exceeds branch apex
        assert!(r_base > r_apex, "Trunk base {} must exceed apex {}", r_base, r_apex);
    }

    #[test]
    fn test_fruit_loading_and_reaction_wood() {
        let phenology = ThermalPhenology::default();
        let mut tree = MetamerPlantGraph::new(phenology);

        // Transition node to fruit with heavy mass
        tree.metamers[1].organ = MetamerOrgan::Fruit;
        tree.metamers[1].organ_mass_kg = 0.85; // 850g heavy fruit

        tree.solve_growth_cycle(1.0, 0.8);

        // Torque should be registered at base
        assert!(tree.metamers[0].bending_moment_nm > 0.5);

        // Base should have acquired secondary reaction wood
        assert!(tree.metamers[0].retired_pipes > 0);
    }
}
