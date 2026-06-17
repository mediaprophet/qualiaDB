// T2 bloom post-pass — Kawase dual-filter (low bandwidth, few dispatches).
// Unified bind group: sampler + tex_a + tex_b + bloom_params + composite_params.

struct BloomParams {
    threshold: f32,
    intensity: f32,
    offset: f32,
    _pad: f32,
};

struct CompositeParams {
    exposure: f32,
    bloom_strength: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var tex_a: texture_2d<f32>;
@group(0) @binding(2) var tex_b: texture_2d<f32>;
@group(0) @binding(3) var<uniform> bloom_params: BloomParams;
@group(0) @binding(4) var<uniform> composite_params: CompositeParams;

fn fullscreen_pos(vi: u32) -> vec4<f32> {
    let pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

fn uv_from_pos(pos: vec4<f32>, dims: vec2<u32>) -> vec2<f32> {
    return pos.xy / vec2<f32>(dims);
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn reinhard(c: vec3<f32>) -> vec3<f32> {
    return c / (vec3<f32>(1.0) + c);
}

@vertex
fn extract_vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_pos(vi);
}

@fragment
fn extract_fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let full = vec2<f32>(textureDimensions(tex_a));
    // Render target is half-res — scale UV to sample full HDR scene.
    let uv = pos.xy / (full * 0.5);
    let c = textureSample(tex_a, samp, uv);
    let bright = max(c.rgb - vec3<f32>(bloom_params.threshold), vec3<f32>(0.0));
    let w = luminance(bright);
    let bloom = bright * smoothstep(bloom_params.threshold, bloom_params.threshold + 0.25, w);
    return vec4<f32>(bloom * bloom_params.intensity, 1.0);
}

@vertex
fn kawase_vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_pos(vi);
}

@fragment
fn kawase_fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(tex_a);
    let uv = uv_from_pos(pos, dims);
    let texel = 1.0 / vec2<f32>(dims);
    let o = bloom_params.offset * texel;
    var sum = textureSample(tex_a, samp, uv) * 4.0;
    sum += textureSample(tex_a, samp, uv + vec2<f32>(o.x, o.y));
    sum += textureSample(tex_a, samp, uv + vec2<f32>(-o.x, o.y));
    sum += textureSample(tex_a, samp, uv + vec2<f32>(o.x, -o.y));
    sum += textureSample(tex_a, samp, uv + vec2<f32>(-o.x, -o.y));
    return sum / 8.0;
}

@vertex
fn composite_vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_pos(vi);
}

@fragment
fn composite_fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(tex_a);
    let uv = uv_from_pos(pos, dims);
    let hdr = textureSample(tex_a, samp, uv).rgb;
    let bloom = textureSample(tex_b, samp, uv).rgb;
    let combined = (hdr + bloom * composite_params.bloom_strength) * composite_params.exposure;
    let mapped = reinhard(combined);
    return vec4<f32>(mapped, 1.0);
}