//! `Render` invoke handlers for AAA graphics capabilities.
//!
//! Provides dynamic scriptability from VibeScript for:
//! - Temporal Anti-Aliasing (TAA) mode & feedback
//! - Environment lighting probes
//! - Shoreline foam & water simulation parameters
//! - Dynamic vegetation collision deflection
//! - KTX2 compressed texture format support query

use std::sync::Mutex;
use super::super::args;
use vibe::{Diagnostic, Span, Value};

#[derive(Debug, Clone, Copy)]
pub struct TemporalState {
    pub enabled: bool,
    pub jitter_scale: f32,
    pub feedback_min: f32,
    pub feedback_max: f32,
    pub history_valid: bool,
}

impl Default for TemporalState {
    fn default() -> Self {
        Self {
            enabled: true,
            jitter_scale: 1.0,
            feedback_min: 0.88,
            feedback_max: 0.97,
            history_valid: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WaterState {
    pub elevation: f32,
    pub foam_falloff: f32,
    pub depth_blend: f32,
    pub wave_speed: f32,
}

impl Default for WaterState {
    fn default() -> Self {
        Self {
            elevation: 0.0,
            foam_falloff: 1.5,
            depth_blend: 2.0,
            wave_speed: 1.0,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct DynamicCollider {
    pub center: [f32; 3],
    pub radius: f32,
    pub height: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct EnvironmentProbeState {
    pub probe_id: u64,
    pub position: [f32; 3],
    pub radius: f32,
    pub irradiance: [f32; 3],
    pub specular_weight: f32,
    pub active: bool,
}

struct PipelineScriptState {
    temporal: TemporalState,
    water: WaterState,
    colliders: Vec<DynamicCollider>,
    probes: Vec<EnvironmentProbeState>,
}

static PIPELINE_STATE: Mutex<PipelineScriptState> = Mutex::new(PipelineScriptState {
    temporal: TemporalState {
        enabled: true,
        jitter_scale: 1.0,
        feedback_min: 0.88,
        feedback_max: 0.97,
        history_valid: false,
    },
    water: WaterState {
        elevation: 0.0,
        foam_falloff: 1.5,
        depth_blend: 2.0,
        wave_speed: 1.0,
    },
    colliders: Vec::new(),
    probes: Vec::new(),
});

fn lock_state() -> std::sync::MutexGuard<'static, PipelineScriptState> {
    PIPELINE_STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// `Render.temporal_set_mode` — configure TAA mode, jitter, and feedback clamping.
pub fn render_temporal_set_mode(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let enabled = args::rec_bool(args, "enabled").unwrap_or(true);
    let jitter_scale = args::rec_f64(args, "jitter_scale").unwrap_or(1.0) as f32;
    let feedback_min = args::rec_f64(args, "feedback_min").unwrap_or(0.88) as f32;
    let feedback_max = args::rec_f64(args, "feedback_max").unwrap_or(0.97) as f32;

    if feedback_min < 0.0 || feedback_min > 1.0 || feedback_max < 0.0 || feedback_max > 1.0 {
        return Err(args::bad(span, "feedback weights must be between 0.0 and 1.0"));
    }

    let mut state = lock_state();
    state.temporal.enabled = enabled;
    state.temporal.jitter_scale = jitter_scale;
    state.temporal.feedback_min = feedback_min;
    state.temporal.feedback_max = feedback_max;
    state.temporal.history_valid = false;

    Ok(args::record([
        ("ok", Value::Bool(true)),
        ("enabled", Value::Bool(enabled)),
        ("jitter_scale", Value::F64(jitter_scale as f64)),
        ("feedback_min", Value::F64(feedback_min as f64)),
        ("feedback_max", Value::F64(feedback_max as f64)),
    ]))
}

/// `Render.temporal_status` — query current TAA active status and capabilities.
pub fn render_temporal_status(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let state = lock_state();
    Ok(args::record([
        ("enabled", Value::Bool(state.temporal.enabled)),
        ("jitter_scale", Value::F64(state.temporal.jitter_scale as f64)),
        ("feedback_min", Value::F64(state.temporal.feedback_min as f64)),
        ("feedback_max", Value::F64(state.temporal.feedback_max as f64)),
        ("history_valid", Value::Bool(state.temporal.history_valid)),
        ("motion_vectors", Value::Bool(true)),
        ("reactive_mask", Value::Bool(true)),
    ]))
}

/// `Render.set_environment_probe` — configure an IBL environment lighting probe.
pub fn render_set_environment_probe(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let probe_id = args::rec_u64(args, "probe_id").unwrap_or(0);
    let radius = args::rec_f64(args, "radius").unwrap_or(10.0) as f32;
    let specular_weight = args::rec_f64(args, "specular_weight").unwrap_or(1.0) as f32;

    let pos = if let Some(p) = args::rec_f64_list(args, "position") {
        if p.len() >= 3 {
            [p[0] as f32, p[1] as f32, p[2] as f32]
        } else {
            return Err(args::bad(span, "position must have at least 3 coordinates"));
        }
    } else {
        let x = args::rec_f64(args, "x").unwrap_or(0.0) as f32;
        let y = args::rec_f64(args, "y").unwrap_or(0.0) as f32;
        let z = args::rec_f64(args, "z").unwrap_or(0.0) as f32;
        [x, y, z]
    };

    let irradiance = if let Some(irr) = args::rec_f64_list(args, "irradiance") {
        if irr.len() >= 3 {
            [irr[0] as f32, irr[1] as f32, irr[2] as f32]
        } else {
            [1.0, 1.0, 1.0]
        }
    } else {
        let r = args::rec_f64(args, "r").unwrap_or(1.0) as f32;
        let g = args::rec_f64(args, "g").unwrap_or(1.0) as f32;
        let b = args::rec_f64(args, "b").unwrap_or(1.0) as f32;
        [r, g, b]
    };

    let mut state = lock_state();
    let probe = EnvironmentProbeState {
        probe_id,
        position: pos,
        radius,
        irradiance,
        specular_weight,
        active: true,
    };

    if let Some(existing) = state.probes.iter_mut().find(|p| p.probe_id == probe_id) {
        *existing = probe;
    } else {
        state.probes.push(probe);
    }

    Ok(args::record([
        ("ok", Value::Bool(true)),
        ("probe_id", Value::U64(probe_id)),
        ("radius", Value::F64(radius as f64)),
        ("specular_weight", Value::F64(specular_weight as f64)),
        ("active", Value::Bool(true)),
        ("total_probes", Value::U64(state.probes.len() as u64)),
    ]))
}

/// `Render.water_set_parameters` — adjust water elevation, shoreline foam falloff, and wave speed.
pub fn render_water_set_parameters(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let elevation = args::rec_f64(args, "elevation").unwrap_or(0.0) as f32;
    let foam_falloff = args::rec_f64(args, "foam_falloff").unwrap_or(1.5) as f32;
    let depth_blend = args::rec_f64(args, "depth_blend").unwrap_or(2.0) as f32;
    let wave_speed = args::rec_f64(args, "wave_speed").unwrap_or(1.0) as f32;

    let mut state = lock_state();
    state.water.elevation = elevation;
    state.water.foam_falloff = foam_falloff;
    state.water.depth_blend = depth_blend;
    state.water.wave_speed = wave_speed;

    Ok(args::record([
        ("ok", Value::Bool(true)),
        ("elevation", Value::F64(elevation as f64)),
        ("foam_falloff", Value::F64(foam_falloff as f64)),
        ("depth_blend", Value::F64(depth_blend as f64)),
        ("wave_speed", Value::F64(wave_speed as f64)),
    ]))
}

/// `Render.vegetation_push_collider` — register a dynamic cylinder collider for blade deflection.
pub fn render_vegetation_push_collider(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let radius = args::rec_f64(args, "radius").unwrap_or(0.5) as f32;
    let height = args::rec_f64(args, "height").unwrap_or(1.8) as f32;

    let center = if let Some(c) = args::rec_f64_list(args, "center") {
        if c.len() >= 3 {
            [c[0] as f32, c[1] as f32, c[2] as f32]
        } else {
            return Err(args::bad(span, "center must have at least 3 coordinates"));
        }
    } else {
        let x = args::rec_f64(args, "x").unwrap_or(0.0) as f32;
        let y = args::rec_f64(args, "y").unwrap_or(0.0) as f32;
        let z = args::rec_f64(args, "z").unwrap_or(0.0) as f32;
        [x, y, z]
    };

    let mut state = lock_state();
    if state.colliders.len() >= 16 {
        state.colliders.remove(0); // Bounded ring of dynamic colliders
    }
    state.colliders.push(DynamicCollider { center, radius, height });

    Ok(args::record([
        ("ok", Value::Bool(true)),
        ("collider_count", Value::U64(state.colliders.len() as u64)),
    ]))
}

/// `Render.vegetation_clear_colliders` — clear all dynamic blade deflection colliders.
pub fn render_vegetation_clear_colliders(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut state = lock_state();
    state.colliders.clear();
    Ok(args::record([
        ("ok", Value::Bool(true)),
        ("collider_count", Value::U64(0)),
    ]))
}

/// `Render.texture_transcode_caps` — query supported KTX2 texture decompression formats.
pub fn render_texture_transcode_caps(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    Ok(args::record([
        ("bc1", Value::Bool(true)),
        ("bc3", Value::Bool(true)),
        ("bc7", Value::Bool(true)),
        ("etc2", Value::Bool(true)),
        ("astc", Value::Bool(true)),
        ("zstandard", Value::Bool(true)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_set_mode_and_status() {
        let span = Span::point(0);
        let set_args = args::record([
            ("enabled", Value::Bool(true)),
            ("jitter_scale", Value::F64(0.8)),
            ("feedback_min", Value::F64(0.90)),
            ("feedback_max", Value::F64(0.96)),
        ]);
        let res = render_temporal_set_mode(&set_args, span).expect("set mode should succeed");
        assert_eq!(args::rec_bool(&res, "ok"), Some(true));
        assert_eq!(args::rec_bool(&res, "enabled"), Some(true));

        let status = render_temporal_status(&Value::Null, span).expect("status query");
        assert_eq!(args::rec_bool(&status, "motion_vectors"), Some(true));
        assert_eq!(args::rec_bool(&status, "reactive_mask"), Some(true));
    }

    #[test]
    fn test_environment_probe_and_water_and_vegetation() {
        let span = Span::point(0);
        let probe_args = args::record([
            ("probe_id", Value::U64(42)),
            ("radius", Value::F64(15.0)),
            ("x", Value::F64(1.0)),
            ("y", Value::F64(2.0)),
            ("z", Value::F64(3.0)),
            ("r", Value::F64(0.8)),
            ("g", Value::F64(0.9)),
            ("b", Value::F64(1.0)),
            ("specular_weight", Value::F64(0.75)),
        ]);
        let res = render_set_environment_probe(&probe_args, span).expect("probe set");
        assert_eq!(args::rec_bool(&res, "ok"), Some(true));
        assert_eq!(args::rec_u64(&res, "probe_id"), Some(42));

        let water_args = args::record([
            ("elevation", Value::F64(0.5)),
            ("foam_falloff", Value::F64(2.0)),
            ("depth_blend", Value::F64(3.0)),
            ("wave_speed", Value::F64(1.2)),
        ]);
        let w_res = render_water_set_parameters(&water_args, span).expect("water set");
        assert_eq!(args::rec_f64(&w_res, "elevation"), Some(0.5));

        let veg_args = args::record([
            ("x", Value::F64(2.0)),
            ("y", Value::F64(0.0)),
            ("z", Value::F64(4.0)),
            ("radius", Value::F64(0.6)),
            ("height", Value::F64(2.0)),
        ]);
        let v_res = render_vegetation_push_collider(&veg_args, span).expect("veg collider push");
        assert_eq!(args::rec_bool(&v_res, "ok"), Some(true));

        let caps = render_texture_transcode_caps(&Value::Null, span).expect("transcode caps");
        assert_eq!(args::rec_bool(&caps, "bc7"), Some(true));
        assert_eq!(args::rec_bool(&caps, "astc"), Some(true));
    }

    #[test]
    fn test_vibe_graphics_backend_policy_evaluation() {
        use vibe::{eval_function, load_program, Value};
        let src = include_str!("../../../render/portal/graphics_backend.vibe");
        let program = load_program(src).expect("graphics_backend.vibe should parse");
        let mut host = vibe::LocalHost::default();
        let mut env = vibe::Env::default();

        let res = eval_function(
            &program,
            "admit_feature",
            vec![Value::String("ultra".into()), Value::String("temporal_aa".into())],
            &mut host,
            &mut env,
        ).expect("eval admit_feature");
        assert_eq!(res, Value::Bool(true));

        let res = eval_function(
            &program,
            "admit_feature",
            vec![Value::String("balanced".into()), Value::String("temporal_aa".into())],
            &mut host,
            &mut env,
        ).expect("eval admit_feature");
        assert_eq!(res, Value::Bool(true));

        let res = eval_function(
            &program,
            "select_vegetation_density",
            vec![Value::String("ultra".into())],
            &mut host,
            &mut env,
        ).expect("eval select_vegetation_density");
        assert_eq!(res, Value::I64(128));

        let res = eval_function(
            &program,
            "select_water_quality",
            vec![Value::String("balanced".into())],
            &mut host,
            &mut env,
        ).expect("eval select_water_quality");
        assert_eq!(res, Value::String("foam_depth".into()));
    }
}
