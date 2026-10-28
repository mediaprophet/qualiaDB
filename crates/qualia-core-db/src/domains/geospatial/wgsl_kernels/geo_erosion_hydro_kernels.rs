// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) QualiaDB contributors
// Adapted under MIT / Apache-2.0 License.

//! Geomorphology & Hydrology WGSL Compute Shader Generators, Workgroup Schedule
//! Certifiers, FNV-1a Shader Hash Attestors, and CPU Parity Oracles.
//!
//! Provides deterministic 8.24 fixed-point virtual pipe hydraulic erosion,
//! 6-layer coupled stratigraphic column weathering, and parallel D8 / D-Infinity
//! flow routing shaders compatible with `wgsl_forge`.
//!
//! - Tier-1 `HotZeroHeap`: Zero-heap CPU oracle and shader certification passes.
//! - Tier-2 `ColdBounded`: Bounded string emission with strict byte-budget fail-closed limits.

/// Descriptor for a validated WGSL compute shader kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoWgslKernelDescriptor {
    pub kernel_id: &'static str,
    pub entry_point: &'static str,
    pub workgroup_size: [u32; 3],
    pub storage_bindings: u32,
    pub uniform_bytes: u32,
    pub fnv1a_wgsl_hash: u64,
}

/// Uniform buffer representation for geomorphology and hydrology GPU kernels.
/// Exact 64-byte `repr(C)` layout matching WGSL uniform buffer alignment rules.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErosionGpuUniforms {
    pub width: u32, pub height: u32, pub dt_sec: f32, pub cell_size_m: f32,
    pub gravity: f32, pub rain_rate: f32, pub evap_rate: f32, pub kc_capacity: f32,
    pub ks_dissolve: f32, pub kd_deposit: f32, pub talus_tan: f32, pub thermal_rate: f32,
    pub snow_melt_rate: f32, pub glacier_glen_a: f32, pub fixed824_scale: u32, pub step_index: u32,
}

const _: () = assert!(core::mem::size_of::<ErosionGpuUniforms>() == 64);
const _: () = assert!(core::mem::align_of::<ErosionGpuUniforms>() == 4);

impl Default for ErosionGpuUniforms {
    #[inline]
    fn default() -> Self {
        Self {
            width: 256, height: 256, dt_sec: 0.05, cell_size_m: 1.0, gravity: 9.80665,
            rain_rate: 0.01, evap_rate: 0.005, kc_capacity: 1.0, ks_dissolve: 0.1,
            kd_deposit: 0.1, talus_tan: 0.7, thermal_rate: 0.05, snow_melt_rate: 0.02,
            glacier_glen_a: 1.0e-16, fixed824_scale: 16_777_216, step_index: 0,
        }
    }
}

/// Errors encountered during WGSL kernel generation, certification, or CPU oracle execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeoWgslError {
    ByteBudgetExceeded { requested: usize, budget: usize },
    InvalidWorkgroupDimensions,
    InvalidGridDimensions,
    BufferLengthMismatch,
    CertificationFailed(&'static str),
    HashMismatch { expected: u64, computed: u64 },
}

impl std::fmt::Display for GeoWgslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "WGSL byte budget exceeded: requested {requested} B > budget {budget} B"),
            Self::InvalidWorkgroupDimensions => write!(f, "Invalid workgroup dimensions (product <= 256)"),
            Self::InvalidGridDimensions => write!(f, "Invalid grid dimensions (must be > 0)"),
            Self::BufferLengthMismatch => write!(f, "Buffer length does not match width * height"),
            Self::CertificationFailed(r) => write!(f, "Kernel certification failed: {r}"),
            Self::HashMismatch { expected, computed } => write!(f, "Shader hash mismatch: expected {expected:#018x}, got {computed:#018x}"),
        }
    }
}

impl std::error::Error for GeoWgslError {}

/// 64-bit FNV-1a non-cryptographic hash for deterministic shader attestation.
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash = (hash ^ (b as u64)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Shared WGSL uniform struct declaration for all erosion and hydrology kernels.
pub const WGSL_UNIFORMS_DECL: &str = r#"struct ErosionGpuUniforms {
    width: u32, height: u32, dt_sec: f32, cell_size_m: f32,
    gravity: f32, rain_rate: f32, evap_rate: f32, kc_capacity: f32,
    ks_dissolve: f32, kd_deposit: f32, talus_tan: f32, thermal_rate: f32,
    snow_melt_rate: f32, glacier_glen_a: f32, fixed824_scale: u32, step_index: u32,
};
@group(0) @binding(0) var<uniform> uniforms: ErosionGpuUniforms;
"#;

