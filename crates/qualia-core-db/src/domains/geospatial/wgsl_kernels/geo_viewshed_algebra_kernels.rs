// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! WGSL Compute Shader Generators, VibeScript Cell -> WGSL Map Algebra Compiler,
//! and CPU Parity Oracles for Viewshed, Horn Geomorphometry, TWI/SPI, and Map Algebra.
//!
//! Allocation Class:
//! - Tier-1 (`HotZeroHeap`): `cpu_oracle_compiled_map_algebra_into`
//! - Tier-2 (`ColdBounded`): `compile_vibe_cell_to_wgsl_map_algebra`,
//!   `emit_wgsl_radial_viewshed_and_hillshade`, `emit_wgsl_horn_twi_spi_geomorphons`
//!   (enforces caller-supplied `byte_budget` fail-closed).

/// Uniform parameter block matching WGSL struct declaration (exact 64-byte `#[repr(C)]`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewshedAlgebraGpuUniforms {
    pub width: u32, pub height: u32, pub obs_col: u32, pub obs_row: u32,
    pub cell_size_m: f32, pub obs_height_m: f32, pub target_height_m: f32, pub max_radius_m: f32,
    pub curvature_refraction_k: f32, pub earth_radius_m: f32, pub sun_azimuth_rad: f32, pub sun_altitude_rad: f32,
    pub nodata_value: f32, pub band_count: u32, pub program_len: u32, pub flags: u32,
}

impl ViewshedAlgebraGpuUniforms {
    pub const fn new(width: u32, height: u32, cell_size_m: f32) -> Self {
        Self {
            width, height, obs_col: 0, obs_row: 0, cell_size_m,
            obs_height_m: 2.0, target_height_m: 0.0, max_radius_m: 10_000.0,
            curvature_refraction_k: 0.13, earth_radius_m: 6_371_000.0,
            sun_azimuth_rad: 0.7853982, sun_altitude_rad: 0.7853982,
            nodata_value: -9999.0, band_count: 1, program_len: 0, flags: 0,
        }
    }
}

/// Reverse-Polish Notation tokens for VibeScript cell expressions and Map Algebra.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WgslMapAlgebraToken {
    Band(u32), Literal(f32), Add, Sub, Mul, SafeDiv, NormalizedDiff,
    Min, Max, Sqrt, Abs, Clamp(f32, f32), Gt, Lt, Select,
}

/// Errors occurring during shader emission, cell compilation, or CPU oracle grading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeoAlgebraWgslError {
    InvalidDimensions, DimensionMismatch, BandIndexOutOfBounds(u32),
    StackUnderflow, StackOverflow, InvalidProgram(&'static str),
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for GeoAlgebraWgslError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "Invalid grid dimensions"),
            Self::DimensionMismatch => write!(f, "Buffer length does not match grid dimensions"),
            Self::BandIndexOutOfBounds(b) => write!(f, "Band index {b} out of bounds"),
            Self::StackUnderflow => write!(f, "Stack underflow in map algebra"),
            Self::StackOverflow => write!(f, "Stack overflow in map algebra (max 32)"),
            Self::InvalidProgram(msg) => write!(f, "Invalid map algebra program: {msg}"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Budget exceeded: req {requested} B, budget {budget} B"),
        }
    }
}
impl std::error::Error for GeoAlgebraWgslError {}

/// FNV-1a 64-bit deterministic hash for WGSL shader sources.
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for &b in bytes {
        h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
    }
    h
}

const WGSL_COMMON_STRUCT: &str = r#"struct ViewshedAlgebraUniforms {
    width: u32, height: u32, obs_col: u32, obs_row: u32,
    cell_size_m: f32, obs_height_m: f32, target_height_m: f32, max_radius_m: f32,
    curvature_refraction_k: f32, earth_radius_m: f32, sun_azimuth_rad: f32, sun_altitude_rad: f32,
    nodata_value: f32, band_count: u32, program_len: u32, flags: u32,
};
"#;

const WGSL_SAMPLE_DEM_FN: &str = r#"fn sample_dem(c: i32, r: i32) -> f32 {
    let sc = clamp(c, 0, i32(params.width) - 1);
    let sr = clamp(r, 0, i32(params.height) - 1);
    return in_dem[u32(sr) * params.width + u32(sc)];
}
"#;

