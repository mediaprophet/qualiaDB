//! Standalone WebGL2 shaders so native tests can validate them without a browser.

pub(super) const OUTPUT_VERTEX: &str = r#"#version 300 es
precision highp float;
out vec2 v_uv;
void main() {
    vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    v_uv = p;
}
"#;

pub(super) const OUTPUT_FRAGMENT: &str = r#"#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_scene;
uniform float u_exposure;
uniform vec3 u_white_balance_gains;
out vec4 out_color;
vec3 pbr_neutral_v1(vec3 input_color) {
    vec3 color = max(input_color, vec3(0.0));
    float min_channel = min(color.r, min(color.g, color.b));
    float offset = min_channel < 0.08
        ? min_channel - 6.25 * min_channel * min_channel
        : 0.04;
    color -= vec3(offset);
    float peak = max(color.r, max(color.g, color.b));
    float start_compression = 0.8 - 0.04;
    if (peak < start_compression) return color;
    float d = 1.0 - start_compression;
    float new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    float gray_mix = 1.0 - 1.0 / (0.15 * (peak - new_peak) + 1.0);
    return mix(color, vec3(new_peak), gray_mix);
}
vec3 linear_to_srgb(vec3 linear_color) {
    vec3 low = linear_color * 12.92;
    vec3 high = 1.055 * pow(linear_color, vec3(1.0 / 2.4)) - vec3(0.055);
    return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}
void main() {
    vec3 scene_linear = texture(u_scene, v_uv).rgb * u_exposure * u_white_balance_gains;
    out_color = vec4(linear_to_srgb(pbr_neutral_v1(scene_linear)), 1.0);
}
"#;
