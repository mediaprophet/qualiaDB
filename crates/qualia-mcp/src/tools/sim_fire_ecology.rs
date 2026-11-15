//! MCP tools for Fire Ecology, Dead Fuel Moisture, and Byram Fireline Intensity.
//!
//! Powered by `qualia-sim::fire_ecology`.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_sim::fire_ecology::{
    evaluate_dead_fine_fuel_moisture, evaluate_fireline_intensity, FirelineRegime,
    ForestFuelStratum, FuelMoistureState,
};
use serde_json::{json, Value};

fn parse_forest_stratum(v: &Value) -> ForestFuelStratum {
    let years = v.get("years_since_fire").and_then(Value::as_f64).unwrap_or(8.0) as f32;
    ForestFuelStratum::mature_dry_sclerophyll(years)
}

/// `sim_evaluate_fuel_stratum` tool — calculates multi-stratum Olson fuel accumulation across post-fire succession.
pub fn evaluate_fuel_stratum_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let stratum = parse_forest_stratum(&v);

    let payload = json!({
        "surfaceLitterTonsHa": stratum.surface_litter_t_ha,
        "nearSurfaceGrassTonsHa": stratum.near_surface_grass_t_ha,
        "elevatedShrubTonsHa": stratum.elevated_shrub_t_ha,
        "barkHazardScore": stratum.bark_hazard_score,
        "canopyFoliageTonsHa": stratum.canopy_foliage_t_ha,
        "totalFineFuelKgM2": stratum.total_fine_fuel_kg_m2(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `sim_evaluate_dead_fuel_moisture` tool — models dead fine fuel moisture (DFMC) from vapor pressure deficit and solar drying.
pub fn evaluate_dead_fuel_moisture_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let temp = v.get("temp_c").and_then(Value::as_f64).unwrap_or(32.0) as f32;
    let rh = v.get("relative_humidity_pct").and_then(Value::as_f64).unwrap_or(22.0) as f32 / 100.0;
    let rad = v.get("solar_radiation_wm2").and_then(Value::as_f64).unwrap_or(850.0) as f32;
    let kbdi = v.get("kbdi_drought_index").and_then(Value::as_f64).unwrap_or(120.0) as f32;

    let dfmc = evaluate_dead_fine_fuel_moisture(temp, rh, rad, kbdi);

    let payload = json!({
        "deadFineFuelMoistureFraction": dfmc,
        "deadFineFuelMoisturePct": dfmc * 100.0,
        "isFlammable": dfmc < 0.16,
        "extinguishmentState": dfmc >= 0.16,
        "criticalCrownRisk": dfmc < 0.05,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `sim_evaluate_fire_behavior` tool — calculates rate of spread, Byram's fireline intensity, crown fire initiation, and spotting.
pub fn evaluate_fire_behavior_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let stratum = parse_forest_stratum(&v);
    let dfmc = v.get("dead_fuel_moisture_pct").and_then(Value::as_f64).unwrap_or(6.5) as f32 / 100.0;
    let kbdi = v.get("kbdi").and_then(Value::as_f64).unwrap_or(85.0) as f32;
    let curing = v.get("curing_pct").and_then(Value::as_f64).unwrap_or(65.0) as f32;
    let moisture = FuelMoistureState {
        dead_fuel_moisture_fraction: dfmc,
        kbdi_drought_index: kbdi,
        grass_curing_percentage: curing,
    };
    let wind_speed_km_h = v.get("wind_speed_10m_km_h").and_then(Value::as_f64).unwrap_or(35.0) as f32;
    let wind_m_s = wind_speed_km_h / 3.6;
    let slope = v.get("slope_degrees").and_then(Value::as_f64).unwrap_or(5.0) as f32;

    let behavior = evaluate_fireline_intensity(&stratum, &moisture, wind_m_s, slope);

    let payload = json!({
        "regime": format!("{:?}", behavior.regime),
        "rateOfSpreadMPerS": behavior.forward_spread_rate_m_s,
        "rateOfSpreadKmPerHour": behavior.forward_spread_rate_m_s * 3.6,
        "firelineIntensityKwPerM": behavior.fireline_intensity_kw_m,
        "fuelConsumedKgM2": behavior.fuel_consumed_kg_m2,
        "flameLengthMeters": behavior.flame_length_m,
        "barkSpottingDistanceKm": behavior.spotting_distance_km,
        "isCatastrophicCrowning": matches!(behavior.regime, FirelineRegime::CatastrophicCrowningPyroCb),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub use evaluate_fire_behavior_tool as sim_fire_ecology_eval;
pub use evaluate_fuel_stratum_tool as sim_fuel_stratum_eval;
pub use evaluate_dead_fuel_moisture_tool as sim_dead_fuel_moisture_eval;