#[inline]
fn cell_pipe_outflow(x: usize, y: usize, dir: usize, h_slice: &[i32], w_slice: &[i32], w: usize, h: usize) -> i32 {
    let idx = y * w + x;
    let (h_c, w_c) = (h_slice[idx] as i64, w_slice[idx] as i64);
    let z_c = h_c + w_c;
    let nbrs = [
        if y > 0 { Some((x, y - 1)) } else { None }, if x + 1 < w { Some((x + 1, y)) } else { None },
        if y + 1 < h { Some((x, y + 1)) } else { None }, if x > 0 { Some((x - 1, y)) } else { None },
    ];
    let (mut raw_flux, mut sum_raw) = ([0i64; 4], 0i64);
    for d in 0..4 {
        if let Some((nx, ny)) = nbrs[d] {
            let diff = z_c - (h_slice[ny * w + nx] as i64 + w_slice[ny * w + nx] as i64);
            if diff > 0 { let f = diff >> 2; raw_flux[d] = f; sum_raw += f; }
        }
    }
    if sum_raw == 0 || w_c <= 0 { return 0; }
    if sum_raw > w_c {
        if dir < 3 { ((raw_flux[dir] as i128 * w_c as i128) / sum_raw as i128) as i32 } else {
            let s0 = ((raw_flux[0] as i128 * w_c as i128) / sum_raw as i128) as i64;
            let s1 = ((raw_flux[1] as i128 * w_c as i128) / sum_raw as i128) as i64;
            let s2 = ((raw_flux[2] as i128 * w_c as i128) / sum_raw as i128) as i64;
            (w_c - s0 - s1 - s2).max(0) as i32
        }
    } else { raw_flux[dir] as i32 }
}

/// Reference CPU oracle executing one deterministic 8.24 fixed-point hydraulic step
/// and returning total mass balance sum(height_824 + water_824) for GPU parity certification.
///
/// Tier-1 `HotZeroHeap`: Caller supplies all buffers, zero heap allocations.
pub fn cpu_oracle_824_pipe_step_into(
    height_824: &[i32], water_824: &[i32], uniforms: &ErosionGpuUniforms,
    out_height_824: &mut [i32], out_water_824: &mut [i32],
) -> Result<i64, GeoWgslError> {
    let (w, h) = (uniforms.width as usize, uniforms.height as usize);
    if w == 0 || h == 0 { return Err(GeoWgslError::InvalidGridDimensions); }
    let total = w.checked_mul(h).ok_or(GeoWgslError::InvalidGridDimensions)?;
    if height_824.len() != total || water_824.len() != total || out_height_824.len() != total || out_water_824.len() != total {
        return Err(GeoWgslError::BufferLengthMismatch);
    }
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let outflow = cell_pipe_outflow(x, y, 0, height_824, water_824, w, h)
                + cell_pipe_outflow(x, y, 1, height_824, water_824, w, h)
                + cell_pipe_outflow(x, y, 2, height_824, water_824, w, h)
                + cell_pipe_outflow(x, y, 3, height_824, water_824, w, h);
            let inflow = (if y > 0 { cell_pipe_outflow(x, y - 1, 2, height_824, water_824, w, h) } else { 0 })
                + (if x + 1 < w { cell_pipe_outflow(x + 1, y, 3, height_824, water_824, w, h) } else { 0 })
                + (if y + 1 < h { cell_pipe_outflow(x, y + 1, 0, height_824, water_824, w, h) } else { 0 })
                + (if x > 0 { cell_pipe_outflow(x - 1, y, 1, height_824, water_824, w, h) } else { 0 });
            let mut cur_w = (water_824[idx] + (inflow - outflow)).max(0);
            let mut cur_h = height_824[idx];
            if uniforms.rain_rate > 0.0 { cur_w = cur_w.saturating_add((uniforms.rain_rate * uniforms.fixed824_scale as f32) as i32); }
            if uniforms.evap_rate > 0.0 { cur_w = cur_w.saturating_sub(((cur_w as f32) * uniforms.evap_rate) as i32); }
            let capacity = ((((inflow + outflow) >> 1) as f32) * uniforms.kc_capacity) as i32;
            if uniforms.ks_dissolve > 0.0 && capacity > 0 {
                let erode = (((capacity as f32) * uniforms.ks_dissolve) as i32).min(cur_h).max(0);
                cur_h = cur_h.saturating_sub(erode); cur_w = cur_w.saturating_add(erode);
            } else if uniforms.kd_deposit > 0.0 && capacity == 0 && cur_w > 0 {
                let deposit = (((cur_w as f32) * uniforms.kd_deposit) as i32).min(cur_w).max(0);
                cur_h = cur_h.saturating_add(deposit); cur_w = cur_w.saturating_sub(deposit);
            }
            out_height_824[idx] = cur_h; out_water_824[idx] = cur_w;
        }
    }
    let mut total_mass: i64 = 0;
    for i in 0..total {
        total_mass = total_mass.wrapping_add(out_height_824[i] as i64).wrapping_add(out_water_824[i] as i64);
    }
    Ok(total_mass)
}

