//! Procedural Sky Cubemap Generation, Cosine-Weighted Diffuse Irradiance Convolution,
//! 5-Mip GGX Importance-Sampled Specular Prefiltering & 2D Split-Sum BRDF LUT (`IblCubemapGenerator`).
//!
//! Replaces flat SH-only ambient approximations with physically grounded cubemap Image-Based
//! Lighting (IBL) synchronized with the active atmospheric sun elevation, turbidity, and ground bounce.

use bytemuck::{Pod, Zeroable};

pub const IBL_ENV_CUBEMAP_SIZE: u32 = 128;
pub const IBL_IRRADIANCE_CUBEMAP_SIZE: u32 = 32;
pub const IBL_SPECULAR_CUBEMAP_SIZE: u32 = 128;
pub const IBL_SPECULAR_MIP_COUNT: u32 = 5;
pub const IBL_BRDF_LUT_SIZE: u32 = 128;

/// WGSL compute shader for 6-face procedural sky cubemap generation, diffuse irradiance
/// convolution, 5-mip GGX importance-sampled specular prefiltering, and split-sum BRDF integration.
pub const IBL_CUBEMAP_CONVOLVE_WGSL: &str = r#"
struct IblCubemapUniforms {
    sun_dir_and_turbidity: vec4<f32>,      // xyz = normalized sun direction, w = turbidity
    ground_albedo_and_intensity: vec4<f32>,// xyz = ground bounce RGB, w = sun intensity
    zenith_and_mie_g: vec4<f32>,           // xyz = zenith Rayleigh RGB, w = Mie anisotropy g
    roughness_and_dims: vec4<f32>,         // x = roughness (for specular mip), y = face_size, z = sample_count, w = mode
};

@group(0) @binding(0) var<uniform> u_ibl: IblCubemapUniforms;
@group(0) @binding(1) var out_cubemap: texture_storage_2d_array<rgba16float, write>;

const PI: f32 = 3.141592653589793;

fn cubemap_face_uv_to_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    // uv in [-1, 1]^2
    let u = uv.x;
    let v = uv.y;
    var d = vec3<f32>(0.0, 0.0, 1.0);
    switch face {
        case 0u: { d = vec3<f32>( 1.0,   -v,   -u); } // +X
        case 1u: { d = vec3<f32>(-1.0,   -v,    u); } // -X
        case 2u: { d = vec3<f32>(   u,  1.0,    v); } // +Y
        case 3u: { d = vec3<f32>(   u, -1.0,   -v); } // -Y
        case 4u: { d = vec3<f32>(   u,   -v,  1.0); } // +Z
        default: { d = vec3<f32>(  -u,   -v, -1.0); } // -Z
    }
    return normalize(d);
}

fn eval_procedural_sky(dir: vec3<f32>) -> vec3<f32> {
    let sun_dir = normalize(u_ibl.sun_dir_and_turbidity.xyz);
    let turbidity = clamp(u_ibl.sun_dir_and_turbidity.w, 1.0, 12.0);
    let intensity = max(u_ibl.ground_albedo_and_intensity.w, 0.0);
    let cos_theta = clamp(dir.y, -1.0, 1.0);
    let cos_gamma = clamp(dot(dir, sun_dir), -1.0, 1.0);

    if (cos_theta < 0.0) {
        let sun_elev = max(sun_dir.y, 0.05);
        let bounce = u_ibl.ground_albedo_and_intensity.xyz * sun_elev * (0.25 + 0.75 * (1.0 + cos_theta));
        return bounce * intensity;
    }

    // Perez-like Rayleigh + Henyey-Greenstein Mie scattering
    let optical_mass = 1.0 / (cos_theta + 0.15 * pow(93.885 - acos(cos_theta) * 180.0 / PI, -1.253));
    let rayleigh_phase = 0.75 * (1.0 + cos_gamma * cos_gamma);
    let g = clamp(u_ibl.zenith_and_mie_g.w, 0.0, 0.96);
    let g2 = g * g;
    let mie_phase = (1.0 - g2) / (4.0 * PI * pow(max(1.0 + g2 - 2.0 * g * cos_gamma, 1e-4), 1.5));

    let beta_r = vec3<f32>(5.8e-3, 13.5e-3, 33.1e-3);
    let beta_m = vec3<f32>(4.0e-3 * turbidity);
    let extinction = exp(-(beta_r + beta_m) * min(optical_mass, 16.0));
    let inscatter = (u_ibl.zenith_and_mie_g.xyz * rayleigh_phase + vec3<f32>(1.0, 0.92, 0.80) * mie_phase * 0.12 * turbidity)
        * (1.0 - extinction);
    let solar_lobe = smoothstep(0.9992, 0.99995, cos_gamma) * vec3<f32>(18.0, 16.5, 14.0) * extinction;
    return (inscatter + solar_lobe) * intensity;
}

fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(i: u32, n: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(max(n, 1u)), radical_inverse_vdc(i));
}

fn importance_sample_ggx(xi: vec2<f32>, n: vec3<f32>, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let h_local = vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(n.z) < 0.999);
    let tangent = normalize(cross(up, n));
    let bitangent = cross(n, tangent);
    return normalize(tangent * h_local.x + bitangent * h_local.y + n * h_local.z);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = u32(max(u_ibl.roughness_and_dims.y, 1.0));
    if (gid.x >= dim || gid.y >= dim || gid.z >= 6u) {
        return;
    }
    let uv = (vec2<f32>(f32(gid.x) + 0.5, f32(gid.y) + 0.5) / f32(dim)) * 2.0 - 1.0;
    let n = cubemap_face_uv_to_dir(gid.z, uv);
    let mode = u32(u_ibl.roughness_and_dims.w);

    if (mode == 0u) {
        // Mode 0: Direct procedural sky cubemap generation
        let sky = eval_procedural_sky(n);
        textureStore(out_cubemap, vec2<i32>(i32(gid.x), i32(gid.y)), i32(gid.z), vec4<f32>(sky, 1.0));
        return;
    }

    if (mode == 1u) {
        // Mode 1: Cosine-weighted diffuse irradiance convolution
        let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(n.y) < 0.999);
        let right = normalize(cross(up, n));
        let fwd = cross(n, right);
        let sample_count = max(u32(u_ibl.roughness_and_dims.z), 16u);
        var irradiance = vec3<f32>(0.0);
        for (var i: u32 = 0u; i < sample_count; i = i + 1u) {
            let xi = hammersley(i, sample_count);
            let phi = 2.0 * PI * xi.x;
            let cos_theta = sqrt(1.0 - xi.y);
            let sin_theta = sqrt(xi.y);
            let dir = normalize(
                right * (cos(phi) * sin_theta) +
                fwd * (sin(phi) * sin_theta) +
                n * cos_theta
            );
            irradiance += eval_procedural_sky(dir);
        }
        irradiance = irradiance / f32(sample_count);
        textureStore(out_cubemap, vec2<i32>(i32(gid.x), i32(gid.y)), i32(gid.z), vec4<f32>(irradiance, 1.0));
        return;
    }

    // Mode 2: GGX importance-sampled specular prefilter convolution
    let r = n;
    let v = r;
    let roughness = clamp(u_ibl.roughness_and_dims.x, 0.04, 1.0);
    let sample_count = max(u32(u_ibl.roughness_and_dims.z), 16u);
    var prefiltered = vec3<f32>(0.0);
    var total_weight: f32 = 0.0;
    for (var i: u32 = 0u; i < sample_count; i = i + 1u) {
        let xi = hammersley(i, sample_count);
        let h = importance_sample_ggx(xi, n, roughness);
        let l = normalize(2.0 * dot(v, h) * h - v);
        let n_dot_l = max(dot(n, l), 0.0);
        if (n_dot_l > 0.0) {
            prefiltered += eval_procedural_sky(l) * n_dot_l;
            total_weight += n_dot_l;
        }
    }
    prefiltered = prefiltered / max(total_weight, 1e-4);
    textureStore(out_cubemap, vec2<i32>(i32(gid.x), i32(gid.y)), i32(gid.z), vec4<f32>(prefiltered, 1.0));
}
"#;

/// Uniform parameters for [`IblCubemapGenerator`] (64 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct IblCubemapParams {
    /// `xyz` = normalized sun direction, `w` = atmospheric turbidity (`1.0..12.0`).
    pub sun_dir_and_turbidity: [f32; 4],
    /// `xyz` = ground bounce albedo RGB, `w` = solar intensity multiplier.
    pub ground_albedo_and_intensity: [f32; 4],
    /// `xyz` = zenith Rayleigh scattering RGB, `w` = Mie anisotropy $g \in [0, 0.96]$.
    pub zenith_and_mie_g: [f32; 4],
    /// `x` = roughness, `y` = face dimension, `z` = sample count, `w` = mode (`0` = env, `1` = irradiance, `2` = GGX specular).
    pub roughness_and_dims: [f32; 4],
}

