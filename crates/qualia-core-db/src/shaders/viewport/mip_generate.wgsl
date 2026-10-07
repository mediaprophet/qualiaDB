@group(0) @binding(0) var source_level: texture_2d<f32>;
struct MipParams {
    values: vec4<f32>,
};
@group(0) @binding(1) var<uniform> mip_params: MipParams;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOut;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

fn source_area(position: vec2<u32>) -> vec4<f32> {
    let source_size = textureDimensions(source_level);
    let destination_size = max(source_size / 2u, vec2<u32>(1u));
    let scale = vec2<f32>(source_size) / vec2<f32>(destination_size);
    let region_begin = vec2<f32>(position) * scale;
    let region_end = vec2<f32>(position + vec2<u32>(1u)) * scale;
    let begin = vec2<u32>(floor(region_begin));
    let end = vec2<u32>(ceil(region_end));
    var total = vec4<f32>(0.0);
    var total_weight = 0.0;
    for (var y = begin.y; y < end.y; y += 1u) {
        for (var x = begin.x; x < end.x; x += 1u) {
            let overlap_x = max(0.0, min(region_end.x, f32(x + 1u)) - max(region_begin.x, f32(x)));
            let overlap_y = max(0.0, min(region_end.y, f32(y + 1u)) - max(region_begin.y, f32(y)));
            let weight = overlap_x * overlap_y;
            total += textureLoad(source_level, vec2<i32>(i32(x), i32(y)), 0) * weight;
            total_weight += weight;
        }
    }
    return total / total_weight;
}

@fragment
fn fragment_color(input: VertexOut) -> @location(0) vec4<f32> {
    var result = source_area(vec2<u32>(input.position.xy));
    // The 1/255 headroom mirrors the CPU coverage planner and protects threshold texels from
    // backend-specific UNORM render-target rounding (observed one to three codes low on GLES).
    result.a = clamp(result.a * mip_params.values.x + mip_params.values.y, 0.0, 1.0);
    return result;
}

@fragment
fn fragment_linear_data(input: VertexOut) -> @location(0) vec4<f32> {
    return source_area(vec2<u32>(input.position.xy));
}

@fragment
fn fragment_normal(input: VertexOut) -> @location(0) vec4<f32> {
    let sample = source_area(vec2<u32>(input.position.xy));
    let normal = sample.xyz * 2.0 - vec3<f32>(1.0);
    let length_squared = dot(normal, normal);
    let normalized = select(
        vec3<f32>(0.0, 0.0, 1.0),
        normal * inverseSqrt(max(length_squared, 1e-12)),
        length_squared > 1e-12,
    );
    return vec4<f32>(normalized * 0.5 + vec3<f32>(0.5), sample.a);
}