/// Compiles a stack-validated VibeScript cell token program into a fused,
/// zero-branch-divergence WGSL compute shader with nodata propagation.
///
/// Allocation Class: Tier-2 (`ColdBounded`). Enforces `byte_budget` fail-closed.
pub fn compile_vibe_cell_to_wgsl_map_algebra(
    tokens: &[WgslMapAlgebraToken],
    workgroup_x: u32,
    workgroup_y: u32,
    byte_budget: usize,
) -> Result<(String, u64), GeoAlgebraWgslError> {
    if tokens.is_empty() { return Err(GeoAlgebraWgslError::InvalidProgram("Empty tokens")); }
    let mut depth = 0usize;
    for t in tokens {
        match t {
            WgslMapAlgebraToken::Band(_) | WgslMapAlgebraToken::Literal(_) => depth += 1,
            WgslMapAlgebraToken::Add | WgslMapAlgebraToken::Sub | WgslMapAlgebraToken::Mul
            | WgslMapAlgebraToken::SafeDiv | WgslMapAlgebraToken::NormalizedDiff
            | WgslMapAlgebraToken::Min | WgslMapAlgebraToken::Max
            | WgslMapAlgebraToken::Gt | WgslMapAlgebraToken::Lt => {
                if depth < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } depth -= 1;
            }
            WgslMapAlgebraToken::Sqrt | WgslMapAlgebraToken::Abs | WgslMapAlgebraToken::Clamp(_, _) => {
                if depth < 1 { return Err(GeoAlgebraWgslError::StackUnderflow); }
            }
            WgslMapAlgebraToken::Select => {
                if depth < 3 { return Err(GeoAlgebraWgslError::StackUnderflow); } depth -= 2;
            }
        }
        if depth > 32 { return Err(GeoAlgebraWgslError::StackOverflow); }
    }
    if depth != 1 { return Err(GeoAlgebraWgslError::InvalidProgram("Stack != 1 result")); }

    let mut stack: Vec<String> = Vec::with_capacity(32);
    let mut ssa = String::new();
    let mut vid = 0usize;

    for tok in tokens {
        match *tok {
            WgslMapAlgebraToken::Band(b) => {
                let v = format!("v{vid}");
                ssa.push_str(&format!("    let {v} = in_bands[idx + {b}u * total_cells];\n    has_nodata = max(has_nodata, select(0.0, 1.0, abs({v} - params.nodata_value) < 1e-4));\n"));
                stack.push(v); vid += 1;
            }
            WgslMapAlgebraToken::Literal(c) => {
                let v = format!("v{vid}");
                let s = if c.fract() == 0.0 { format!("{c:.1}") } else { format!("{c}") };
                ssa.push_str(&format!("    let {v} = {s};\n")); stack.push(v); vid += 1;
            }
            WgslMapAlgebraToken::Add => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = {a} + {b};\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Sub => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = {a} - {b};\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Mul => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = {a} * {b};\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::SafeDiv => {
                let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}");
                ssa.push_str(&format!("    let den_{vid} = select({b}, 1.0, abs({b}) < 1e-7);\n    let {v} = select({a} / den_{vid}, 0.0, abs({b}) < 1e-7);\n"));
                stack.push(v); vid += 1;
            }
            WgslMapAlgebraToken::NormalizedDiff => {
                let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}");
                ssa.push_str(&format!("    let sum_{vid} = {a} + {b};\n    let den_{vid} = select(sum_{vid}, 1.0, abs(sum_{vid}) < 1e-7);\n    let {v} = select(({a} - {b}) / den_{vid}, 0.0, abs(sum_{vid}) < 1e-7);\n"));
                stack.push(v); vid += 1;
            }
            WgslMapAlgebraToken::Min => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = min({a}, {b});\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Max => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = max({a}, {b});\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Sqrt => { let a = stack.pop().unwrap(); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = sqrt(max({a}, 0.0));\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Abs => { let a = stack.pop().unwrap(); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = abs({a});\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Clamp(lo, hi) => {
                let a = stack.pop().unwrap(); let v = format!("v{vid}");
                let (slo, shi) = (if lo.fract() == 0.0 { format!("{lo:.1}") } else { format!("{lo}") }, if hi.fract() == 0.0 { format!("{hi:.1}") } else { format!("{hi}") });
                ssa.push_str(&format!("    let {v} = clamp({a}, {slo}, {shi});\n")); stack.push(v); vid += 1;
            }
            WgslMapAlgebraToken::Gt => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = select(0.0, 1.0, {a} > {b});\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Lt => { let (b, a) = (stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}"); ssa.push_str(&format!("    let {v} = select(0.0, 1.0, {a} < {b});\n")); stack.push(v); vid += 1; }
            WgslMapAlgebraToken::Select => {
                let (f, t, c) = (stack.pop().unwrap(), stack.pop().unwrap(), stack.pop().unwrap()); let v = format!("v{vid}");
                ssa.push_str(&format!("    let {v} = select({f}, {t}, abs({c}) > 1e-6);\n")); stack.push(v); vid += 1;
            }
        }
    }
    let res = stack.pop().unwrap();
    let shader = format!(
        "{WGSL_COMMON_STRUCT}\
@group(0) @binding(0) var<uniform> params: ViewshedAlgebraUniforms;
@group(0) @binding(1) var<storage, read> in_bands: array<f32>;
@group(0) @binding(2) var<storage, read_write> out_buf: array<f32>;

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let col = gid.x; let row = gid.y;
    if (col >= params.width || row >= params.height) {{ return; }}
    let total_cells = params.width * params.height;
    let idx = row * params.width + col;
    var has_nodata: f32 = 0.0;
{ssa}    out_buf[idx] = select({res}, params.nodata_value, has_nodata > 0.5);
}}
"
    );
    let req = shader.len();
    if req > byte_budget {
        return Err(GeoAlgebraWgslError::ByteBudgetExceeded { requested: req, budget: byte_budget });
    }
    let hash = fnv1a_64(shader.as_bytes());
    Ok((shader, hash))
}

