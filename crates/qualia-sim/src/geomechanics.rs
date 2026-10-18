//! Geotechnical Soil Mechanics & Excavation Kinematics (`geomechanics.rs`)
//! Zero-heap continuum physics: hand spade, pickaxe, excavator, fossorial claws vs mouse raycast.
//! Mohr-Coulomb shear, Wu-Waldron root mat, Mckyes-Ali cutting force, bulking, and track compaction.
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, deterministic).

/// Canonical geotechnical soil failure and physical mechanics profile.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilMechanicsProfile {
    /// Effective cohesion $c'$ in kPa (pure sand ~0, loam ~10, dense clay ~50+).
    pub cohesion_kpa: f32,
    /// Effective internal friction angle $\phi$ in radians (sand ~0.60 rad / 34°, clay ~0.35 rad / 20°).
    pub friction_angle_rad: f32,
    /// Soil unit weight / bulk density $\gamma$ in kN/m³ (typical 15.0 - 20.0 kN/m³).
    pub bulk_density_kn_m3: f32,
    /// Pore water pressure $u$ in kPa (negative for matric suction in damp soil, positive in saturated mud).
    pub pore_water_pressure_kpa: f32,
    /// Wu-Waldron root tensile cohesion reinforcement $\Delta c_{\text{root}}$ in kPa.
    pub root_tensile_reinforcement_kpa: f32,
    /// Soil-tool adhesion $c_a$ in kPa (sticky clay adhering to tool face).
    pub soil_tool_adhesion_kpa: f32,
    /// Volumetric bulking expansion factor $B_f = V_{\text{loose}} / V_{\text{bank}}$ (typically 1.15 - 1.35).
    pub bulking_factor: f32,
    /// Critical angle of repose for loose spoil heap in radians ($\theta_r \approx \phi$).
    pub angle_of_repose_rad: f32,
    /// Unconfined compressive strength $q_u$ in kPa (identifies lithic refusal threshold).
    pub unconfined_compressive_strength_kpa: f32,
}

impl Default for SoilMechanicsProfile {
    fn default() -> Self {
        Self::loam()
    }
}

impl SoilMechanicsProfile {
    /// Surface humic root-mat epipedon (Depth 1: 0–5 cm, dense subalpine grass roots).
    pub fn humic_root_mat() -> Self {
        Self {
            cohesion_kpa: 12.0,
            friction_angle_rad: 0.5236, // 30°
            bulk_density_kn_m3: 13.5,
            pore_water_pressure_kpa: -5.0, // slight matric suction
            root_tensile_reinforcement_kpa: 28.0, // strong root fibrous resistance
            soil_tool_adhesion_kpa: 4.0,
            bulking_factor: 1.30,
            angle_of_repose_rad: 0.6108, // 35°
            unconfined_compressive_strength_kpa: 120.0,
        }
    }

    /// Friable sandy loam topsoil (Depth 2: 5–15 cm).
    pub fn loam() -> Self {
        Self {
            cohesion_kpa: 10.0,
            friction_angle_rad: 0.5585, // 32°
            bulk_density_kn_m3: 16.5,
            pore_water_pressure_kpa: -2.0,
            root_tensile_reinforcement_kpa: 5.0,
            soil_tool_adhesion_kpa: 2.0,
            bulking_factor: 1.25,
            angle_of_repose_rad: 0.5585, // 32°
            unconfined_compressive_strength_kpa: 80.0,
        }
    }

    /// Dense plastic illuvial clay (Depth 4: 30–60 cm).
    pub fn illuvial_clay() -> Self {
        Self {
            cohesion_kpa: 55.0,
            friction_angle_rad: 0.3491, // 20°
            bulk_density_kn_m3: 19.5,
            pore_water_pressure_kpa: 0.0,
            root_tensile_reinforcement_kpa: 0.0,
            soil_tool_adhesion_kpa: 22.0, // very sticky
            bulking_factor: 1.35,
            angle_of_repose_rad: 0.4363, // 25°
            unconfined_compressive_strength_kpa: 250.0,
        }
    }

    /// Weathered gravelly saprolite / bedrock transition (Depth 5: 60–100 cm).
    pub fn saprolite() -> Self {
        Self {
            cohesion_kpa: 30.0,
            friction_angle_rad: 0.6981, // 40° (high friction from angular cobbles)
            bulk_density_kn_m3: 21.0,
            pore_water_pressure_kpa: 0.0,
            root_tensile_reinforcement_kpa: 0.0,
            soil_tool_adhesion_kpa: 0.5,
            bulking_factor: 1.18,
            angle_of_repose_rad: 0.6981, // 40°
            unconfined_compressive_strength_kpa: 850.0,
        }
    }

