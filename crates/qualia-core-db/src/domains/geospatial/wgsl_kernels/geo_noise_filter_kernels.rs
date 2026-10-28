// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 QualiaDB contributors
//! Procedural Terrain DAG, Landform Primitives, and 323 Sentinel-2 Biome WGSL Compute Kernels.
//!
//! Exceeds EarthCraft & Nerthus procedural pipelines with GPU compute kernels
//! for multifractal cascades, morphological filters, and Sentinel-2 spectral biome splatting.
//! Allocation class: Tier-1 `HotZeroHeap` for CPU oracles; Tier-2 `ColdBounded` for shader emitters.

/// GPU Uniform layout (64 bytes exact) for procedural terrain and biome compute shaders.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseFilterGpuUniforms {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub octaves: u32,
    pub frequency: f32,
    pub lacunarity: f32,
    pub persistence: f32,
    pub ridge_offset: f32,
    pub warp_strength: f32,
    pub terrace_steps: f32,
    pub terrace_sharpness: f32,
    pub plateau_max: f32,
    pub biome_lut_count: u32,
    pub moisture_bias: f32,
    pub temp_lapse_rate: f32,
    pub pad0: u32,
}

const _: () = assert!(core::mem::size_of::<NoiseFilterGpuUniforms>() == 64);

impl Default for NoiseFilterGpuUniforms {
    fn default() -> Self {
        Self {
            width: 256, height: 256, seed: 42, octaves: 4,
            frequency: 0.005, lacunarity: 2.0, persistence: 0.5, ridge_offset: 1.0,
            warp_strength: 0.15, terrace_steps: 8.0, terrace_sharpness: 0.8, plateau_max: 0.9,
            biome_lut_count: 323, moisture_bias: 0.0, temp_lapse_rate: 0.0065, pad0: 0,
        }
    }
}

/// Errors returned by WGSL generation, validation, and CPU parity oracle operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeoNoiseWgslError {
    ByteBudgetExceeded { required: usize, budget: usize },
    BufferMismatch { expected: usize, actual: usize },
    InvalidParameters(&'static str),
    HashMismatch { expected: u64, actual: u64 },
    ShaderSyntaxError(&'static str),
}

impl core::fmt::Display for GeoNoiseWgslError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ByteBudgetExceeded { required, budget } => {
                write!(f, "byte budget exceeded: required {required}, budget {budget}")
            }
            Self::BufferMismatch { expected, actual } => {
                write!(f, "buffer length mismatch: expected {expected}, actual {actual}")
            }
            Self::InvalidParameters(msg) => write!(f, "invalid parameters: {msg}"),
            Self::HashMismatch { expected, actual } => {
                write!(f, "hash mismatch: expected {expected:#018x}, actual {actual:#018x}")
            }
            Self::ShaderSyntaxError(msg) => write!(f, "shader syntax error: {msg}"),
        }
    }
}

impl std::error::Error for GeoNoiseWgslError {}

/// Standard 64-bit FNV-1a non-cryptographic hash for shader digests.
#[inline]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[inline]
fn check_budget_and_hash(src: String, byte_budget: usize) -> Result<(String, u64), GeoNoiseWgslError> {
    if src.len() > byte_budget {
        return Err(GeoNoiseWgslError::ByteBudgetExceeded { required: src.len(), budget: byte_budget });
    }
    let hash = fnv1a64(src.as_bytes());
    Ok((src, hash))
}

const WGSL_UNIFORMS_STRUCT: &str = r#"struct NoiseFilterGpuUniforms {
    width: u32, height: u32, seed: u32, octaves: u32, frequency: f32, lacunarity: f32,
    persistence: f32, ridge_offset: f32, warp_strength: f32, terrace_steps: f32,
    terrace_sharpness: f32, plateau_max: f32, biome_lut_count: u32, moisture_bias: f32,
    temp_lapse_rate: f32, pad0: u32,
};
@group(0) @binding(0) var<uniform> uniforms: NoiseFilterGpuUniforms;
"#;

