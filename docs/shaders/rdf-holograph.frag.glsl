#version 300 es
precision highp float;
uniform sampler2D u_nodes;
uniform vec3 u_eye;
uniform float u_time;
uniform int u_selected;
uniform float u_w_angle;
in vec3 v_world;
flat in int v_index;
out vec4 out_color;

vec3 hue_rgb(float h) {
    float x = 1.0 - abs(mod(h * 6.0, 2.0) - 1.0);
    if (h < 1.0 / 6.0) return vec3(1.0, x, 0.0);
    if (h < 2.0 / 6.0) return vec3(x, 1.0, 0.0);
    if (h < 3.0 / 6.0) return vec3(0.0, 1.0, x);
    if (h < 4.0 / 6.0) return vec3(0.0, x, 1.0);
    if (h < 5.0 / 6.0) return vec3(x, 0.0, 1.0);
    return vec3(1.0, 0.0, x);
}

float sdf_sphere(vec3 p, float r) { return length(p) - r; }

float sdf_octahedron(vec3 p, float s) {
    p = abs(p);
    return (p.x + p.y + p.z - s) * 0.57735027;
}

float sdf_diamond(vec3 p, float s) {
    p = abs(p);
    p.y *= 0.70;
    return sdf_octahedron(p, s);
}

float sdf_shard(vec3 p, float s) {
    vec3 q = p;
    q.y *= 2.15;
    q.z *= 1.65;
    return sdf_octahedron(q, s);
}

float sdf_capsule(vec3 p, vec3 a, vec3 b, float r) {
    vec3 pa = p - a;
    vec3 ba = b - a;
    float h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-5), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