    /// Solid lithic bedrock (granite / basalt).
    pub fn bedrock() -> Self {
        Self {
            cohesion_kpa: 5_000.0,
            friction_angle_rad: 0.7854, // 45°
            bulk_density_kn_m3: 26.5,
            pore_water_pressure_kpa: 0.0,
            root_tensile_reinforcement_kpa: 0.0,
            soil_tool_adhesion_kpa: 0.0,
            bulking_factor: 1.50,
            angle_of_repose_rad: 0.7854,
            unconfined_compressive_strength_kpa: 65_000.0, // 65 MPa
        }
    }

    /// Adjusts soil mechanical resistance across diurnal freeze-thaw and annual seasons.
    ///
    /// - $T \le 0^\circ\text{C}$: Cryo-cementation (needle ice / permafrost crust) causes
    ///   cohesion and compressive strength to jump dramatically, leading to spade refusal.
    /// - $S_w > 0.80$: Spring snowmelt or saturated rain induces positive pore pressure ($u > 0$),
    ///   dropping effective stress while boosting clay-tool adhesion ($c_a$).
    /// - $S_w < 0.20$: Summer drought creates strong matric suction ($u \ll 0$), baking clay
    ///   into hard clods and generating airborne dust under mechanical impacts.
    pub fn with_seasonal_state(&self, temp_c: f32, moisture_fraction: f32) -> Self {
        let mut s = *self;
        let moisture = moisture_fraction.clamp(0.02, 1.0);

        if temp_c <= 0.0 {
            // Subalpine winter freezing / cryo-cementation
            let frost_severity = (temp_c.abs() * 0.4).min(4.0);
            s.cohesion_kpa += 180.0 * frost_severity;
            s.unconfined_compressive_strength_kpa += 1_200.0 * frost_severity;
            s.soil_tool_adhesion_kpa = 0.0; // frozen soil does not adhere
            s.bulking_factor = 1.12;
        } else if moisture > 0.80 {
            // Saturated spring snowmelt / winter rain
            let excess = (moisture - 0.80) / 0.20;
            s.pore_water_pressure_kpa = 12.0 * excess; // positive pore pressure
            s.cohesion_kpa = (s.cohesion_kpa * (1.0 - 0.60 * excess)).max(1.0);
            s.soil_tool_adhesion_kpa *= 1.7; // sticky clay
        } else if moisture < 0.20 {
            // Summer drought baking
            let drought = (0.20 - moisture) / 0.18;
            s.pore_water_pressure_kpa = -120.0 * drought;
            s.cohesion_kpa += 35.0 * drought;
            s.unconfined_compressive_strength_kpa += 300.0 * drought;
            s.soil_tool_adhesion_kpa *= 0.2;
        }
        s
    }
}

/// Indigenous and ecological seasonal categories for East Coast Australia.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AustralianSeason {
    /// Winter Snow & Frost (June–August): frozen soil crust, subnivean space.
    WinterSnowFrost = 0,
    /// Spring Snowmelt & Budburst (September–October): saturated mud, wattle bloom.
    SpringSnowmelt = 1,
    /// Early Summer Warmth (November–December): friable easy working, peak root growth.
    EarlySummerWarmth = 2,
    /// Late Summer Drought (January–February): baked clay, cracking soil, high fire risk.
    LateSummerDrought = 3,
    /// Early Autumn Smoke (March): post-fire ash crust, cooling nights.
    EarlyAutumnSmoke = 4,
    /// Late Autumn Pre-Winter (April–May): re-wetting, fungal flushes, geosmin release.
    LateAutumnPreWinter = 5,
}

/// Tool physical class distinguishing human implements, machinery, and abstract clicks.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// Mouse pointer or camera raycast: dimensionless observational probe (0 N force).
    ObservationRay = 0,
    /// Manual hand spade or garden shovel (wedge cutting blade, foot drive).
    HandSpade = 1,
    /// Manual pickaxe or mattock (high-impulse point chisel for brittle fracture).
    Pickaxe = 2,
    /// Heavy hydraulic excavator bucket with Ripper teeth.
    ExcavatorBucket = 3,
    /// Animal fossorial scratching claws (Wombat *Vombatus ursinus*).
    FossorialClaw = 4,
}