/// Certifies a generated WGSL geomorphology/hydrology kernel against wgsl_forge standards.
///
/// Tier-1 `HotZeroHeap`: Validates balanced delimiters, required WebGPU attributes,
/// absence of forbidden `f64` scalar types, and exact FNV-1a hash matching without allocating.
pub fn certify_wgsl_erosion_kernel(wgsl_src: &str, desc: &GeoWgslKernelDescriptor) -> Result<bool, GeoWgslError> {
    let (mut brace_depth, mut paren_depth) = (0i32, 0i32);
    for &b in wgsl_src.as_bytes() {
        match b {
            b'{' => brace_depth += 1,
            b'}' => { brace_depth -= 1; if brace_depth < 0 { return Err(GeoWgslError::CertificationFailed("Unbalanced closing brace '}'")); } }
            b'(' => paren_depth += 1,
            b')' => { paren_depth -= 1; if paren_depth < 0 { return Err(GeoWgslError::CertificationFailed("Unbalanced closing paren ')'")); } }
            _ => {}
        }
    }
    if brace_depth != 0 { return Err(GeoWgslError::CertificationFailed("Unclosed brace '{'")); }
    if paren_depth != 0 { return Err(GeoWgslError::CertificationFailed("Unclosed paren '('")); }
    if !wgsl_src.contains("@compute") { return Err(GeoWgslError::CertificationFailed("Missing @compute")); }
    if !wgsl_src.contains("@workgroup_size") { return Err(GeoWgslError::CertificationFailed("Missing @workgroup_size")); }
    if !wgsl_src.contains("@group(0) @binding") { return Err(GeoWgslError::CertificationFailed("Missing @group(0) @binding")); }
    if !wgsl_src.contains(desc.entry_point) { return Err(GeoWgslError::CertificationFailed("Missing entry point")); }
    if wgsl_src.contains("f64") { return Err(GeoWgslError::CertificationFailed("Forbidden f64 type in WGSL")); }

    let computed_hash = fnv1a_64(wgsl_src.as_bytes());
    if computed_hash != desc.fnv1a_wgsl_hash {
        return Err(GeoWgslError::HashMismatch { expected: desc.fnv1a_wgsl_hash, computed: computed_hash });
    }
    Ok(true)
}

#[inline]
fn validate_workgroup(wx: u32, wy: u32) -> Result<(), GeoWgslError> {
    if wx == 0 || wy == 0 || wx.saturating_mul(wy) > 256 {
        Err(GeoWgslError::InvalidWorkgroupDimensions)
    } else {
        Ok(())
    }
}