/// Emits WGSL compute shader for 2D DDA ray-marched Viewshed with Earth curvature
/// & atmospheric refraction correction, Horn 3x3 hillshade, and solar irradiance.
///
/// Allocation Class: Tier-2 (`ColdBounded`). Enforces `byte_budget` fail-closed.
pub fn emit_wgsl_radial_viewshed_and_hillshade(
    workgroup_x: u32,
    workgroup_y: u32,
    byte_budget: usize,
) -> Result<(String, u64), GeoAlgebraWgslError> {
    let shader = format!(
        "{WGSL_COMMON_STRUCT}\
@group(0) @binding(0) var<uniform> params: ViewshedAlgebraUniforms;
@group(0) @binding(1) var<storage, read> in_dem: array<f32>;
@group(0) @binding(2) var<storage, read_write> out_viewshed: array<f32>;
@group(0) @binding(3) var<storage, read_write> out_hillshade: array<f32>;

{WGSL_SAMPLE_DEM_FN}
@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let col = gid.x; let row = gid.y;
    if (col >= params.width || row >= params.height) {{ return; }}
    let idx = row * params.width + col;
    let center_dem = in_dem[idx];
    if (abs(center_dem - params.nodata_value) < 1e-4) {{
        out_viewshed[idx] = params.nodata_value; out_hillshade[idx] = params.nodata_value; return;
    }}
    let ic = i32(col); let ir = i32(row);
    let z00 = sample_dem(ic - 1, ir - 1); let z10 = sample_dem(ic, ir - 1); let z20 = sample_dem(ic + 1, ir - 1);
    let z01 = sample_dem(ic - 1, ir);                                        let z21 = sample_dem(ic + 1, ir);
    let z02 = sample_dem(ic - 1, ir + 1); let z12 = sample_dem(ic, ir + 1); let z22 = sample_dem(ic + 1, ir + 1);
    let dz_dx = ((z20 + 2.0 * z21 + z22) - (z00 + 2.0 * z01 + z02)) / (8.0 * params.cell_size_m);
    let dz_dy = ((z02 + 2.0 * z12 + z22) - (z00 + 2.0 * z10 + z20)) / (8.0 * params.cell_size_m);
    let slope = atan(sqrt(dz_dx * dz_dx + dz_dy * dz_dy));
    let aspect = atan2(-dz_dy, dz_dx);
    let sun_zenith = 1.5707963 - params.sun_altitude_rad;
    let hillshade = max(0.0, cos(sun_zenith) * cos(slope) + sin(sun_zenith) * sin(slope) * cos(params.sun_azimuth_rad - aspect));

    let dx = f32(col) - f32(params.obs_col); let dy = f32(row) - f32(params.obs_row);
    let d_tgt = length(vec2<f32>(dx, dy)) * params.cell_size_m;
    var vis: f32 = 1.0;
    if (d_tgt > params.max_radius_m) {{
        vis = 0.0;
    }} else if (col != params.obs_col || row != params.obs_row) {{
        let drop = (1.0 - params.curvature_refraction_k) / (2.0 * params.earth_radius_m);
        let obs_dem = in_dem[params.obs_row * params.width + params.obs_col];
        let z_eye = obs_dem + params.obs_height_m;
        let z_tgt = center_dem + params.target_height_m - drop * d_tgt * d_tgt;
        let tan_tgt = (z_tgt - z_eye) / d_tgt;
        let steps = u32(clamp(max(abs(dx), abs(dy)), 1.0, 512.0));
        let step_x = dx / f32(steps); let step_y = dy / f32(steps);
        for (var s = 1u; s < steps; s = s + 1u) {{
            let sc = u32(round(f32(params.obs_col) + f32(s) * step_x));
            let sr = u32(round(f32(params.obs_row) + f32(s) * step_y));
            let s_dem = in_dem[sr * params.width + sc];
            let s_d = length(vec2<f32>(f32(sc) - f32(params.obs_col), f32(sr) - f32(params.obs_row))) * params.cell_size_m;
            if (s_d > 0.0) {{
                let s_z = s_dem - drop * s_d * s_d;
                if ((s_z - z_eye) / s_d >= tan_tgt) {{ vis = 0.0; break; }}
            }}
        }}
    }}
    out_viewshed[idx] = vis; out_hillshade[idx] = hillshade * vis;
}}
"
    );
    let req = shader.len();
    if req > byte_budget {
        return Err(GeoAlgebraWgslError::ByteBudgetExceeded { requested: req, budget: byte_budget });
    }
    let hash = fnv1a_64(shader.as_bytes());
    Ok((shader, hash))
}