/// Kinematic tool effector parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolEffector {
    /// Category of tool or actuator.
    pub kind: ToolKind,
    /// Transverse blade or contact edge width in meters.
    pub blade_width_m: f32,
    /// Forward rake angle relative to soil surface in radians (typically 0.5 - 1.2 rad).
    pub rake_angle_rad: f32,
    /// Sharpness wedge angle at the cutting tip in radians.
    pub wedge_angle_rad: f32,
    /// Maximum normal/breakout force available from operator or hydraulic ram in Newtons.
    pub applied_force_n: f32,
    /// Operator applied prying torque in Newton-meters.
    pub applied_torque_nm: f32,
    /// Maximum scoop or bucket volumetric capacity in cubic meters.
    pub capacity_volume_m3: f32,
}

impl ToolEffector {
    /// Pure mouse click / pointer gaze (informational, non-destructive).
    pub fn mouse_click() -> Self {
        Self {
            kind: ToolKind::ObservationRay,
            blade_width_m: 0.0,
            rake_angle_rad: 1.5708, // 90°
            wedge_angle_rad: 0.0,
            applied_force_n: 0.0, // Zero force
            applied_torque_nm: 0.0,
            capacity_volume_m3: 0.0,
        }
    }

    /// Standard steel trenching spade operated by an adult human with foot-step drive.
    pub fn hand_spade() -> Self {
        Self {
            kind: ToolKind::HandSpade,
            blade_width_m: 0.20,     // 20 cm wide blade
            rake_angle_rad: 1.0472,  // 60° plunge angle
            wedge_angle_rad: 0.3491, // 20° beveled edge
            applied_force_n: 650.0,  // ~66 kg operator driving with boot heel
            applied_torque_nm: 95.0, // Prying leverage on handle
            capacity_volume_m3: 0.005, // 5 liters scoop volume
        }
    }

    /// Pointed pickaxe swing.
    pub fn pickaxe() -> Self {
        Self {
            kind: ToolKind::Pickaxe,
            blade_width_m: 0.025,    // 2.5 cm point tip
            rake_angle_rad: 1.2217,  // 70° impact angle
            wedge_angle_rad: 0.4363, // 25° forged tip
            applied_force_n: 2_400.0, // Dynamic impact force ($F = \Delta p / \Delta t$)
            applied_torque_nm: 40.0,
            capacity_volume_m3: 0.001,
        }
    }

    /// Mini-excavator 5-tonne backhoe bucket.
    pub fn mini_excavator() -> Self {
        Self {
            kind: ToolKind::ExcavatorBucket,
            blade_width_m: 0.60,      // 60 cm bucket with 4 teeth
            rake_angle_rad: 0.7854,   // 45° crowd angle
            wedge_angle_rad: 0.5236,  // 30° tooth wedge
            applied_force_n: 38_000.0, // 38 kN hydraulic breakout force
            applied_torque_nm: 4_500.0,
            capacity_volume_m3: 0.18,  // 180 liters
        }
    }

    /// Wombat forelimb fossorial claws (*Vombatus ursinus*).
    pub fn wombat_claws() -> Self {
        Self {
            kind: ToolKind::FossorialClaw,
            blade_width_m: 0.06,     // 6 cm total 5-claw spread
            rake_angle_rad: 0.8727,  // 50° scratching stroke
            wedge_angle_rad: 0.3840, // 22° sharp keratinous curves
            applied_force_n: 140.0,  // Forelimb muscular force
            applied_torque_nm: 12.0,
            capacity_volume_m3: 0.0015, // 1.5 liters per scratch cycle
        }
    }
}

/// The physical outcome of a tool contact attempt on a soil stratum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExcavationVerdict {
    /// True if the applied tool force exceeded geotechnical cutting resistance.
    pub severed: bool,
    /// Total calculated cutting force required to displace the target volume (N).
    pub required_force_n: f32,
    /// Net margin force (applied - required). Positive means easy cut; negative means stall.
    pub margin_force_n: f32,
    /// Solid in-situ bank volume removed from terrain (m³).
    pub excavated_bank_volume_m3: f32,
    /// Expanded loose volume ready for tool bucket or spoil heap (m³).
    pub excavated_loose_volume_m3: f32,
    /// True if root mat tensile reinforcement was severed during the pass.
    pub root_mat_severed: bool,
    /// True if stratum was too rigid for the tool (e.g. spade hitting basalt rock).
    pub refusal_hit: bool,
}