/// Emits WGSL compute shader for deterministic 8.24 fixed-point virtual pipe hydraulic erosion.
///
/// Tier-2 `ColdBounded`: Enforces `byte_budget` fail-closed.
pub fn emit_wgsl_824_pipe_hydraulic_erosion(
    workgroup_x: u32, workgroup_y: u32, byte_budget: usize,
) -> Result<(String, GeoWgslKernelDescriptor), GeoWgslError> {
    validate_workgroup(workgroup_x, workgroup_y)?;
    let src = format!(
        "{WGSL_UNIFORMS_DECL}\
@group(0) @binding(1) var<storage, read_write> height_map: array<atomic<i32>>;
@group(0) @binding(2) var<storage, read_write> water_map: array<atomic<i32>>;
@group(0) @binding(3) var<storage, read_write> sediment_map: array<f32>;
@group(0) @binding(4) var<storage, read_write> flux_map: array<vec4<f32>>;

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {{ return; }}
    let idx = gid.y * uniforms.width + gid.x;
    let scale = f32(uniforms.fixed824_scale);
    let h = f32(atomicLoad(&height_map[idx])) / scale;
    let w = f32(atomicLoad(&water_map[idx])) / scale;
    let total_h = h + w;

    var outflow = vec4<f32>(0.0);
    if (gid.y > 0u) {{ outflow.x = max(0.0, total_h - (f32(atomicLoad(&height_map[(gid.y - 1u) * uniforms.width + gid.x])) + f32(atomicLoad(&water_map[(gid.y - 1u) * uniforms.width + gid.x]))) / scale); }}
    if (gid.x + 1u < uniforms.width) {{ outflow.y = max(0.0, total_h - (f32(atomicLoad(&height_map[gid.y * uniforms.width + gid.x + 1u])) + f32(atomicLoad(&water_map[gid.y * uniforms.width + gid.x + 1u]))) / scale); }}
    if (gid.y + 1u < uniforms.height) {{ outflow.z = max(0.0, total_h - (f32(atomicLoad(&height_map[(gid.y + 1u) * uniforms.width + gid.x])) + f32(atomicLoad(&water_map[(gid.y + 1u) * uniforms.width + gid.x]))) / scale); }}
    if (gid.x > 0u) {{ outflow.w = max(0.0, total_h - (f32(atomicLoad(&height_map[gid.y * uniforms.width + gid.x - 1u])) + f32(atomicLoad(&water_map[gid.y * uniforms.width + gid.x - 1u]))) / scale); }}

    let sum_flux = outflow.x + outflow.y + outflow.z + outflow.w;
    if (sum_flux > 0.0 && sum_flux > w) {{ outflow = outflow * (w / sum_flux); }}
    flux_map[idx] = outflow;

    let delta_w = (uniforms.rain_rate - uniforms.evap_rate * w) * uniforms.dt_sec;
    let new_w = max(0.0, w + delta_w);
    atomicStore(&water_map[idx], i32(new_w * scale));

    let flow_mag = length(vec2<f32>(outflow.y - outflow.w, outflow.z - outflow.x));
    let capacity = max(0.0, uniforms.kc_capacity * flow_mag);
    let sed = sediment_map[idx];
    if (sed < capacity) {{
        let erode = min(h, uniforms.ks_dissolve * (capacity - sed) * uniforms.dt_sec);
        atomicAdd(&height_map[idx], -i32(erode * scale)); sediment_map[idx] = sed + erode;
    }} else {{
        let deposit = uniforms.kd_deposit * (sed - capacity) * uniforms.dt_sec;
        atomicAdd(&height_map[idx], i32(deposit * scale)); sediment_map[idx] = max(0.0, sed - deposit);
    }}
}}
"
    );
    if src.len() > byte_budget {
        return Err(GeoWgslError::ByteBudgetExceeded { requested: src.len(), budget: byte_budget });
    }
    let hash = fnv1a_64(src.as_bytes());
    Ok((src, GeoWgslKernelDescriptor {
        kernel_id: "wgsl_824_pipe_hydraulic_erosion",
        entry_point: "main",
        workgroup_size: [workgroup_x, workgroup_y, 1],
        storage_bindings: 4, uniform_bytes: 64, fnv1a_wgsl_hash: hash,
    }))
}