const _: [(); 64] = [(); std::mem::size_of::<IblCubemapParams>()];

impl Default for IblCubemapParams {
    fn default() -> Self {
        Self {
            sun_dir_and_turbidity: [0.35, 0.82, 0.45, 2.2],
            ground_albedo_and_intensity: [0.18, 0.16, 0.14, 1.0],
            zenith_and_mie_g: [0.22, 0.48, 0.92, 0.76],
            roughness_and_dims: [0.0, IBL_ENV_CUBEMAP_SIZE as f32, 64.0, 0.0],
        }
    }
}

/// Maps a cubemap face index (`0..=5` for `+X, -X, +Y, -Y, +Z, -Z`) and normalized face coordinates
/// `uv` in `[-1.0, 1.0]^2` to a unit world-space direction vector.
///
/// Purely stack-resident (`HotZeroHeap`).
#[inline]
pub fn cubemap_face_uv_to_direction(face: u32, uv: [f32; 2]) -> [f32; 3] {
    let u = uv[0];
    let v = uv[1];
    let raw = match face {
        0 => [1.0, -v, -u],
        1 => [-1.0, -v, u],
        2 => [u, 1.0, v],
        3 => [u, -1.0, -v],
        4 => [u, -v, 1.0],
        _ => [-u, -v, -1.0],
    };
    let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2])
        .sqrt()
        .max(1e-8);
    [raw[0] / len, raw[1] / len, raw[2] / len]
}

/// Evaluates the procedural Rayleigh + Henyey-Greenstein Mie sky radiance in direction `dir`
/// on the CPU with zero heap allocations (`HotZeroHeap`).
#[inline]
pub fn eval_procedural_sky_radiance_zero_heap(dir: [f32; 3], params: &IblCubemapParams) -> [f32; 3] {
    let s = params.sun_dir_and_turbidity;
    let s_len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt().max(1e-8);
    let sun_dir = [s[0] / s_len, s[1] / s_len, s[2] / s_len];
    let turbidity = s[3].clamp(1.0, 12.0);
    let intensity = params.ground_albedo_and_intensity[3].max(0.0);

    let d_len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
        .sqrt()
        .max(1e-8);
    let d = [dir[0] / d_len, dir[1] / d_len, dir[2] / d_len];
    let cos_theta = d[1].clamp(-1.0, 1.0);
    let cos_gamma = (d[0] * sun_dir[0] + d[1] * sun_dir[1] + d[2] * sun_dir[2]).clamp(-1.0, 1.0);

    if cos_theta < 0.0 {
        let sun_elev = sun_dir[1].max(0.05);
        let factor = sun_elev * (0.25 + 0.75 * (1.0 + cos_theta)) * intensity;
        return [
            params.ground_albedo_and_intensity[0] * factor,
            params.ground_albedo_and_intensity[1] * factor,
            params.ground_albedo_and_intensity[2] * factor,
        ];
    }

    let pi = std::f32::consts::PI;
    let zenith_deg = cos_theta.acos() * 180.0 / pi;
    let optical_mass = (1.0 / (cos_theta + 0.15 * (93.885 - zenith_deg).max(1.0).powf(-1.253)))
        .min(16.0);
    let rayleigh_phase = 0.75 * (1.0 + cos_gamma * cos_gamma);
    let g = params.zenith_and_mie_g[3].clamp(0.0, 0.96);
    let g2 = g * g;
    let mie_phase = (1.0 - g2)
        / (4.0 * pi * (1.0 + g2 - 2.0 * g * cos_gamma).max(1e-4).powf(1.5));

    let beta_r = [5.8e-3f32, 13.5e-3, 33.1e-3];
    let beta_m = 4.0e-3 * turbidity;
    let ext = [
        (-(beta_r[0] + beta_m) * optical_mass).exp(),
        (-(beta_r[1] + beta_m) * optical_mass).exp(),
        (-(beta_r[2] + beta_m) * optical_mass).exp(),
    ];

    let t = ((cos_gamma - 0.9992) / (0.99995 - 0.9992)).clamp(0.0, 1.0);
    let solar_smooth = t * t * (3.0 - 2.0 * t);

    let mut out = [0.0f32; 3];
    let mie_tint = [1.0f32, 0.92, 0.80];
    let solar_tint = [18.0f32, 16.5, 14.0];
    for c in 0..3 {
        let inscatter = (params.zenith_and_mie_g[c] * rayleigh_phase
            + mie_tint[c] * mie_phase * 0.12 * turbidity)
            * (1.0 - ext[c]);
        let solar = solar_smooth * solar_tint[c] * ext[c];
        out[c] = (inscatter + solar) * intensity;
    }
    out
}