/// Evaluates Mckyes-Ali earthmoving cutting resistance and geotechnical failure.
///
/// Returns an `ExcavationVerdict` detailing whether the tool slices, stalls, or refuses.
pub fn evaluate_excavation_pass(
    tool: &ToolEffector,
    target_depth_m: f32,
    soil: &SoilMechanicsProfile,
) -> ExcavationVerdict {
    // 1. Mouse click / observation gaze: strictly 0 force, cannot deform terrain
    if tool.kind == ToolKind::ObservationRay || tool.applied_force_n <= 0.0 {
        return ExcavationVerdict {
            severed: false,
            required_force_n: 0.0,
            margin_force_n: 0.0,
            excavated_bank_volume_m3: 0.0,
            excavated_loose_volume_m3: 0.0,
            root_mat_severed: false,
            refusal_hit: false,
        };
    }

    let d = target_depth_m.max(0.01);
    let w = tool.blade_width_m.max(0.01);

    // 2. Lithic refusal check: if soil compressive strength exceeds tool capability
    let max_penetrable_ucs = match tool.kind {
        ToolKind::ObservationRay => 0.0,
        ToolKind::HandSpade => 400.0,       // Hand spade halts on hardpan / soft rock
        ToolKind::FossorialClaw => 300.0,   // Claws cannot dig solid shale/rock
        ToolKind::Pickaxe => 35_000.0,      // Pickaxe fractures soft to medium rock
        ToolKind::ExcavatorBucket => 12_000.0, // Excavator teeth rip gravel/caliche, not solid granite
    };

    if soil.unconfined_compressive_strength_kpa > max_penetrable_ucs {
        return ExcavationVerdict {
            severed: false,
            required_force_n: soil.unconfined_compressive_strength_kpa * w * d * 100.0,
            margin_force_n: tool.applied_force_n - 100_000.0,
            excavated_bank_volume_m3: 0.0,
            excavated_loose_volume_m3: 0.0,
            root_mat_severed: false,
            refusal_hit: true,
        };
    }

    // 3. Mohr-Coulomb effective cohesion including Wu-Waldron root reinforcement
    let effective_cohesion = soil.cohesion_kpa + soil.root_tensile_reinforcement_kpa;

    // 4. Mckyes-Ali N-coefficients & wedge cutting factors
    let phi = soil.friction_angle_rad.max(0.1);
    let n_gamma = (tool.rake_angle_rad * 1.8).sin() * (1.0 + phi.tan() * 2.2);
    let n_c = (tool.rake_angle_rad).cos().abs() + 2.0 * (1.0 + phi.sin()) / (1.0 - phi.sin());
    let n_ca = 1.0 + (tool.rake_angle_rad).cos();

    // Sharp beveled cutting edges (spades, ripper teeth) cleave rather than displace a massive passive wedge
    let wedge_reduction = (tool.wedge_angle_rad * 0.5).sin().clamp(0.18, 1.0);

    // 5. Total cutting force P = [gamma * d^2 * N_gamma * w_red + c * d * N_c * w_red + c_a * d * N_ca] * w
    let unit_weight = soil.bulk_density_kn_m3;
    let pressure_kpa = unit_weight * d * d * n_gamma * wedge_reduction
        + effective_cohesion * d * n_c * wedge_reduction
        + soil.soil_tool_adhesion_kpa * d * n_ca;
    let required_force_n = (pressure_kpa * w * 1_000.0).max(10.0);

    let severed = tool.applied_force_n >= required_force_n;
    let margin = tool.applied_force_n - required_force_n;

    if severed {
        let bank_cut_volume = (w * d * d * 0.75).min(tool.capacity_volume_m3);
        let loose_volume = bank_cut_volume * soil.bulking_factor;
        let root_severed = soil.root_tensile_reinforcement_kpa > 0.0;

        ExcavationVerdict {
            severed: true,
            required_force_n,
            margin_force_n: margin,
            excavated_bank_volume_m3: bank_cut_volume,
            excavated_loose_volume_m3: loose_volume,
            root_mat_severed: root_severed,
            refusal_hit: false,
        }
    } else {
        ExcavationVerdict {
            severed: false,
            required_force_n,
            margin_force_n: margin,
            excavated_bank_volume_m3: 0.0,
            excavated_loose_volume_m3: 0.0,
            root_mat_severed: false,
            refusal_hit: false,
        }
    }
}

/// Evaluates topsoil compaction under vehicular tracks or footsteps.
pub fn evaluate_soil_compaction(
    contact_pressure_kpa: f32,
    current_porosity: f32,
    soil: &SoilMechanicsProfile,
) -> (f32, f32) {
    if contact_pressure_kpa <= 0.0 {
        return (current_porosity, 1.0);
    }

    // Compression index C_c for topsoil (humic/clay loam C_c ~ 0.20-0.35)
    let c_c = (0.20 + (soil.cohesion_kpa / 100.0) * 0.15).clamp(0.12, 0.45);
    let initial_effective_stress = 15.0; // ~1m overburden
    let delta_e = c_c * ((initial_effective_stress + contact_pressure_kpa) / initial_effective_stress).log10();

    let initial_void_ratio = current_porosity / (1.0 - current_porosity).max(0.01);
    let final_void_ratio = (initial_void_ratio - delta_e).max(0.20);
    let final_porosity = final_void_ratio / (1.0 + final_void_ratio);

    // Kozeny-Carman hydraulic conductivity degradation: K ~ e^3 / (1 + e)
    let initial_kc = (initial_void_ratio * initial_void_ratio * initial_void_ratio) / (1.0 + initial_void_ratio);
    let final_kc = (final_void_ratio * final_void_ratio * final_void_ratio) / (1.0 + final_void_ratio);
    let k_multiplier = (final_kc / initial_kc.max(1e-4)).clamp(0.001, 1.0);

    (final_porosity, k_multiplier)
}