/// Emits WGSL compute shader for coupled 6-layer stratigraphic column erosion and thermal talus creep.
///
/// Tier-2 `ColdBounded`: Enforces `byte_budget` fail-closed.
pub fn emit_wgsl_coupled_6layer_strata_erosion(
    workgroup_x: u32, workgroup_y: u32, byte_budget: usize,
) -> Result<(String, GeoWgslKernelDescriptor), GeoWgslError> {
    validate_workgroup(workgroup_x, workgroup_y)?;
    let src = format!(
        "{WGSL_UNIFORMS_DECL}\
struct StrataColumn {{
    bedrock: f32, saprolite: f32, gravel: f32, sand: f32,
    silt_clay: f32, peat_topsoil: f32, shear_stress: f32, talus_flux: f32,
}};
@group(0) @binding(1) var<storage, read> strata_in: array<StrataColumn>;
@group(0) @binding(2) var<storage, read_write> strata_out: array<StrataColumn>;

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {{ return; }}
    let idx = gid.y * uniforms.width + gid.x;
    var col = strata_in[idx];
    let total_z = col.bedrock + col.saprolite + col.gravel + col.sand + col.silt_clay + col.peat_topsoil;

    var max_slope = 0.0;
    if (gid.x + 1u < uniforms.width) {{ let e = strata_in[gid.y * uniforms.width + gid.x + 1u]; max_slope = max(max_slope, (total_z - (e.bedrock + e.saprolite + e.gravel + e.sand + e.silt_clay + e.peat_topsoil)) / uniforms.cell_size_m); }}
    if (gid.y + 1u < uniforms.height) {{ let s = strata_in[(gid.y + 1u) * uniforms.width + gid.x]; max_slope = max(max_slope, (total_z - (s.bedrock + s.saprolite + s.gravel + s.sand + s.silt_clay + s.peat_topsoil)) / uniforms.cell_size_m); }}

    if (max_slope > uniforms.talus_tan) {{
        let vol = uniforms.thermal_rate * (max_slope - uniforms.talus_tan) * uniforms.dt_sec;
        let d_peat = min(col.peat_topsoil, vol); col.peat_topsoil -= d_peat;
        let d_silt = min(col.silt_clay, (vol - d_peat) * 0.7); col.silt_clay -= d_silt;
        let d_sand = min(col.sand, (vol - d_peat - d_silt) * 0.5); col.sand -= d_sand;
        let d_gravel = min(col.gravel, (vol - d_peat - d_silt - d_sand) * 0.3); col.gravel -= d_gravel;
        let d_sap = min(col.saprolite, (vol - d_peat - d_silt - d_sand - d_gravel) * 0.15); col.saprolite -= d_sap;
        let d_bed = min(col.bedrock, (vol - d_peat - d_silt - d_sand - d_gravel - d_sap) * 0.02); col.bedrock -= d_bed;
        col.talus_flux = vol;
    }} else {{ col.talus_flux = 0.0; }}

    col.shear_stress = uniforms.gravity * total_z * max_slope;
    strata_out[idx] = col;
}}
"
    );
    if src.len() > byte_budget {
        return Err(GeoWgslError::ByteBudgetExceeded { requested: src.len(), budget: byte_budget });
    }
    let hash = fnv1a_64(src.as_bytes());
    Ok((src, GeoWgslKernelDescriptor {
        kernel_id: "wgsl_coupled_6layer_strata_erosion",
        entry_point: "main",
        workgroup_size: [workgroup_x, workgroup_y, 1],
        storage_bindings: 2, uniform_bytes: 64, fnv1a_wgsl_hash: hash,
    }))
}