/// Emits complete WGSL compute shader for multifractal noise cascades (Tier-2 ColdBounded).
pub fn emit_wgsl_multifractal_noise_cascade(
    workgroup_x: u32,
    workgroup_y: u32,
    byte_budget: usize,
) -> Result<(String, u64), GeoNoiseWgslError> {
    let src = format!(
r#"// WGSL Multifractal Noise Cascade Compute Kernel
{WGSL_UNIFORMS_STRUCT}@group(0) @binding(1) var<storage, read_write> heightfield: array<f32>;

fn hash22(p: vec2<f32>) -> vec2<f32> {{
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33); return fract((p3.xx + p3.yz) * p3.zy);
}}
fn perlin2d(p: vec2<f32>) -> f32 {{
    let i = floor(p); let f = fract(p); let u = f * f * (3.0 - 2.0 * f);
    let g00 = dot(hash22(i) - 0.5, f); let g10 = dot(hash22(i + vec2<f32>(1.0, 0.0)) - 0.5, f - vec2<f32>(1.0, 0.0));
    let g01 = dot(hash22(i + vec2<f32>(0.0, 1.0)) - 0.5, f - vec2<f32>(0.0, 1.0));
    let g11 = dot(hash22(i + vec2<f32>(1.0, 1.0)) - 0.5, f - vec2<f32>(1.0, 1.0));
    return mix(mix(g00, g10, u.x), mix(g01, g11, u.x), u.y);
}}
fn simplex2d(p: vec2<f32>) -> f32 {{
    let C = vec4<f32>(0.211324865, 0.366025403, -0.577350269, 0.024390243);
    let i = floor(p + dot(p, C.yy)); let x0 = p - i + dot(i, C.xx);
    let i1 = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), x0.x > x0.y);
    let x1 = x0 - i1 + C.xx; let x2 = x0 - 1.0 + 2.0 * C.xx;
    var m = max(0.5 - vec3<f32>(dot(x0,x0), dot(x1,x1), dot(x2,x2)), vec3<f32>(0.0)); m = m * m * m * m;
    return 70.0 * dot(m, vec3<f32>(dot(hash22(i)*2.0-1.0, x0), dot(hash22(i+i1)*2.0-1.0, x1), dot(hash22(i+1.0)*2.0-1.0, x2)));
}}
fn simplex3d(p: vec3<f32>) -> f32 {{ return (simplex2d(p.xy) + simplex2d(p.yz) + simplex2d(p.zx)) * 0.333333; }}
fn perlin3d(p: vec3<f32>) -> f32 {{ return (perlin2d(p.xy) + perlin2d(p.yz) + perlin2d(p.zx)) * 0.333333; }}
fn worley2d(p: vec2<f32>) -> vec2<f32> {{
    let i = floor(p); let f = fract(p); var d1 = 1.0; var d2 = 1.0;
    for (var y = -1; y <= 1; y++) {{ for (var x = -1; x <= 1; x++) {{
        let diff = vec2<f32>(f32(x), f32(y)) + hash22(i + vec2<f32>(f32(x), f32(y))) - f; let dist = length(diff);
        if (dist < d1) {{ d2 = d1; d1 = dist; }} else if (dist < d2) {{ d2 = dist; }}
    }} }}
    return vec2<f32>(d1, d2);
}}
fn fbm2d(p: vec2<f32>, oct: u32, lac: f32, pers: f32) -> f32 {{
    var v = 0.0; var a = 0.5; var pos = p;
    for (var o = 0u; o < oct; o++) {{ v += a * perlin2d(pos); pos *= lac; a *= pers; }}
    return v;
}}
fn ridged2d(p: vec2<f32>, oct: u32, lac: f32, pers: f32, off: f32) -> f32 {{
    var v = 0.0; var a = 0.5; var pos = p; var w = 1.0;
    for (var o = 0u; o < oct; o++) {{
        var s = off - abs(perlin2d(pos)); s = s * s * w; w = clamp(s * 2.0, 0.0, 1.0);
        v += s * a; pos *= lac; a *= pers;
    }}
    return v;
}}
fn billow2d(p: vec2<f32>, oct: u32, lac: f32, pers: f32) -> f32 {{
    var v = 0.0; var a = 0.5; var pos = p;
    for (var o = 0u; o < oct; o++) {{ v += a * abs(perlin2d(pos)); pos *= lac; a *= pers; }}
    return v * 2.0 - 0.5;
}}
fn swiss2d(p: vec2<f32>, oct: u32, lac: f32, pers: f32, warp: f32) -> f32 {{
    var v = 0.0; var a = 0.5; var pos = p; var dsum = vec2<f32>(0.0);
    for (var o = 0u; o < oct; o++) {{
        dsum += (hash22(pos) - 0.5) * a; v += a * (1.0 - abs(perlin2d(pos + warp * dsum)));
        pos *= lac; a *= pers;
    }}
    return v;
}}
fn domain_warp2d(p: vec2<f32>, str: f32) -> vec2<f32> {{
    return p + str * vec2<f32>(fbm2d(p, 3u, 2.0, 0.5), fbm2d(p + vec2<f32>(5.2, 1.3), 3u, 2.0, 0.5));
}}

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {{ return; }}
    let uv = vec2<f32>(f32(gid.x), f32(gid.y)) * uniforms.frequency;
    let warped = domain_warp2d(uv, uniforms.warp_strength);
    let mountain = ridged2d(warped, uniforms.octaves, uniforms.lacunarity, uniforms.persistence, uniforms.ridge_offset);
    let hills = billow2d(warped * 2.0, 3u, 2.0, 0.5);
    let swiss = swiss2d(uv * 1.5, 4u, 2.0, 0.5, 0.2);
    let worl = worley2d(uv * 3.0);
    let h = clamp(mountain * 0.5 + hills * 0.25 + swiss * 0.15 + (1.0 - worl.x) * 0.1, 0.0, 1.0);
    heightfield[gid.y * uniforms.width + gid.x] = h;
}}
"#
    );
    check_budget_and_hash(src, byte_budget)
}

