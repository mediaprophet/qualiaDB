//! MCP tools for Space Weather, Heliophysical Oscillations, and Geomagnetic Coupling.
//!
//! Powered by `qualia-sim::space_weather`.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_sim::space_weather::{
    evaluate_solar_cycle_phase, evaluate_space_weather_coupling, SolarWindState,
};
use serde_json::{json, Value};

/// `sim_evaluate_space_weather` tool — models 11-year Schwabe and 22-year Hale magnetic cycles, EUV, TSI, and GCR modulation.
pub fn evaluate_space_weather_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let date = v.get("decimal_year").and_then(Value::as_f64).unwrap_or(2026.5) as f32;

    let (schwabe, hale) = evaluate_solar_cycle_phase(date);

    let payload = json!({
        "decimalYear": date,
        "solarCycle": schwabe.cycle_number,
        "phaseRad": schwabe.phase_rad,
        "sunspotNumber": schwabe.sunspot_number,
        "f107RadioFluxSfu": schwabe.f10_7_radio_flux_sfu,
        "tsiAnomalyWm2": schwabe.tsi_anomaly_w_m2,
        "thermosphereClimateIndex10e10W": schwabe.thermosphere_climate_index_10e10_w,
        "spotlessDaysRatio": schwabe.spotless_days_ratio,
        "isSolarNorthPositive": hale.is_solar_north_positive,
        "cosmicRayModulationFactor": hale.cosmic_ray_modulation_factor,
        "decadalDroughtPropensity": hale.decadal_drought_propensity,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `sim_evaluate_solar_wind_coupling` tool — evaluates solar wind dynamic pressure, IMF Bz reconnection, and Kp/Dst storm indices.
pub fn evaluate_solar_wind_coupling_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let date = v.get("decimal_year").and_then(Value::as_f64).unwrap_or(2026.5) as f32;
    let speed = v.get("speed_km_s").and_then(Value::as_f64).unwrap_or(450.0) as f32;
    let density = v.get("proton_density_cm3").and_then(Value::as_f64).unwrap_or(5.0) as f32;
    let bz = v.get("imf_bz_nt").and_then(Value::as_f64).unwrap_or(-2.5) as f32;
    let b_total = v.get("imf_total_nt").and_then(Value::as_f64).unwrap_or(5.0) as f32;

    let (solar, hale) = evaluate_solar_cycle_phase(date);
    let p_dyn = 1.6726e-27 * (density * 1e6) * (speed * 1e3) * (speed * 1e3) * 1e9;
    let wind = SolarWindState {
        velocity_km_s: speed,
        proton_density_cm3: density,
        imf_total_nt: b_total,
        imf_bz_nt: bz,
        dynamic_pressure_npa: p_dyn,
    };

    let (storm, gec) = evaluate_space_weather_coupling(&solar, &hale, &wind);

    let payload = json!({
        "solarWindVelocityKmS": wind.velocity_km_s,
        "protonDensityCm3": wind.proton_density_cm3,
        "imfBzNt": wind.imf_bz_nt,
        "imfTotalNt": wind.imf_total_nt,
        "dynamicPressureNPa": wind.dynamic_pressure_npa,
        "isReconnectionActive": storm.is_reconnection_active,
        "planetaryKpIndex": storm.kp_index,
        "dstIndexNt": storm.dst_index_nt,
        "auroralPowerGw": storm.auroral_power_gw,
        "ionosphericPotentialKv": gec.ionospheric_potential_kv,
        "fairWeatherFieldVm": gec.fair_weather_field_v_m,
        "dryLightningSusceptibility": gec.dry_lightning_susceptibility,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub use evaluate_space_weather_tool as sim_space_weather_eval;
pub use evaluate_solar_wind_coupling_tool as sim_solar_wind_coupling;
