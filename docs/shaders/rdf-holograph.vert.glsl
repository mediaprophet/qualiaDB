#version 300 es
precision highp float;
uniform mat4 u_view_projection;
uniform sampler2D u_nodes;
uniform float u_w_angle;
uniform float u_time;
out vec3 v_world;
flat out int v_index;

const vec3 CUBE[8] = vec3[8](
    vec3(-1.0, -1.0, -1.0),
    vec3( 1.0, -1.0, -1.0),
    vec3( 1.0,  1.0, -1.0),
    vec3(-1.0,  1.0, -1.0),
    vec3(-1.0, -1.0,  1.0),
    vec3( 1.0, -1.0,  1.0),
    vec3( 1.0,  1.0,  1.0),
    vec3(-1.0,  1.0,  1.0)
);
const int IDX[36] = int[36](
    0, 1, 2, 0, 2, 3,
    4, 6, 5, 4, 7, 6,
    0, 4, 5, 0, 5, 1,
    3, 2, 6, 3, 6, 7,
    1, 5, 6, 1, 6, 2,
    3, 7, 4, 3, 4, 0
);

void main() {
    int inst = gl_InstanceID;
    vec4 pos_scale = texelFetch(u_nodes, ivec2(0, inst), 0);
    vec4 half_prim = texelFetch(u_nodes, ivec2(1, inst), 0);
    vec4 freq_amp = texelFetch(u_nodes, ivec2(2, inst), 0);
    vec4 manifold = texelFetch(u_nodes, ivec2(3, inst), 0);
    int kind = int(manifold.y + 0.5);
    vec3 corner = CUBE[IDX[gl_VertexID]];
    float grow = 0.90 + 0.10 * smoothstep(0.0, 1.15, u_time - float(inst) * 0.040);
    float breath = 1.0 + 0.05 * sin(u_time * freq_amp.x * 0.55 + freq_amp.z);
    float crystal = max(pos_scale.w, 0.28) * grow * breath;
    float pad = 0.42;
    vec3 extents;
    if (kind == 5) {
        extents = vec3(crystal * 1.08);
    } else if (kind >= 3) {
        extents = (half_prim.xyz + vec3(pad * 0.55)) * grow * breath;
    } else if (kind == 1) {
        extents = vec3(crystal * 1.05 + pad, crystal * 1.50 + pad, crystal * 1.05 + pad);
    } else if (kind == 2) {
        float ss = crystal * 0.95;
        extents = vec3(ss + pad, ss / 2.15 + pad, ss / 1.65 + pad);
    } else {
        extents = vec3(crystal * 0.90 + pad);
    }
    vec3 p = pos_scale.xyz;
    float c = cos(u_w_angle);
    float s = sin(u_w_angle);
    vec3 rotated = vec3(p.x * c - p.z * s, p.y, p.x * s + p.z * c);
    v_world = rotated + corner * extents;
    v_index = inst;
    gl_Position = u_view_projection * vec4(v_world, 1.0);
}