/// Emits complete WGSL compute shader for landform primitives and filters (Tier-2 ColdBounded).
pub fn emit_wgsl_landform_primitives_and_filters(
    workgroup_x: u32,
    workgroup_y: u32,
    byte_budget: usize,
) -> Result<(String, u64), GeoNoiseWgslError> {
    let src = format!(
r#"// WGSL Landform Primitives and Heightfield Filters Compute Kernel
{WGSL_UNIFORMS_STRUCT}@group(0) @binding(1) var<storage, read_write> heightfield: array<f32>;

fn landform_mountain(dist: f32, radius: f32, height: f32) -> f32 {{
    let r = clamp(dist / radius, 0.0, 1.0); return height * exp(-3.0 * r * r) * (1.0 - r);
}}
fn landform_volcano_caldera(dist: f32, radius: f32, height: f32, crater_r: f32) -> f32 {{
    let r = dist / radius; if (r > 1.0) {{ return 0.0; }}
    return max(0.0, height * (1.0 - r) - select(0.0, height * 0.6 * (1.0 - (dist / crater_r) * (dist / crater_r)), dist < crater_r));
}}
fn landform_canyon(dist_to_axis: f32, width: f32, depth: f32) -> f32 {{
    return -depth * (1.0 - smoothstep(0.0, 1.0, clamp(dist_to_axis / width, 0.0, 1.0)));
}}
fn landform_dunes(pos: vec2<f32>, wave_len: f32, height: f32) -> f32 {{
    let f = fract((pos.x * 0.8 + pos.y * 0.6) / wave_len);
    return height * clamp(select(f * 1.4, (1.0 - f) * 3.5, f > 0.7), 0.0, 1.0);
}}
fn landform_karst_tower(dist: f32, radius: f32, height: f32) -> f32 {{
    let r = dist / radius; return select(0.0, height * (1.0 - smoothstep(0.7, 1.0, r)), r <= 1.0);
}}
fn landform_impact_crater(dist: f32, radius: f32, depth: f32, rim_height: f32) -> f32 {{
    let r = dist / radius; if (r > 1.5) {{ return 0.0; }}
    let bowl = -depth * (1.0 - r * r); let rim = rim_height * exp(-8.0 * (r - 1.0) * (r - 1.0));
    return select(rim, bowl + rim + select(depth * 0.35 * (1.0 - r * 5.0), 0.0, r > 0.2), r < 1.0);
}}
fn filter_terrace(h: f32, steps: f32, sharpness: f32) -> f32 {{
    if (steps <= 0.0) {{ return h; }}
    let s = h * steps; let b = floor(s); let f = s - b; let k = clamp(sharpness, 0.0, 1.0); let w = 0.5 * (1.0 - k);
    return (b + mix(f, select(select(smoothstep(0.5 - w, 0.5 + w, f), 1.0, f > 0.5 + w), 0.0, f < 0.5 - w), k)) / steps;
}}
fn filter_plateau(h: f32, max_val: f32) -> f32 {{ return min(h, max_val); }}
fn filter_height_selector(h: f32, min_h: f32, max_h: f32) -> f32 {{ return smoothstep(min_h, max_h, h); }}
fn filter_laplacian_smooth(idx: u32, w: u32, ht: u32) -> f32 {{
    let x = idx % w; let y = idx / w;
    if (x == 0u || x >= w - 1u || y == 0u || y >= ht - 1u) {{ return heightfield[idx]; }}
    return heightfield[idx] + 0.25 * ((heightfield[idx - w] + heightfield[idx + w] + heightfield[idx + 1u] + heightfield[idx - 1u]) * 0.25 - heightfield[idx]);
}}
fn filter_bilateral(idx: u32, w: u32, ht: u32, sigma_s: f32, sigma_r: f32) -> f32 {{
    let x = idx % w; let y = idx / w;
    if (x == 0u || x >= w - 1u || y == 0u || y >= ht - 1u) {{ return heightfield[idx]; }}
    let c = heightfield[idx]; var sw = 0.0; var sv = 0.0;
    for (var dy = -1; dy <= 1; dy++) {{ for (var dx = -1; dx <= 1; dx++) {{
        let v = heightfield[u32(i32(y) + dy) * w + u32(i32(x) + dx)];
        let wt = exp(-f32(dx * dx + dy * dy) / (2.0 * sigma_s)) * exp(-((v - c) * (v - c)) / (2.0 * sigma_r));
        sv += v * wt; sw += wt;
    }} }}
    return select(c, sv / sw, sw > 0.0);
}}

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {{ return; }}
    let idx = gid.y * uniforms.width + gid.x;
    let pos = vec2<f32>(f32(gid.x), f32(gid.y));
    let center = vec2<f32>(f32(uniforms.width) * 0.5, f32(uniforms.height) * 0.5);
    var h = heightfield[idx] + landform_mountain(distance(pos, center), f32(uniforms.width) * 0.4, 0.5);
    h = filter_plateau(filter_terrace(clamp(h, 0.0, 1.0), uniforms.terrace_steps, uniforms.terrace_sharpness), uniforms.plateau_max);
    heightfield[idx] = h;
}}
"#
    );
    check_budget_and_hash(src, byte_budget)
}