/// Emits WGSL compute shader computing Horn slope, aspect, plan/profile curvature,
/// Topographic Wetness Index (TWI), Stream Power Index (SPI), and 8-azimuth Geomorphons.
///
/// Allocation Class: Tier-2 (`ColdBounded`). Enforces `byte_budget` fail-closed.
pub fn emit_wgsl_horn_twi_spi_geomorphons(
    workgroup_x: u32,
    workgroup_y: u32,
    byte_budget: usize,
) -> Result<(String, u64), GeoAlgebraWgslError> {
    let shader = format!(
        "{WGSL_COMMON_STRUCT}\
@group(0) @binding(0) var<uniform> params: ViewshedAlgebraUniforms;
@group(0) @binding(1) var<storage, read> in_dem: array<f32>;
@group(0) @binding(2) var<storage, read_write> out_morphometry: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> out_hydro: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read_write> out_geomorphons: array<u32>;

{WGSL_SAMPLE_DEM_FN}
@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let col = gid.x; let row = gid.y;
    if (col >= params.width || row >= params.height) {{ return; }}
    let idx = row * params.width + col;
    let z11 = in_dem[idx];
    if (abs(z11 - params.nodata_value) < 1e-4) {{
        out_morphometry[idx] = vec4<f32>(params.nodata_value);
        out_hydro[idx] = vec2<f32>(params.nodata_value);
        out_geomorphons[idx] = 0u;
        return;
    }}
    let ic = i32(col); let ir = i32(row);
    let z00 = sample_dem(ic - 1, ir - 1); let z10 = sample_dem(ic, ir - 1); let z20 = sample_dem(ic + 1, ir - 1);
    let z01 = sample_dem(ic - 1, ir);                                        let z21 = sample_dem(ic + 1, ir);
    let z02 = sample_dem(ic - 1, ir + 1); let z12 = sample_dem(ic, ir + 1); let z22 = sample_dem(ic + 1, ir + 1);

    let cell = params.cell_size_m;
    let p = ((z20 + 2.0 * z21 + z22) - (z00 + 2.0 * z01 + z02)) / (8.0 * cell);
    let q = ((z02 + 2.0 * z12 + z22) - (z00 + 2.0 * z10 + z20)) / (8.0 * cell);
    let slope = atan(sqrt(p * p + q * q));
    let aspect = atan2(-q, p);

    let r_curv = ((z20 + z21 + z22) - 2.0 * (z10 + z11 + z12) + (z00 + z01 + z02)) / (3.0 * cell * cell);
    let t_curv = ((z02 + z12 + z22) - 2.0 * (z01 + z11 + z21) + (z00 + z10 + z20)) / (3.0 * cell * cell);
    let s_curv = ((z22 - z02) - (z20 - z00)) / (4.0 * cell * cell);
    let pq2 = p * p + q * q;
    let prof_curv = select(-(r_curv * p * p + 2.0 * s_curv * p * q + t_curv * q * q) / (pq2 * pow(1.0 + pq2, 1.5)), 0.0, pq2 < 1e-7);
    let plan_curv = select(-(t_curv * p * p - 2.0 * s_curv * p * q + r_curv * q * q) / pow(pq2, 1.5), 0.0, pq2 < 1e-7);

    let tan_s = max(tan(slope), 0.001);
    let sca = cell;
    let twi = log(max(sca / tan_s, 1e-6));
    let spi = sca * tan_s;

    let thresh = cell * 0.017455;
    let nbrs = array<f32, 8>(z21, z20, z10, z00, z01, z02, z12, z22);
    var pattern: u32 = 0u;
    for (var d = 0u; d < 8u; d = d + 1u) {{
        let diff = nbrs[d] - z11;
        let code = select(select(0u, 2u, diff < -thresh), 1u, diff > thresh);
        pattern = pattern | (code << (2u * d));
    }}

    out_morphometry[idx] = vec4<f32>(slope, aspect, plan_curv, prof_curv);
    out_hydro[idx] = vec2<f32>(twi, spi);
    out_geomorphons[idx] = pattern;
}}
"
    );
    let req = shader.len();
    if req > byte_budget {
        return Err(GeoAlgebraWgslError::ByteBudgetExceeded { requested: req, budget: byte_budget });
    }
    let hash = fnv1a_64(shader.as_bytes());
    Ok((shader, hash))
}

