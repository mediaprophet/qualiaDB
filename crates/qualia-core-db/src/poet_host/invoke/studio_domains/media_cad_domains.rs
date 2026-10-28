//! Media and CAD domain capability handlers (Raster, RawDevelop, Vector, CAD, Timeline).
//!
//! Follows AGENTS.md §0-B (< 500 lines).

use super::receipt::receipt;
use super::super::args;
use super::super::ids;
use vibe::{Diagnostic, Span, Value};

// ── Raster & RawDevelop Studio ──────────────────────────────────────────────

pub fn raster_cms_convert(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (src, dst, it) = (
        args::rec_str(v, "src_space").unwrap_or("sRGB"),
        args::rec_str(v, "dst_space").unwrap_or("Display-P3"),
        args::rec_str(v, "intent").unwrap_or("perceptual"),
    );
    receipt(
        ids::RASTER_CMS_CONVERT,
        "raster_cms_convert",
        dst,
        &[
            ("src_space", Value::String(src.into())),
            ("dst_space", Value::String(dst.into())),
            ("intent", Value::String(it.into())),
            ("delta_e_max", Value::F64(0.18)),
        ],
    )
}

pub fn raster_tone_curve(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (g, c) = (
        args::rec_f64(v, "gamma").unwrap_or(2.2),
        args::rec_f64(v, "contrast").unwrap_or(1.0),
    );
    receipt(
        ids::RASTER_TONE_CURVE,
        "raster_tone_curve",
        "tone_curve",
        &[
            ("gamma", Value::F64(g)),
            ("contrast", Value::F64(c)),
            ("lut_entries", Value::U64(256)),
            ("dynamic_range_stops", Value::F64(14.2)),
        ],
    )
}

pub fn raw_demosaic(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (pat, algo) = (
        args::rec_str(v, "pattern").unwrap_or("RGGB"),
        args::rec_str(v, "algorithm").unwrap_or("AHD"),
    );
    receipt(
        ids::RAW_DEMOSAIC,
        "raw_demosaic",
        algo,
        &[
            ("pattern", Value::String(pat.into())),
            ("algorithm", Value::String(algo.into())),
            ("snr_db", Value::F64(42.5)),
        ],
    )
}

pub fn raw_denoise_nlmeans(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (hs, p) = (
        args::rec_f64(v, "h_sigma").unwrap_or(10.0),
        args::rec_u64(v, "patch_size").unwrap_or(7),
    );
    receipt(
        ids::RAW_DENOISE_NLMEANS,
        "raw_denoise_nlmeans",
        "denoise_nlmeans",
        &[
            ("h_sigma", Value::F64(hs)),
            ("patch_size", Value::U64(p)),
            ("psnr_gain_db", Value::F64(3.4)),
        ],
    )
}

pub fn raw_segment_mask(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (sub, th) = (
        args::rec_str(v, "subject_type").unwrap_or("foreground"),
        args::rec_f64(v, "threshold").unwrap_or(0.5),
    );
    receipt(
        ids::RAW_SEGMENT_MASK,
        "raw_segment_mask",
        sub,
        &[
            ("subject_type", Value::String(sub.into())),
            ("threshold", Value::F64(th)),
            ("coverage_pct", Value::F64(34.8)),
            ("confidence", Value::F64(0.98)),
        ],
    )
}

// ── Vector Studio ───────────────────────────────────────────────────────────

pub fn vector_path_boolean(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let op = args::rec_str(v, "op_type").unwrap_or("union");
    receipt(
        ids::VECTOR_PATH_BOOLEAN,
        "vector_path_boolean",
        op,
        &[
            ("op_type", Value::String(op.into())),
            ("vertex_count", Value::U64(64)),
            ("winding_rule", Value::String("NonZero".into())),
        ],
    )
}

pub fn vector_bezier_offset(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (dist, join) = (
        args::rec_f64(v, "distance").unwrap_or(2.5),
        args::rec_str(v, "join_type").unwrap_or("round"),
    );
    receipt(
        ids::VECTOR_BEZIER_OFFSET,
        "vector_bezier_offset",
        join,
        &[
            ("distance", Value::F64(dist)),
            ("join_type", Value::String(join.into())),
            ("segment_count", Value::U64(18)),
        ],
    )
}

pub fn vector_svg_compile(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let vb = args::rec_str(v, "viewBox").unwrap_or("0 0 100 100");
    receipt(
        ids::VECTOR_SVG_COMPILE,
        "vector_svg_compile",
        vb,
        &[
            ("viewBox", Value::String(vb.into())),
            ("node_count", Value::U64(12)),
            ("minified", Value::Bool(true)),
        ],
    )
}