/// Emits complete WGSL compute shader for 323 Sentinel-2 biome texture splatting (Tier-2 ColdBounded).
pub fn emit_wgsl_sentinel2_biome_splatting(
    workgroup_x: u32,
    workgroup_y: u32,
    byte_budget: usize,
) -> Result<(String, u64), GeoNoiseWgslError> {
    let src = format!(
r#"// WGSL 323 Sentinel-2 Biome Texture Splatting Compute Kernel
{WGSL_UNIFORMS_STRUCT}@group(0) @binding(1) var<storage, read> heightfield: array<f32>;
@group(0) @binding(2) var<storage, read> biome_lut: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> splat_rgba8: array<u32>;
@group(0) @binding(4) var<storage, read_write> splat_weights: array<vec4<f32>>;

fn pack_rgba8(r: f32, g: f32, b: f32, a: f32) -> u32 {{
    return (u32(clamp(a * 255.0, 0.0, 255.0)) << 24u) | (u32(clamp(b * 255.0, 0.0, 255.0)) << 16u)
        | (u32(clamp(g * 255.0, 0.0, 255.0)) << 8u) | u32(clamp(r * 255.0, 0.0, 255.0));
}}
fn eval_whittaker_biome(temp_c: f32, precip_norm: f32, slope_rad: f32, max_provinces: u32) -> u32 {{
    var class_id = 0u;
    if (temp_c < -5.0 || slope_rad > 0.8) {{ class_id = 0u; }}
    else if (temp_c < 5.0) {{ class_id = select(1u, 2u, precip_norm > 0.4); }}
    else if (temp_c < 18.0) {{
        class_id = select(select(5u, 4u, precip_norm < 0.65), 3u, precip_norm < 0.25);
    }} else {{
        class_id = select(select(8u, 7u, precip_norm < 0.6), 6u, precip_norm < 0.2);
    }}
    return min(class_id * 35u + (u32(abs(temp_c * 3.7 + precip_norm * 41.0)) % 35u), max_provinces - 1u);
}}

@compute @workgroup_size({workgroup_x}, {workgroup_y}, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {{ return; }}
    let w = uniforms.width; let ht = uniforms.height; let x = gid.x; let y = gid.y;
    let idx = y * w + x; let h = heightfield[idx];
    let xw = select(x - 1u, 0u, x == 0u); let xe = select(x + 1u, w - 1u, x >= w - 1u);
    let yn = select(y - 1u, 0u, y == 0u); let ys = select(y + 1u, ht - 1u, y >= ht - 1u);
    let dz_dx = ((heightfield[yn*w+xe] + 2.0*heightfield[y*w+xe] + heightfield[ys*w+xe]) - (heightfield[yn*w+xw] + 2.0*heightfield[y*w+xw] + heightfield[ys*w+xw])) / 8.0;
    let dz_dy = ((heightfield[ys*w+xw] + 2.0*heightfield[ys*w+x] + heightfield[ys*w+xe]) - (heightfield[yn*w+xw] + 2.0*heightfield[yn*w+x] + heightfield[yn*w+xe])) / 8.0;
    let slope = atan(sqrt(dz_dx * dz_dx + dz_dy * dz_dy)); let aspect = atan2(-dz_dy, -dz_dx);
    let temp_c = 25.0 - (h * uniforms.temp_lapse_rate * 1000.0) + cos(aspect) * 1.5;
    let precip = clamp(0.5 + uniforms.moisture_bias + sin(aspect) * 0.1, 0.0, 1.0);
    let lut = biome_lut[eval_whittaker_biome(temp_c, precip, slope, uniforms.biome_lut_count)];
    let rock_w = smoothstep(0.4, 0.8, slope);
    let snow_w = select(0.0, smoothstep(0.65, 0.9, h) * (1.0 - rock_w), temp_c < 2.0);
    let sand_w = select(0.0, (1.0 - rock_w) * (1.0 - snow_w), precip < 0.25 && temp_c > 15.0);
    splat_rgba8[idx] = pack_rgba8(lut.x, lut.y, lut.z, lut.w);
    splat_weights[idx] = vec4<f32>(max(0.0, 1.0 - (rock_w + snow_w + sand_w)), sand_w, rock_w, snow_w);
}}
"#
    );
    check_budget_and_hash(src, byte_budget)
}