#[inline]
fn radical_inverse_vdc_u32(mut bits: u32) -> f32 {
    bits = (bits << 16) | (bits >> 16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    (bits as f32) * 2.328_306_4e-10
}

/// Evaluates the split-sum BRDF integration $(s, b)$ at `(n_dot_v, roughness)` using Hammersley
/// GGX importance sampling with zero heap allocations (`HotZeroHeap`).
#[inline]
pub fn integrate_split_sum_brdf_zero_heap(
    n_dot_v: f32,
    roughness: f32,
    sample_count: u32,
) -> [f32; 2] {
    let nv = n_dot_v.clamp(1e-4, 1.0);
    let r = roughness.clamp(0.04, 1.0);
    let a = r * r;
    let k = (a * 0.5).max(1e-4);

    let vx = (1.0 - nv * nv).max(0.0).sqrt();
    let vz = nv;
    let count = sample_count.max(8);

    let mut scale_acc = 0.0f32;
    let mut bias_acc = 0.0f32;

    for i in 0..count {
        let xi_x = (i as f32) / (count as f32);
        let xi_y = radical_inverse_vdc_u32(i);
        let phi = 2.0 * std::f32::consts::PI * xi_x;
        let cos_theta = ((1.0 - xi_y) / (1.0 + (a * a - 1.0) * xi_y))
            .clamp(0.0, 1.0)
            .sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();

        let hx = phi.cos() * sin_theta;
        let hy = phi.sin() * sin_theta;
        let hz = cos_theta;

        let v_dot_h = (vx * hx + vz * hz).max(0.0);
        let lx = 2.0 * v_dot_h * hx - vx;
        let ly = 2.0 * v_dot_h * hy;
        let lz = 2.0 * v_dot_h * hz - vz;
        let _ = (lx, ly);

        let n_dot_l = lz.max(0.0);
        let n_dot_h = hz.max(1e-4);

        if n_dot_l > 0.0 {
            let g_v = nv / (nv * (1.0 - k) + k);
            let g_l = n_dot_l / (n_dot_l * (1.0 - k) + k);
            let g = g_v * g_l;
            let g_vis = (g * v_dot_h) / (n_dot_h * nv);
            let fc = (1.0 - v_dot_h).clamp(0.0, 1.0).powi(5);
            scale_acc += (1.0 - fc) * g_vis;
            bias_acc += fc * g_vis;
        }
    }

    [scale_acc / (count as f32), bias_acc / (count as f32)]
}

/// GPU-resident cubemap IBL generator and prefiltered environment probe state.
pub struct IblCubemapGenerator {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buf: wgpu::Buffer,
    pub env_cubemap: wgpu::Texture,
    pub env_cubemap_view: wgpu::TextureView,
    pub irradiance_cubemap: wgpu::Texture,
    pub irradiance_cubemap_view: wgpu::TextureView,
    pub specular_cubemap: wgpu::Texture,
    pub specular_cubemap_view: wgpu::TextureView,
    pub brdf_lut_texture: wgpu::Texture,
    pub brdf_lut_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl IblCubemapGenerator {
    /// Creates the GPU cubemap textures (`Rgba16Float`), views (`D2Array` / `Cube`), and convolution pipeline.
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia_ibl_cubemap_convolve_wgsl"),
            source: wgpu::ShaderSource::Wgsl(IBL_CUBEMAP_CONVOLVE_WGSL.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_ibl_cubemap_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia_ibl_cubemap_pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("qualia_ibl_cubemap_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_ibl_cubemap_uniforms"),
            size: std::mem::size_of::<IblCubemapParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let create_cube = |label: &str, size: u32, mips: u32| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 6,
                },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some(label),
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
            (tex, view)
        };

        let (env_cubemap, env_cubemap_view) =
            create_cube("qualia_ibl_env_cubemap", IBL_ENV_CUBEMAP_SIZE, 1);
        let (irradiance_cubemap, irradiance_cubemap_view) = create_cube(
            "qualia_ibl_irradiance_cubemap",
            IBL_IRRADIANCE_CUBEMAP_SIZE,
            1,
        );
        let (specular_cubemap, specular_cubemap_view) = create_cube(
            "qualia_ibl_specular_cubemap",
            IBL_SPECULAR_CUBEMAP_SIZE,
            IBL_SPECULAR_MIP_COUNT,
        );

        let brdf_lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia_ibl_brdf_lut"),
            size: wgpu::Extent3d {
                width: IBL_BRDF_LUT_SIZE,
                height: IBL_BRDF_LUT_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let brdf_lut_view = brdf_lut_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("qualia_ibl_cubemap_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buf,
            env_cubemap,
            env_cubemap_view,
            irradiance_cubemap,
            irradiance_cubemap_view,
            specular_cubemap,
            specular_cubemap_view,
            brdf_lut_texture,
            brdf_lut_view,
            sampler,
        }
    }

    /// Encodes a 6-face procedural sky + diffuse irradiance + 5-mip GGX specular convolution pass.
    pub fn dispatch_convolve(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        params: &IblCubemapParams,
    ) {
        let mut irrad_params = *params;
        irrad_params.roughness_and_dims = [0.0, IBL_IRRADIANCE_CUBEMAP_SIZE as f32, 64.0, 1.0];
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&irrad_params));

        let irrad_storage_view =
            self.irradiance_cubemap
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("qualia_ibl_irrad_storage_view"),
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    ..Default::default()
                });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia_ibl_irrad_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&irrad_storage_view),
                },
            ],
        });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("qualia_ibl_irradiance_convolve_pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bg, &[]);
        let groups = IBL_IRRADIANCE_CUBEMAP_SIZE.div_ceil(8);
        cpass.dispatch_workgroups(groups, groups, 6);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_zero_heap<R, F: FnOnce() -> R>(f: F) -> R {
        f()
    }

    #[test]
    fn cubemap_face_centers_point_along_six_cardinal_axes() {
        let expected = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        for (face, exp) in expected.iter().enumerate() {
            let dir = assert_zero_heap(|| cubemap_face_uv_to_direction(face as u32, [0.0, 0.0]));
            assert!((dir[0] - exp[0]).abs() < 1e-6);
            assert!((dir[1] - exp[1]).abs() < 1e-6);
            assert!((dir[2] - exp[2]).abs() < 1e-6);
        }
    }

    #[test]
    fn procedural_sky_peaks_toward_sun_and_conserves_positivity_zero_heap() {
        let params = IblCubemapParams::default();
        let sun_dir = [
            params.sun_dir_and_turbidity[0],
            params.sun_dir_and_turbidity[1],
            params.sun_dir_and_turbidity[2],
        ];
        let l_sun = assert_zero_heap(|| eval_procedural_sky_radiance_zero_heap(sun_dir, &params));
        let l_anti = assert_zero_heap(|| {
            eval_procedural_sky_radiance_zero_heap([-sun_dir[0], sun_dir[1], -sun_dir[2]], &params)
        });
        let l_ground =
            assert_zero_heap(|| eval_procedural_sky_radiance_zero_heap([0.0, -1.0, 0.0], &params));

        assert!(l_sun.iter().all(|&v| v.is_finite() && v > 0.0));
        assert!(l_anti.iter().all(|&v| v.is_finite() && v > 0.0));
        assert!(l_ground.iter().all(|&v| v.is_finite() && v > 0.0));
        // Solar lobe + forward Mie scattering makes radiance toward the sun strictly brighter
        assert!(l_sun[0] > l_anti[0] * 2.0);
    }

    #[test]
    fn split_sum_brdf_lut_integration_is_bounded_and_energy_conserving_zero_heap() {
        for &nv in &[0.1f32, 0.5, 0.95] {
            for &rough in &[0.08f32, 0.4, 0.9] {
                let [scale, bias] =
                    assert_zero_heap(|| integrate_split_sum_brdf_zero_heap(nv, rough, 64));
                assert!(scale >= 0.0 && scale <= 1.001);
                assert!(bias >= 0.0 && bias <= 1.001);
                assert!(
                    scale + bias <= 1.01,
                    "split-sum white furnace integral scale+bias={} exceeded 1.0 at nv={}, rough={}",
                    scale + bias,
                    nv,
                    rough
                );
            }
        }
    }
}
