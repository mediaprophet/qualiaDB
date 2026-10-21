//! MCP tools for Geotechnical Soil Mechanics, Compaction, and Excavation Kinematics.
//!
//! Powered by `qualia-sim::geomechanics`.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_sim::geomechanics::{
    critical_trench_depth_m, evaluate_excavation_pass, evaluate_soil_compaction,
    SoilMechanicsProfile, ToolEffector, ToolKind,
};
use serde_json::{json, Value};

fn parse_soil_profile(v: &Value) -> SoilMechanicsProfile {
    if let Some(name) = v.get("soil_preset").and_then(Value::as_str) {
        let base = match name.to_lowercase().as_str() {
            "bedrock" | "rock" => SoilMechanicsProfile::bedrock(),
            "clay" | "dense_clay" => SoilMechanicsProfile::illuvial_clay(),
            "humic" | "root_mat" => SoilMechanicsProfile::humic_root_mat(),
            "saprolite" | "sand" | "dry_sand" => SoilMechanicsProfile::saprolite(),
            _ => SoilMechanicsProfile::loam(),
        };
        let temp = v.get("temperature_c").and_then(Value::as_f64).map(|t| t as f32);
        let sat = v.get("saturation_ratio").and_then(Value::as_f64).map(|s| s as f32);
        if temp.is_some() || sat.is_some() {
            return base.with_seasonal_state(temp.unwrap_or(15.0), sat.unwrap_or(0.35));
        }
        return base;
    }

    SoilMechanicsProfile {
        cohesion_kpa: v.get("cohesion_kpa").and_then(Value::as_f64).unwrap_or(10.0) as f32,
        friction_angle_rad: v.get("friction_angle_rad").and_then(Value::as_f64).unwrap_or(0.488) as f32,
        bulk_density_kn_m3: v.get("bulk_density_kn_m3").and_then(Value::as_f64).unwrap_or(16.5) as f32,
        pore_water_pressure_kpa: v.get("pore_water_pressure_kpa").and_then(Value::as_f64).unwrap_or(-15.0) as f32,
        root_tensile_reinforcement_kpa: v.get("root_tensile_reinforcement_kpa").and_then(Value::as_f64).unwrap_or(5.0) as f32,
        soil_tool_adhesion_kpa: v.get("soil_tool_adhesion_kpa").and_then(Value::as_f64).unwrap_or(2.5) as f32,
        bulking_factor: v.get("bulking_factor").and_then(Value::as_f64).unwrap_or(1.25) as f32,
        angle_of_repose_rad: v.get("angle_of_repose_rad").and_then(Value::as_f64).unwrap_or(0.558) as f32,
        unconfined_compressive_strength_kpa: v.get("unconfined_compressive_strength_kpa").and_then(Value::as_f64).unwrap_or(85.0) as f32,
    }
}

fn parse_tool_effector(v: &Value) -> ToolEffector {
    if let Some(kind_str) = v.get("tool_preset").and_then(Value::as_str) {
        return match kind_str.to_lowercase().as_str() {
            "pickaxe" => ToolEffector::pickaxe(),
            "mini_excavator" | "excavator" => ToolEffector::mini_excavator(),
            "fossorial_claw" | "claw" => ToolEffector::wombat_claws(),
            "raycast" | "mouse" => ToolEffector::mouse_click(),
            _ => ToolEffector::hand_spade(),
        };
    }

    let kind = match v.get("kind").and_then(Value::as_str).unwrap_or("spade") {
        "pickaxe" => ToolKind::Pickaxe,
        "excavator" => ToolKind::ExcavatorBucket,
        "claw" => ToolKind::FossorialClaw,
        "raycast" => ToolKind::ObservationRay,
        _ => ToolKind::HandSpade,
    };

    ToolEffector {
        kind,
        blade_width_m: v.get("blade_width_m").and_then(Value::as_f64).unwrap_or(0.18) as f32,
        rake_angle_rad: v.get("rake_angle_rad").and_then(Value::as_f64).unwrap_or(0.785) as f32,
        wedge_angle_rad: v.get("wedge_angle_rad").and_then(Value::as_f64).unwrap_or(0.349) as f32,
        applied_force_n: v.get("applied_force_n").and_then(Value::as_f64).unwrap_or(650.0) as f32,
        applied_torque_nm: v.get("applied_torque_nm").and_then(Value::as_f64).unwrap_or(95.0) as f32,
        capacity_volume_m3: v.get("capacity_volume_m3").and_then(Value::as_f64).unwrap_or(0.0035) as f32,
    }
}

/// `sim_evaluate_excavation_pass` tool — evaluates elasto-plastic cutting resistance, failure, and volumetric bulking.
pub fn evaluate_excavation_pass_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let tool = parse_tool_effector(&v);
    let soil = parse_soil_profile(&v);
    let depth_m = v.get("depth_m").and_then(Value::as_f64).unwrap_or(0.10) as f32;

    let verdict = evaluate_excavation_pass(&tool, depth_m, &soil);

    let payload = json!({
        "severed": verdict.severed,
        "requiredForceN": verdict.required_force_n,
        "appliedForceN": tool.applied_force_n,
        "marginForceN": verdict.margin_force_n,
        "excavatedBankVolumeM3": verdict.excavated_bank_volume_m3,
        "excavatedLooseVolumeM3": verdict.excavated_loose_volume_m3,
        "rootMatSevered": verdict.root_mat_severed,
        "refusalHit": verdict.refusal_hit,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `sim_evaluate_soil_compaction` tool — evaluates pore collapse and Kozeny-Carman permeability degradation under load.
pub fn evaluate_soil_compaction_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let pressure_kpa = v.get("contact_pressure_kpa").and_then(Value::as_f64).unwrap_or(150.0) as f32;
    let initial_porosity = v.get("initial_porosity").and_then(Value::as_f64).unwrap_or(0.45) as f32;
    let soil = parse_soil_profile(&v);

    let (final_porosity, k_multiplier) = evaluate_soil_compaction(pressure_kpa, initial_porosity, &soil);

    let payload = json!({
        "initialPorosity": initial_porosity,
        "finalPorosity": final_porosity,
        "porosityReduction": initial_porosity - final_porosity,
        "hydraulicConductivityMultiplier": k_multiplier,
        "compactionLevel": if k_multiplier < 0.3 { "severe" } else if k_multiplier < 0.7 { "moderate" } else { "mild" },
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `sim_critical_trench_depth` tool — calculates Rankine unconfined vertical trench wall collapse threshold.
pub fn critical_trench_depth_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let soil = parse_soil_profile(&v);
    let depth_m = critical_trench_depth_m(&soil);

    let payload = json!({
        "criticalTrenchDepthMeters": depth_m,
        "safeWorkingDepthMeters": depth_m * 0.65,
        "soilCohesionKpa": soil.cohesion_kpa,
        "soilFrictionAngleRad": soil.friction_angle_rad,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub use evaluate_excavation_pass_tool as sim_trench_excavation;
pub use evaluate_soil_compaction_tool as sim_soil_compaction;
pub use critical_trench_depth_tool as sim_critical_trench_depth;