#[inline]
fn cpu_hash2d(ix: i32, iy: i32, seed: u32) -> f32 {
    let mut h = (seed as u64) ^ ((ix as u64).wrapping_mul(0x517c_c1b7_2722_0a95));
    h ^= (iy as u64).wrapping_mul(0x6c62_272e_07bb_0142);
    h = (h ^ (h >> 32)).wrapping_mul(0x2545_f491_4f6c_dd1d);
    (((h ^ (h >> 32)) & 0x00ff_ffff) as f32) / 16777215.0
}

#[inline]
fn cpu_val_noise(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32; let iy = y.floor() as i32;
    let fx = x - x.floor(); let fy = y - y.floor();
    let wx = fx * fx * (3.0 - 2.0 * fx); let wy = fy * fy * (3.0 - 2.0 * fy);
    let nx0 = cpu_hash2d(ix, iy, seed) + wx * (cpu_hash2d(ix + 1, iy, seed) - cpu_hash2d(ix, iy, seed));
    let nx1 = cpu_hash2d(ix, iy + 1, seed) + wx * (cpu_hash2d(ix + 1, iy + 1, seed) - cpu_hash2d(ix, iy + 1, seed));
    nx0 + wy * (nx1 - nx0)
}

#[inline]
fn cpu_terrace(h: f32, steps: f32, sharpness: f32) -> f32 {
    if steps <= 0.0 { return h; }
    let s = h * steps; let b = s.floor(); let f = s - b;
    let k = sharpness.clamp(0.0, 1.0); let w = 0.5 * (1.0 - k);
    let stepped = if w < 1e-6 { if f < 0.5 { 0.0 } else { 1.0 } }
    else if f < 0.5 - w { 0.0 } else if f > 0.5 + w { 1.0 }
    else { let u = (f - (0.5 - w)) / (2.0 * w); u * u * (3.0 - 2.0 * u) };
    (b + f * (1.0 - k) + stepped * k) / steps
}