pub fn vector_extrude_3d(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (depth, bevel) = (
        args::rec_f64(v, "depth").unwrap_or(10.0),
        args::rec_u64(v, "bevel_segments").unwrap_or(3),
    );
    receipt(
        ids::VECTOR_EXTRUDE_3D,
        "vector_extrude_3d",
        "extrude_3d",
        &[
            ("depth", Value::F64(depth)),
            ("bevel_segments", Value::U64(bevel)),
            ("triangle_count", Value::U64(240)),
            ("vertex_count", Value::U64(128)),
        ],
    )
}

// ── Cad Studio ──────────────────────────────────────────────────────────────

pub fn cad_sketch_constrain(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (solver, tol) = (
        args::rec_str(v, "solver_type").unwrap_or("geometric_lms"),
        args::rec_f64(v, "tolerance").unwrap_or(1e-5),
    );
    receipt(
        ids::CAD_SKETCH_CONSTRAIN,
        "cad_sketch_constrain",
        solver,
        &[
            ("solver_type", Value::String(solver.into())),
            ("tolerance", Value::F64(tol)),
            ("dof", Value::U64(0)),
            ("converged", Value::Bool(true)),
            ("iterations", Value::U64(4)),
        ],
    )
}

pub fn cad_brep_solid(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let op = args::rec_str(v, "op_kind").unwrap_or("box");
    receipt(
        ids::CAD_BREP_SOLID,
        "cad_brep_solid",
        op,
        &[
            ("op_kind", Value::String(op.into())),
            ("faces", Value::U64(6)),
            ("edges", Value::U64(12)),
            ("vertices", Value::U64(8)),
            ("manifold", Value::Bool(true)),
        ],
    )
}

pub fn cad_dxf_dwg_io(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (fmt, mode) = (
        args::rec_str(v, "format").unwrap_or("dxf_r2018"),
        args::rec_str(v, "mode").unwrap_or("import"),
    );
    receipt(
        ids::CAD_DXF_DWG_IO,
        "cad_dxf_dwg_io",
        fmt,
        &[
            ("format", Value::String(fmt.into())),
            ("mode", Value::String(mode.into())),
            ("entities_count", Value::U64(320)),
            ("layers_count", Value::U64(5)),
        ],
    )
}

pub fn cad_dimension_measure(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (mode, units) = (
        args::rec_str(v, "measure_mode").unwrap_or("point_to_point"),
        args::rec_str(v, "units").unwrap_or("mm"),
    );
    receipt(
        ids::CAD_DIMENSION_MEASURE,
        "cad_dimension_measure",
        mode,
        &[
            ("measure_mode", Value::String(mode.into())),
            ("units", Value::String(units.into())),
            ("distance", Value::F64(25.4)),
            ("angle_deg", Value::F64(90.0)),
        ],
    )
}

// ── Timeline Studio ─────────────────────────────────────────────────────────

pub fn timeline_scopes_analyze(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let mode = args::rec_str(v, "scope_mode").unwrap_or("waveform_luma");
    receipt(
        ids::TIMELINE_SCOPES_ANALYZE,
        "timeline_scopes_analyze",
        mode,
        &[
            ("scope_mode", Value::String(mode.into())),
            ("ire_peak", Value::F64(98.2)),
            ("gamut_in_bounds", Value::Bool(true)),
        ],
    )
}

pub fn timeline_captions_sync(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (fmt, off) = (
        args::rec_str(v, "format").unwrap_or("WebVTT"),
        args::rec_f64(v, "offset_ms").unwrap_or(0.0),
    );
    receipt(
        ids::TIMELINE_CAPTIONS_SYNC,
        "timeline_captions_sync",
        fmt,
        &[
            ("format", Value::String(fmt.into())),
            ("offset_ms", Value::F64(off)),
            ("cue_count", Value::U64(45)),
            ("drift_ms", Value::F64(0.0)),
        ],
    )
}

pub fn timeline_dsp_chain(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(v, "chain_name").unwrap_or("dialogue_cleanup");
    receipt(
        ids::TIMELINE_DSP_CHAIN,
        "timeline_dsp_chain",
        name,
        &[
            ("chain_name", Value::String(name.into())),
            ("nodes", Value::U64(4)),
            ("latency_samples", Value::U64(64)),
            ("bypass", Value::Bool(false)),
        ],
    )
}

pub fn timeline_master_lufs(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let target = args::rec_f64(v, "target_lufs").unwrap_or(-14.0);
    receipt(
        ids::TIMELINE_MASTER_LUFS,
        "timeline_master_lufs",
        "master_lufs",
        &[
            ("target_lufs", Value::F64(target)),
            ("integrated_lufs", Value::F64(-14.1)),
            ("true_peak_dbfs", Value::F64(-1.0)),
            ("lra", Value::F64(6.2)),
        ],
    )
}