float sdf_rounded_box(vec3 p, vec3 b, float r) {
    vec3 q = abs(p) - (b - vec3(r));
    return length(max(q, vec3(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

void main() {
    vec4 pos_scale = texelFetch(u_nodes, ivec2(0, v_index), 0);
    vec4 half_prim = texelFetch(u_nodes, ivec2(1, v_index), 0);
    vec4 freq_amp = texelFetch(u_nodes, ivec2(2, v_index), 0);
    vec4 manifold = texelFetch(u_nodes, ivec2(3, v_index), 0);
    vec4 endpoint = texelFetch(u_nodes, ivec2(4, v_index), 0);
    vec4 emissive = texelFetch(u_nodes, ivec2(5, v_index), 0);

    float c = cos(u_w_angle);
    float s = sin(u_w_angle);
    vec3 center = vec3(pos_scale.x * c - pos_scale.z * s, pos_scale.y, pos_scale.x * s + pos_scale.z * c);
    vec3 endp = vec3(endpoint.x * c - endpoint.z * s, endpoint.y, endpoint.x * s + endpoint.z * c);
    vec3 rd = normalize(v_world - u_eye);
    vec3 ro = u_eye - center;

    int kind = int(manifold.y + 0.5);
    int prim = int(half_prim.w + 0.5);
    float grow = 0.90 + 0.10 * smoothstep(0.0, 1.15, u_time - float(v_index) * 0.040);
    float breath = 1.0 + 0.045 * sin(u_time * freq_amp.x * 0.55 + freq_amp.z);
    float scale = max(pos_scale.w, 0.28) * grow * breath;

    float t_hit = length(v_world - u_eye);
    float t = max(t_hit - 1.25, 0.0);
    float d = 1.0;
    float closest = 8.0;
    float along = 0.0;
    for (int i = 0; i < 48; i++) {
        vec3 p = ro + rd * t;
        if (kind == 5) {
            d = sdf_sphere(p, scale * 0.92);
        } else if (kind == 3 || kind == 4 || prim == 4) {
            float rad = (kind == 3 ? 0.085 : 0.052) * (0.75 + 0.35 * pos_scale.w);
            vec3 halfv = endp - center;
            d = sdf_capsule(p, -halfv, halfv, rad);
            along = clamp(dot(p, halfv) / max(dot(halfv, halfv), 1e-5) * 0.5 + 0.5, 0.0, 1.0);
        } else if (kind == 1) {
            float spin = u_time * 0.11 + freq_amp.z;
            float cs = cos(spin);
            float ss = sin(spin);
            vec3 q = vec3(p.x * cs - p.z * ss, p.y, p.x * ss + p.z * ss);
            d = sdf_diamond(q, scale * 1.05);
        } else if (kind == 2) {
            d = sdf_shard(p, scale * 0.95);
        } else if (prim == 1) {
            d = sdf_rounded_box(p, half_prim.xyz * grow * breath, 0.10);
        } else {
            d = sdf_octahedron(p, scale * 0.90);
        }
        closest = min(closest, d);
        if (d < 0.0016) break;
        t += max(d, 0.003);
        if (t > t_hit + 8.0) break;
    }

    vec3 void_col = vec3(0.008, 0.016, 0.038);
    float selected = float(v_index == u_selected);
    float dim = mix(1.0, 0.28, float(u_selected >= 0) * (1.0 - selected));
    vec3 base = max(emissive.rgb, vec3(0.18, 0.45, 0.85));

    if (kind == 5) {
        float well = exp(-max(closest, 0.0) * 2.35) * grow * dim;
        if (well < 0.035) discard;
        vec3 glow = vec3(0.08, 0.14, 0.40) * (0.18 + well * 0.55);
        float fog = exp(-0.018 * t);
        out_color = vec4(mix(void_col, glow, fog), clamp(0.012 + well * 0.055, 0.012, 0.045));
        return;
    }

    if (d > 0.024) {
        float halo = exp(-max(closest, 0.0) * mix(8.4, 6.2, float(kind >= 3))) * grow;
        if (kind >= 3) halo *= 0.50;
        halo *= dim;
        if (halo < 0.045) discard;
        vec3 glow = base * (0.22 + halo * 0.80) * (1.0 + selected * 0.35);
        if (kind == 3) glow = mix(glow, vec3(1.0, 0.84, 0.32), 0.62);
        float fog = exp(-0.028 * t);
        out_color = vec4(mix(void_col, glow, fog), clamp(0.035 + halo * 0.18, 0.03, 0.16));
        return;
    }

    vec3 hit = u_eye + rd * t;
    vec3 n = normalize(hit - center);
    if (kind == 1 || kind == 2) {
        vec3 e = vec3(0.004, 0.0, 0.0);
        vec3 p = hit - center;
        float spin = u_time * 0.11 + freq_amp.z;
        float cs = cos(spin);
        float ss = sin(spin);
        vec3 q = vec3(p.x * cs - p.z * ss, p.y, p.x * ss + p.z * ss);
        float s0 = kind == 1 ? sdf_diamond(q, scale * 1.05) : sdf_shard(p, scale * 0.95);
        n = normalize(vec3(
            (kind == 1 ? sdf_diamond(q + e.xyy, scale * 1.05) : sdf_shard(p + e.xyy, scale * 0.95)) - s0,
            (kind == 1 ? sdf_diamond(q + e.yxy, scale * 1.05) : sdf_shard(p + e.yxy, scale * 0.95)) - s0,
            (kind == 1 ? sdf_diamond(q + e.yyx, scale * 1.05) : sdf_shard(p + e.yyx, scale * 0.95)) - s0
        ));
    }

    float fresnel = pow(1.0 - abs(dot(n, -rd)), 2.6);
    float pulse = 0.78 + 0.22 * sin(u_time * freq_amp.x * 0.7 + freq_amp.z);
    vec3 core = base * (0.42 + 0.28 * pulse);
    vec3 prism = hue_rgb(fract(dot(n, vec3(0.18, 0.62, 0.31)) + u_time * 0.03 + manifold.x * 0.05));
    vec3 inner = mix(core, core * vec3(0.62, 0.78, 1.12), 0.38);
    vec3 col = inner + prism * fresnel * 0.42 + base * fresnel * 0.38;

    if (kind >= 3) {
        float travel = smoothstep(0.07, 0.0, abs(fract(along - u_time * 0.22) - 0.5) - 0.08);
        col = base * (0.42 + travel * 1.15 + fresnel * 0.55);
        if (kind == 3) col = mix(col, vec3(1.0, 0.84, 0.30), 0.58 + travel * 0.28);
    }

    if (kind == 1) {
        float facet = pow(max(max(abs(n.x), abs(n.y)), abs(n.z)), 8.0);
        col = mix(vec3(0.16, 0.28, 0.62), base, 0.42);
        col += vec3(0.86, 0.93, 1.0) * facet * 0.95;
        col += prism * fresnel * 0.35;
        col += vec3(0.12, 0.20, 0.48) * (1.0 - fresnel) * 0.40;
    }
    if (kind == 0) {
        col = mix(col, base * 0.82, 0.22);
        col += vec3(0.10, 0.42, 0.68) * pow(fresnel, 1.6) * 0.55;
    }
    if (kind == 2) {
        col = mix(col, vec3(0.92, 0.68, 0.22), 0.38);
        col += vec3(1.0, 0.90, 0.52) * fresnel * 0.45;
    }

    col *= dim;
    col = mix(col, vec3(1.0, 0.90, 0.48) * 1.15, selected * 0.40);
    col = min(col, vec3(1.05));
    float fog = exp(-0.022 * t);
    col = mix(void_col, col, fog);
    float alpha = clamp(0.34 + fresnel * 0.28 + selected * 0.12, 0.22, 0.62);
    if (kind >= 3) alpha = clamp(0.18 + fresnel * 0.22, 0.12, 0.42);
    out_color = vec4(col, alpha);
}