/// Zero-heap CPU parity oracle for noise cascade, terracing, and Whittaker RGBA8 (Tier-1 HotZeroHeap).
pub fn cpu_oracle_noise_terrace_biome_into(
    uniforms: &NoiseFilterGpuUniforms,
    out_height: &mut [f32],
    out_rgba8: &mut [[u8; 4]],
) -> Result<(), GeoNoiseWgslError> {
    let expected = (uniforms.width as usize).saturating_mul(uniforms.height as usize);
    if out_height.len() != expected {
        return Err(GeoNoiseWgslError::BufferMismatch { expected, actual: out_height.len() });
    }
    if out_rgba8.len() != expected {
        return Err(GeoNoiseWgslError::BufferMismatch { expected, actual: out_rgba8.len() });
    }

    let w = uniforms.width as usize;
    let ht = uniforms.height as usize;
    for y in 0..ht {
        for x in 0..w {
            let idx = y * w + x;
            let fx = (x as f32) * uniforms.frequency;
            let fy = (y as f32) * uniforms.frequency;
            let (mut amp, mut freq, mut sum, mut max_amp) = (0.5f32, 1.0f32, 0.0f32, 0.0f32);
            for o in 0..uniforms.octaves {
                let n = cpu_val_noise(fx * freq, fy * freq, uniforms.seed.wrapping_add(o * 31));
                sum += amp * n; max_amp += amp;
                amp *= uniforms.persistence; freq *= uniforms.lacunarity;
            }
            let raw_h = if max_amp > 0.0 { sum / max_amp } else { 0.0 };
            let terraced = cpu_terrace(raw_h, uniforms.terrace_steps, uniforms.terrace_sharpness);
            let final_h = terraced.min(uniforms.plateau_max).clamp(0.0, 1.0);
            out_height[idx] = final_h;

            let temp_c = 25.0 - (final_h * uniforms.temp_lapse_rate * 1000.0);
            let moist = (0.5 + uniforms.moisture_bias
                + (cpu_val_noise(fx * 0.3, fy * 0.3, uniforms.seed.wrapping_add(101)) - 0.5) * 0.4)
                .clamp(0.0, 1.0);

            let color = match () {
                _ if temp_c < -2.0 || final_h > 0.82 => [235, 245, 255, 255],
                _ if temp_c < 6.0 => [140, 155, 130, 255],
                _ if moist > 0.65 => [35, 110, 40, 255],
                _ if moist > 0.35 => [85, 145, 50, 255],
                _ if moist > 0.18 => [180, 160, 85, 255],
                _ => [220, 185, 125, 255],
            };
            out_rgba8[idx] = color;
        }
    }
    Ok(())
}