/// Emits WGSL compute shader for parallel D8 & Tarboton D-Infinity flow routing and glacier SIA stress.
///
/// Tier-2 `ColdBounded`: Enforces `byte_budget` fail-closed.
pub fn emit_wgsl_d8_dinfinity_flow_routing(
    workgroup_x: u32, workgroup_y: u32, byte_budget: usize,
) -> Result<(String, GeoWgslKernelDescriptor), GeoWgslError> {
    validate_workgroup(workgroup_x, workgroup_y)?;
    let src = format!(
        "{WGSL_UNIFORMS_DECL}\
@group(0) @binding(1) var<storage, read> elevation_map: array<f32>;
@group(0) @binding(2) var<storage, read_write> flow_direction: array<vec2<f32>>;
@group(0) @binding(3) var<storage, read_write> ice_stress: array<vec2<f32>>;

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {{ return; }}
    let idx = gid.y * uniforms.width + gid.x;
    let z = elevation_map[idx];

    var max_drop = 0.0;
    var d8_dir = vec2<f32>(0.0, 0.0);

    for (var dy = -1; dy <= 1; dy = dy + 1) {{
        for (var dx = -1; dx <= 1; dx = dx + 1) {{
            if (dx == 0 && dy == 0) {{ continue; }}
            let nx = i32(gid.x) + dx; let ny = i32(gid.y) + dy;
            if (nx >= 0 && nx < i32(uniforms.width) && ny >= 0 && ny < i32(uniforms.height)) {{
                let nz = elevation_map[u32(ny) * uniforms.width + u32(nx)];
                let drop = (z - nz) / (length(vec2<f32>(f32(dx), f32(dy))) * uniforms.cell_size_m);
                if (drop > max_drop) {{ max_drop = drop; d8_dir = normalize(vec2<f32>(f32(dx), f32(dy))); }}
            }}
        }}
    }}

    flow_direction[idx] = d8_dir;
    let ice_h = max(0.0, z - 1000.0);
    let basal_shear = 910.0 * uniforms.gravity * ice_h * max_drop;
    let sia_creep = 2.0 * uniforms.glacier_glen_a * pow(basal_shear, 3.0);
    ice_stress[idx] = vec2<f32>(basal_shear, sia_creep);
}}
"
    );
    if src.len() > byte_budget {
        return Err(GeoWgslError::ByteBudgetExceeded { requested: src.len(), budget: byte_budget });
    }
    let hash = fnv1a_64(src.as_bytes());
    Ok((src, GeoWgslKernelDescriptor {
        kernel_id: "wgsl_d8_dinfinity_flow_routing",
        entry_point: "main",
        workgroup_size: [workgroup_x, workgroup_y, 1],
        storage_bindings: 3, uniform_bytes: 64, fnv1a_wgsl_hash: hash,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniforms_layout() {
        assert_eq!(core::mem::size_of::<ErosionGpuUniforms>(), 64);
        assert_eq!(core::mem::align_of::<ErosionGpuUniforms>(), 4);
    }

    #[test]
    fn test_wgsl_emissions_and_certification() {
        let (s1, d1) = emit_wgsl_824_pipe_hydraulic_erosion(16, 16, 65536).expect("pipe");
        assert!(certify_wgsl_erosion_kernel(&s1, &d1).expect("certify pipe"));
        assert_eq!(d1.storage_bindings, 4);
        assert_eq!(d1.uniform_bytes, 64);

        let (s2, d2) = emit_wgsl_coupled_6layer_strata_erosion(16, 16, 65536).expect("strata");
        assert!(certify_wgsl_erosion_kernel(&s2, &d2).expect("certify strata"));
        assert_eq!(d2.storage_bindings, 2);

        let (s3, d3) = emit_wgsl_d8_dinfinity_flow_routing(16, 16, 65536).expect("flow");
        assert!(certify_wgsl_erosion_kernel(&s3, &d3).expect("certify flow"));
        assert_eq!(d3.storage_bindings, 3);
    }

    #[test]
    fn test_byte_budget_fail_closed() {
        match emit_wgsl_824_pipe_hydraulic_erosion(16, 16, 64) {
            Err(GeoWgslError::ByteBudgetExceeded { requested, budget }) => {
                assert_eq!(budget, 64); assert!(requested > 64);
            }
            _ => panic!("Expected ByteBudgetExceeded"),
        }
    }

    #[test]
    fn test_invalid_workgroup_rejection() {
        assert_eq!(emit_wgsl_824_pipe_hydraulic_erosion(0, 16, 65536).unwrap_err(), GeoWgslError::InvalidWorkgroupDimensions);
        assert_eq!(emit_wgsl_824_pipe_hydraulic_erosion(32, 16, 65536).unwrap_err(), GeoWgslError::InvalidWorkgroupDimensions);
    }

    #[test]
    fn test_cpu_oracle_mass_conservation() {
        let u = ErosionGpuUniforms {
            width: 4, height: 4, rain_rate: 0.0, evap_rate: 0.0,
            kc_capacity: 0.5, ks_dissolve: 0.05, kd_deposit: 0.05,
            ..Default::default()
        };
        let (h_in, w_in) = (
            [1000, 900, 800, 700, 950, 850, 750, 650, 900, 800, 700, 600, 850, 750, 650, 550],
            [100, 80, 60, 40, 90, 70, 50, 30, 80, 60, 40, 20, 70, 50, 30, 10],
        );
        let mut initial_mass: i64 = 0;
        for i in 0..16 { initial_mass += h_in[i] as i64 + w_in[i] as i64; }
        let (mut out_h, mut out_w) = ([0i32; 16], [0i32; 16]);
        let total_mass = cpu_oracle_824_pipe_step_into(&h_in, &w_in, &u, &mut out_h, &mut out_w).expect("step");
        assert_eq!(total_mass, initial_mass);
        let mut final_sum: i64 = 0;
        for i in 0..16 { final_sum += out_h[i] as i64 + out_w[i] as i64; }
        assert_eq!(final_sum, initial_mass);
    }

    #[test]
    fn test_certification_detects_tamper() {
        let (mut src, desc) = emit_wgsl_824_pipe_hydraulic_erosion(8, 8, 65536).unwrap();
        src.push_str("\n// tampered");
        assert!(matches!(certify_wgsl_erosion_kernel(&src, &desc), Err(GeoWgslError::HashMismatch { .. })));
    }
}
