// Section 20 (SURF1): Hardware Instanced Surfel Quad Vertex & Fragment Pipeline.
//
// Expands 32-byte SURF1 primitives into world-space oriented elliptical quads
// using the vertex_index trick (0 to 3) without a bound vertex buffer.
// Employs Stochastic Blue-Noise Discard to resolve Order-Independent Transparency (OIT)
// on deferred G-Buffers, allowing TAA to temporally accumulate smooth volumetric Gaussian splats.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    temporal_jitter: vec4<f32>, // xy = jitter in NDC, z = frame_index, w = golden_ratio_offset
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var blue_noise_tex: texture_2d<f32>;

struct SurfelInstance {
    @location(0) position: vec3<f32>,
    @location(1) normal_oct32: u32,
    @location(2) radii: vec2<f32>,         // Major (x) and Minor (y) splat radii (f16 unpacked)
    @location(3) spectral_coeffs: vec3<f32>,// Jakob-Hanika (c0, c1, c2)
    @location(4) roughness: f32,
    @location(5) flags: u32,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) spectral_coeffs: vec3<f32>,
    @location(3) roughness: f32,
    @location(4) flags: u32,
};

struct GBufferOutput {
    @location(0) gbuffer0: vec4<f32>, // c0, c1, c2, roughness
    @location(1) gbuffer1: vec4<f32>, // octahedral normal xy, metallic, flags
};

fn sign_not_zero(v: f32) -> f32 {
    return select(-1.0, 1.0, v >= 0.0);
}

// Decode 32-bit octahedral normal into unit 3D vector.
fn decode_oct32(packed: u32) -> vec3<f32> {
    let ix = f32(i32(packed & 0xFFFFu) - select(0, 65536, (packed & 0x8000u) != 0u));
    let iy = f32(i32(packed >> 16u) - select(0, 65536, (packed & 0x80000000u) != 0u));

    var x = max(-1.0, ix / 32767.0);
    var y = max(-1.0, iy / 32767.0);
    var z = 1.0 - abs(x) - abs(y);

    if (z < 0.0) {
        let old_x = x;
        x = (1.0 - abs(y)) * sign_not_zero(old_x);
        y = (1.0 - abs(old_x)) * sign_not_zero(y);
    }
    return normalize(vec3<f32>(x, y, z));
}

// Pack unit 3D normal into 2D octahedral coordinates in [0.0, 1.0]^2.
fn pack_oct16(normal: vec3<f32>) -> vec2<f32> {
    let l1 = abs(normal.x) + abs(normal.y) + abs(normal.z);
    var x = normal.x / l1;
    var y = normal.y / l1;
    if (normal.z < 0.0) {
        let ox = (1.0 - abs(y)) * sign_not_zero(x);
        let oy = (1.0 - abs(x)) * sign_not_zero(y);
        x = ox;
        y = oy;
    }
    return vec2<f32>(x, y) * 0.5 + vec2<f32>(0.5);
}

@vertex
fn vs_surfel(in: SurfelInstance, @builtin(vertex_index) vi: u32) -> VertexOutput {
    // 1. Generate Quad UVs in [-1.0, 1.0] from 4 vertices (vi in 0..3)
    let uv = vec2<f32>(
        f32(vi & 1u) * 2.0 - 1.0,
        f32(vi >> 1u) * 2.0 - 1.0
    );

    // 2. Decode Oct32 Normal
    let n = decode_oct32(in.normal_oct32);

    // 3. Frisvad (2012) Singularity-Free Orthonormal Basis Construction
    let sign_z = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (sign_z + n.z);
    let b = n.x * n.y * a;
    let tangent = vec3<f32>(1.0 + sign_z * n.x * n.x * a, sign_z * b, -sign_z * n.x);
    let bitangent = vec3<f32>(b, sign_z + n.y * n.y * a, -n.y);

    // 4. Expand quad in world space along surface tangent axes
    let world_pos = in.position 
                  + (tangent * in.radii.x * uv.x) 
                  + (bitangent * in.radii.y * uv.y);

    var out: VertexOutput;
    out.clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    out.world_normal = n;
    out.spectral_coeffs = in.spectral_coeffs;
    out.roughness = in.roughness;
    out.flags = in.flags;
    return out;
}

@fragment
fn fs_surfel(in: VertexOutput) -> GBufferOutput {
    // 1. Radial Gaussian Splat Falloff: exp(-0.5 * (u^2 + v^2) * 4.0)
    let d2 = dot(in.uv, in.uv);
    if (d2 > 1.0) {
        discard; // Outside unit ellipse
    }
    let alpha = exp(-2.0 * d2);

    // 2. Fetch screen-space Blue Noise modulated by temporal Golden Ratio offset
    let screen_coord = vec2<u32>(in.clip_pos.xy) % 64u;
    let base_noise = textureLoad(blue_noise_tex, screen_coord, 0).r;
    let dithered_noise = fract(base_noise + camera.temporal_jitter.w);

    // 3. Stochastic Blue-Noise Discard for Order-Independent Depth Sorting
    if (alpha < dithered_noise) {
        discard;
    }

    // 4. Output to Deferred G-Buffers
    var out: GBufferOutput;
    // GBuffer0 (rgba16float): c0, c1, c2, Roughness
    out.gbuffer0 = vec4<f32>(in.spectral_coeffs, in.roughness);
    // GBuffer1 (rgba8unorm): Normal.xy, Metallic, Material Flags
    let oct_norm = pack_oct16(in.world_normal);
    let metallic = select(0.0, 1.0, (in.flags & 1u) != 0u);
    let flags_norm = f32(in.flags & 0xFFu) / 255.0;
    out.gbuffer1 = vec4<f32>(oct_norm.x, oct_norm.y, metallic, flags_norm);

    return out;
}