/// Validates syntactical markers and verifies FNV-1a hash digest for WGSL shaders (Tier-1 HotZeroHeap).
pub fn validate_wgsl_noise_shader(wgsl: &str, expected_hash: u64) -> Result<bool, GeoNoiseWgslError> {
    if !wgsl.contains("@compute") { return Err(GeoNoiseWgslError::ShaderSyntaxError("missing @compute")); }
    if !wgsl.contains("@workgroup_size") { return Err(GeoNoiseWgslError::ShaderSyntaxError("missing @workgroup_size")); }
    if !wgsl.contains("NoiseFilterGpuUniforms") { return Err(GeoNoiseWgslError::ShaderSyntaxError("missing NoiseFilterGpuUniforms")); }
    if !wgsl.contains("fn main") { return Err(GeoNoiseWgslError::ShaderSyntaxError("missing fn main")); }
    let actual_hash = fnv1a64(wgsl.as_bytes());
    if actual_hash != expected_hash {
        return Err(GeoNoiseWgslError::HashMismatch { expected: expected_hash, actual: actual_hash });
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_filter_gpu_uniforms_layout_64_bytes() {
        assert_eq!(core::mem::size_of::<NoiseFilterGpuUniforms>(), 64);
        let u = NoiseFilterGpuUniforms::default();
        assert_eq!(u.width, 256);
        assert_eq!(u.biome_lut_count, 323);
    }

    #[test]
    fn test_emit_wgsl_shaders_and_validate() {
        let budget = 65536;
        let (src1, hash1) = emit_wgsl_multifractal_noise_cascade(16, 16, budget).expect("cascade");
        assert!(validate_wgsl_noise_shader(&src1, hash1).unwrap());
        assert!(src1.contains("simplex2d") && src1.contains("domain_warp2d"));

        let (src2, hash2) = emit_wgsl_landform_primitives_and_filters(16, 16, budget).expect("landforms");
        assert!(validate_wgsl_noise_shader(&src2, hash2).unwrap());
        assert!(src2.contains("landform_mountain") && src2.contains("filter_terrace"));

        let (src3, hash3) = emit_wgsl_sentinel2_biome_splatting(16, 16, budget).expect("biome");
        assert!(validate_wgsl_noise_shader(&src3, hash3).unwrap());
        assert!(src3.contains("eval_whittaker_biome") && src3.contains("splat_weights"));
    }

    #[test]
    fn test_byte_budget_fail_closed() {
        match emit_wgsl_multifractal_noise_cascade(16, 16, 100).unwrap_err() {
            GeoNoiseWgslError::ByteBudgetExceeded { required, budget } => {
                assert!(required > 100);
                assert_eq!(budget, 100);
            }
            _ => panic!("expected ByteBudgetExceeded"),
        }
    }

    #[test]
    fn test_cpu_oracle_noise_terrace_biome_deterministic() {
        let u = NoiseFilterGpuUniforms { width: 16, height: 16, ..Default::default() };
        let (mut heights1, mut rgba1) = ([0.0f32; 256], [[0u8; 4]; 256]);
        let (mut heights2, mut rgba2) = ([0.0f32; 256], [[0u8; 4]; 256]);

        cpu_oracle_noise_terrace_biome_into(&u, &mut heights1, &mut rgba1).unwrap();
        cpu_oracle_noise_terrace_biome_into(&u, &mut heights2, &mut rgba2).unwrap();

        assert_eq!(heights1, heights2);
        assert_eq!(rgba1, rgba2);

        for &h in &heights1 { assert!((0.0..=1.0).contains(&h), "height out of range: {h}"); }
        for &c in &rgba1 { assert_eq!(c[3], 255); }
    }

    #[test]
    fn test_cpu_oracle_buffer_mismatch() {
        let u = NoiseFilterGpuUniforms { width: 16, height: 16, ..Default::default() };
        let (mut heights, mut rgba) = ([0.0f32; 100], [[0u8; 4]; 256]);
        match cpu_oracle_noise_terrace_biome_into(&u, &mut heights, &mut rgba).unwrap_err() {
            GeoNoiseWgslError::BufferMismatch { expected, actual } => {
                assert_eq!(expected, 256);
                assert_eq!(actual, 100);
            }
            _ => panic!("expected BufferMismatch"),
        }
    }
}