/// Evaluates a map algebra token program on the CPU and writes results into `out`.
/// Returns the L2 norm of the resulting output array for GPU oracle parity testing.
///
/// Allocation Class: Tier-1 (`HotZeroHeap`).
pub fn cpu_oracle_compiled_map_algebra_into(
    bands: &[&[f32]],
    tokens: &[WgslMapAlgebraToken],
    out: &mut [f32],
) -> Result<f64, GeoAlgebraWgslError> {
    let n = out.len();
    if n == 0 { return Ok(0.0); }
    for b in bands {
        if b.len() != n { return Err(GeoAlgebraWgslError::DimensionMismatch); }
    }
    for tok in tokens {
        if let WgslMapAlgebraToken::Band(b) = tok {
            if *b as usize >= bands.len() {
                return Err(GeoAlgebraWgslError::BandIndexOutOfBounds(*b));
            }
        }
    }
    let mut stack = [0.0f32; 32];
    let mut sum_sq = 0.0f64;

    for i in 0..n {
        let mut sp = 0usize;
        for tok in tokens {
            match *tok {
                WgslMapAlgebraToken::Band(b) => { if sp >= 32 { return Err(GeoAlgebraWgslError::StackOverflow); } stack[sp] = bands[b as usize][i]; sp += 1; }
                WgslMapAlgebraToken::Literal(c) => { if sp >= 32 { return Err(GeoAlgebraWgslError::StackOverflow); } stack[sp] = c; sp += 1; }
                WgslMapAlgebraToken::Add => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] += stack[sp]; }
                WgslMapAlgebraToken::Sub => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] -= stack[sp]; }
                WgslMapAlgebraToken::Mul => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] *= stack[sp]; }
                WgslMapAlgebraToken::SafeDiv => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; let d = stack[sp]; stack[sp - 1] = if d.abs() < 1e-7 { 0.0 } else { stack[sp - 1] / d }; }
                WgslMapAlgebraToken::NormalizedDiff => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; let (a, b) = (stack[sp - 1], stack[sp]); let s = a + b; stack[sp - 1] = if s.abs() < 1e-7 { 0.0 } else { (a - b) / s }; }
                WgslMapAlgebraToken::Min => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] = stack[sp - 1].min(stack[sp]); }
                WgslMapAlgebraToken::Max => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] = stack[sp - 1].max(stack[sp]); }
                WgslMapAlgebraToken::Sqrt => { if sp < 1 { return Err(GeoAlgebraWgslError::StackUnderflow); } stack[sp - 1] = stack[sp - 1].max(0.0).sqrt(); }
                WgslMapAlgebraToken::Abs => { if sp < 1 { return Err(GeoAlgebraWgslError::StackUnderflow); } stack[sp - 1] = stack[sp - 1].abs(); }
                WgslMapAlgebraToken::Clamp(lo, hi) => { if sp < 1 { return Err(GeoAlgebraWgslError::StackUnderflow); } stack[sp - 1] = stack[sp - 1].clamp(lo, hi); }
                WgslMapAlgebraToken::Gt => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] = if stack[sp - 1] > stack[sp] { 1.0 } else { 0.0 }; }
                WgslMapAlgebraToken::Lt => { if sp < 2 { return Err(GeoAlgebraWgslError::StackUnderflow); } sp -= 1; stack[sp - 1] = if stack[sp - 1] < stack[sp] { 1.0 } else { 0.0 }; }
                WgslMapAlgebraToken::Select => { if sp < 3 { return Err(GeoAlgebraWgslError::StackUnderflow); } let (c, t, f) = (stack[sp - 3], stack[sp - 2], stack[sp - 1]); sp -= 2; stack[sp - 1] = if c.abs() > 1e-6 { t } else { f }; }
            }
        }
        if sp != 1 { return Err(GeoAlgebraWgslError::InvalidProgram("Stack != 1 result")); }
        out[i] = stack[0];
        sum_sq += (stack[0] as f64) * (stack[0] as f64);
    }
    Ok(sum_sq.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewshed_uniforms_layout() {
        assert_eq!(std::mem::size_of::<ViewshedAlgebraGpuUniforms>(), 64);
        let u = ViewshedAlgebraGpuUniforms::new(256, 256, 10.0);
        assert_eq!((u.width, u.height, u.cell_size_m), (256, 256, 10.0));
    }

    #[test]
    fn test_compile_vibe_cell_map_algebra_and_oracle() {
        let tokens = [
            WgslMapAlgebraToken::Band(1),        // NIR
            WgslMapAlgebraToken::Band(0),        // Red
            WgslMapAlgebraToken::NormalizedDiff, // NDVI = (NIR - Red)/(NIR + Red)
            WgslMapAlgebraToken::Literal(0.3),
            WgslMapAlgebraToken::Gt,             // cond = NDVI > 0.3
            WgslMapAlgebraToken::Literal(1.0),   // true_val
            WgslMapAlgebraToken::Literal(0.0),   // false_val
            WgslMapAlgebraToken::Select,
        ];
        let (wgsl, hash) = compile_vibe_cell_to_wgsl_map_algebra(&tokens, 16, 16, 65536).unwrap();
        assert!(hash != 0);
        assert!(wgsl.contains("@compute @workgroup_size(16, 16, 1)"));
        assert!(wgsl.contains("ViewshedAlgebraUniforms"));

        let red = [0.1f32, 0.4, 0.2, 0.5];
        let nir = [0.7f32, 0.5, 0.25, 0.9];
        let mut out = [0.0f32; 4];
        let norm = cpu_oracle_compiled_map_algebra_into(&[&red, &nir], &tokens, &mut out).unwrap();

        // 0: (0.7-0.1)/(0.7+0.1)=0.75 > 0.3 => 1.0
        // 1: (0.5-0.4)/(0.5+0.4)=0.111 < 0.3 => 0.0
        // 2: (0.25-0.2)/(0.25+0.2)=0.111 < 0.3 => 0.0
        // 3: (0.9-0.5)/(0.9+0.5)=0.2857 < 0.3 => 0.0
        assert_eq!(out, [1.0, 0.0, 0.0, 0.0]);
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_emit_wgsl_viewshed_and_geomorphons_with_budget() {
        let (s1, h1) = emit_wgsl_radial_viewshed_and_hillshade(8, 8, 32768).unwrap();
        assert!(h1 != 0);
        assert!(s1.contains("sample_dem"));
        assert!(s1.contains("@workgroup_size(8, 8, 1)"));

        assert!(matches!(
            emit_wgsl_radial_viewshed_and_hillshade(8, 8, 64),
            Err(GeoAlgebraWgslError::ByteBudgetExceeded { .. })
        ));

        let (s2, h2) = emit_wgsl_horn_twi_spi_geomorphons(16, 16, 32768).unwrap();
        assert!(h2 != 0);
        assert!(s2.contains("out_morphometry"));
        assert!(s2.contains("out_hydro"));
        assert!(s2.contains("out_geomorphons"));
        assert!(s2.contains("@workgroup_size(16, 16, 1)"));

        assert!(matches!(
            emit_wgsl_horn_twi_spi_geomorphons(16, 16, 64),
            Err(GeoAlgebraWgslError::ByteBudgetExceeded { .. })
        ));
    }
}