/// Critical maximum unsupported vertical trench wall depth before geotechnical collapse (Rankine slip).
pub fn critical_trench_depth_m(soil: &SoilMechanicsProfile) -> f32 {
    let phi = soil.friction_angle_rad;
    let rankine_ka = (1.0 - phi.sin()) / (1.0 + phi.sin()).max(1e-3);
    let sqrt_ka = rankine_ka.sqrt();

    // H_c = (4 * c) / (gamma * sqrt(K_a))
    let total_c = soil.cohesion_kpa + soil.root_tensile_reinforcement_kpa;
    (4.0 * total_c) / (soil.bulk_density_kn_m3 * sqrt_ka).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_click_never_excavates() {
        let mouse = ToolEffector::mouse_click();
        let loam = SoilMechanicsProfile::loam();
        let verdict = evaluate_excavation_pass(&mouse, 0.15, &loam);
        assert!(!verdict.severed);
        assert_eq!(verdict.excavated_bank_volume_m3, 0.0);
    }

    #[test]
    fn hand_spade_slices_friable_loam() {
        let spade = ToolEffector::hand_spade();
        let loam = SoilMechanicsProfile::loam();
        // 10 cm slice in friable topsoil should yield cleanly
        let verdict = evaluate_excavation_pass(&spade, 0.10, &loam);
        assert!(verdict.severed);
        assert!(verdict.excavated_loose_volume_m3 > verdict.excavated_bank_volume_m3);
        assert!(!verdict.refusal_hit);
    }

    #[test]
    fn hand_spade_refuses_on_solid_bedrock() {
        let spade = ToolEffector::hand_spade();
        let rock = SoilMechanicsProfile::bedrock();
        let verdict = evaluate_excavation_pass(&spade, 0.05, &rock);
        assert!(!verdict.severed);
        assert!(verdict.refusal_hit);
    }

    #[test]
    fn excavator_rips_deep_dense_clay() {
        let spade = ToolEffector::hand_spade();
        let excavator = ToolEffector::mini_excavator();
        let clay = SoilMechanicsProfile::illuvial_clay();

        // Hand spade stalls in deep dense clay (35 cm deep)
        let spade_verdict = evaluate_excavation_pass(&spade, 0.35, &clay);
        assert!(!spade_verdict.severed);

        // Heavy excavator bucket breaks through
        let excavator_verdict = evaluate_excavation_pass(&excavator, 0.35, &clay);
        assert!(excavator_verdict.severed);
        assert!(excavator_verdict.excavated_loose_volume_m3 > 0.05);
    }

    #[test]
    fn track_compaction_reduces_permeability() {
        let loam = SoilMechanicsProfile::loam();
        let contact_pressure_kpa = 250.0; // Heavy machine track
        let initial_porosity = 0.45;
        let (new_n, k_ratio) = evaluate_soil_compaction(contact_pressure_kpa, initial_porosity, &loam);
        assert!(new_n < initial_porosity);
        assert!(k_ratio < 0.5); // Permeability significantly reduced
    }

    #[test]
    fn seasonal_frost_and_snowmelt_response() {
        let spade = ToolEffector::hand_spade();
        let loam = SoilMechanicsProfile::loam();

        // Subalpine winter frost (-5°C): cryo-cementation causes refusal
        let frozen = loam.with_seasonal_state(-5.0, 0.40);
        let frozen_verdict = evaluate_excavation_pass(&spade, 0.10, &frozen);
        assert!(!frozen_verdict.severed);
        assert!(frozen_verdict.refusal_hit);

        // Spring snowmelt (95% saturated): easy shear cut, high clay adhesion
        let spring = loam.with_seasonal_state(12.0, 0.95);
        let spring_verdict = evaluate_excavation_pass(&spade, 0.10, &spring);
        assert!(spring_verdict.severed);
        assert!(spring.soil_tool_adhesion_kpa > loam.soil_tool_adhesion_kpa);
    }
}
