// Tensor SOA projector — 10D structural nodes with depth (Track C phenomenal viewport).
//
// Object space: P' = Ω P Ω̃  (3D PGA sandwich) · clip = camera.view_projection · P'
// Phase 2b: dual-quaternion motor — d=0 regresses to Phase 1 quaternion path
// Phase 2c: bilateral T_pull via motor d channel (tensor.mu = deontic lane)
// Phase 3: v-band topology (cyclic / hyperbolic / boundary clique anchor)
//
// @group(0) camera + observer · @group(1) tensor SOA (offset past 32 B header)

struct Camera {
    view_projection: mat4x4<f32>,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    tensor_mode: u32,
    // [0] = frame time (seconds); [1..4] = camera eye xyz for T_pull
    _padding: array<f32, 12>,
};

// Matches portal_telemetry::ObserverStandpoint (128 B). u64 fields as vec2<u32> LE.
struct ObserverStandpoint {
    standpoint_hash: vec2<u32>,
    session_nonce: vec2<u32>,
    epistemic_q: f32,
    t_slice: f32,
    t_window: f32,
    deontic_lane: u32,
    standpoint_class: u32,
    fabric_gate: u32,
    _padding: array<f32, 22>,
};

// Matches Tensor10D SOA stride (40 B).
struct Tensor10D {
    q: f32,
    v: f32,
    w: f32,
    x: f32,
    y: f32,
    z: f32,
    t: f32,
    alpha: f32,
    mu: f32,
    sigma: f32,
};

// 3D PGA motor — rotation (r) + translation (d). Phase 1 uses r only.
struct Motor {
    r: vec4<f32>, // scalar + e12, e13, e23
    d: vec4<f32>, // e0123 + e01, e02, e03 (identity in Phase 1)
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> observer: ObserverStandpoint;
@group(1) @binding(0) var<storage, read> tensors: array<Tensor10D>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) epistemic_q: f32,
    @location(3) v_band: f32,
    @location(4) alpha_gain: f32,
    @location(5) pick_id: u32,
};

const TWO_PI: f32 = 6.283185307;
const MANIFOLD_COUNT: f32 = 5.0;
const Q_COLLAPSED_EPS: f32 = 0.001;

// Human-Centric observer standpoint classes (portal_telemetry.rs)
const STANDPOINT_SPECTATOR: u32 = 0u;
const STANDPOINT_EPHEMERAL: u32 = 1u;
const STANDPOINT_IDENTIFIER: u32 = 2u;
const STANDPOINT_VAULT: u32 = 3u;

const DEONTIC_LANE_BILATERAL: u32 = 2u;
const T_PULL_GAIN: f32 = 0.12;
const CLUSTER_COUNT: u32 = 8u;
const T_RADIAL_GAIN: f32 = 0.06;
const ANCHOR_RING_RADIUS: f32 = 0.35;

// σ → linear sRGB via spectral.wgsl (prepended by mod.rs)

// ── Cl(3,0) rotor as vec4(s, e12, e13, e23) ───────────────────────────────

fn rotor_identity() -> vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 0.0);
}

fn motor_identity() -> Motor {
    return Motor(rotor_identity(), vec4<f32>(0.0));
}

fn rotor_from_axis_angle(axis: vec3<f32>, angle: f32) -> vec4<f32> {
    let half = angle * 0.5;
    let c = cos(half);
    let s = sin(half);
    // R = cos(θ/2) + sin(θ/2)(ax·e23 - ay·e13 + az·e12)  (right-handed Cl(3,0))
    return vec4<f32>(c, s * (-axis.z), s * axis.y, s * (-axis.x));
}

fn rotor_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.x * b.x - a.y * b.y - a.z * b.z - a.w * b.w,
        a.x * b.y + a.y * b.x + a.z * b.w - a.w * b.z,
        a.x * b.z - a.y * b.w + a.z * b.x + a.w * b.y,
        a.x * b.w + a.y * b.z - a.z * b.y + a.w * b.x
    );
}

fn rotor_reverse(r: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(r.x, -r.y, -r.z, -r.w);
}

// Map Cl(3,0) rotor → quaternion (w, x, y, z) for sandwich on grade-1 vectors.
fn rotor_to_quat(r: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(r.x, -r.w, -r.z, -r.y);
}

fn quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.x * b.x - a.y * b.y - a.z * b.z - a.w * b.w,
        a.x * b.y + a.y * b.x + a.z * b.w - a.w * b.z,
        a.x * b.z - a.y * b.w + a.z * b.x + a.w * b.y,
        a.x * b.w + a.y * b.z - a.z * b.y + a.w * b.x
    );
}

fn quat_to_blade4(q: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(q.x, -q.w, -q.z, -q.y);
}

fn quat_conj(q: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(q.x, -q.y, -q.z, -q.w);
}

fn quat_add(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return a + b;
}

// PGA reversion: flip bivector signs; scalar + pseudoscalar stay positive.
fn motor_reverse(m: Motor) -> Motor {
    return Motor(
        vec4<f32>(m.r.x, -m.r.y, -m.r.z, -m.r.w),
        vec4<f32>(m.d.x, -m.d.y, -m.d.z, -m.d.w)
    );
}

// Dual-quaternion product (qr1, qd1) ⊗ (qr2, qd2).
fn motor_mul(a: Motor, b: Motor) -> Motor {
    let qr1 = rotor_to_quat(a.r);
    let qd1 = rotor_to_quat(a.d);
    let qr2 = rotor_to_quat(b.r);
    let qd2 = rotor_to_quat(b.d);
    let qr3 = quat_mul(qr1, qr2);
    let qd3 = quat_add(quat_mul(qr1, qd2), quat_mul(qd1, qr2));
    return Motor(quat_to_blade4(qr3), quat_to_blade4(qd3));
}

fn motor_translate(v: vec3<f32>) -> Motor {
    let half = vec4<f32>(0.0, v.x * 0.5, v.y * 0.5, v.z * 0.5);
    return Motor(vec4<f32>(1.0, 0.0, 0.0, 0.0), quat_to_blade4(half));
}

fn tensor_deontic_lane(mu: f32) -> u32 {
    return u32(round(mu));
}

fn bilateral_pull_active(tensor_mu: f32, standpoint_class: u32) -> bool {
    return tensor_deontic_lane(tensor_mu) == DEONTIC_LANE_BILATERAL
        && standpoint_class >= STANDPOINT_IDENTIFIER;
}

fn pull_vector(node: vec3<f32>, camera_eye: vec3<f32>, alpha: f32, epistemic_q: f32) -> vec3<f32> {
    let dir = camera_eye - node;
    let len = length(dir);
    if (len < 1e-6) {
        return vec3<f32>(0.0);
    }
    let gain = clamp(alpha, 0.2, 1.0);
    let delta = T_PULL_GAIN * gain * clamp(epistemic_q, 0.0, 1.0);
    return (dir / len) * delta;
}

// PGA null point P = e0 + x·e1 + y·e2 + z·e3 — P' = r P r̃ + 2(d r̃) vector part.
// When d = 0, ε terms vanish → exact Phase 1 quaternion sandwich.
fn sandwich_point(m: Motor, p: vec3<f32>) -> vec3<f32> {
    let qr = rotor_to_quat(m.r);
    let qd = rotor_to_quat(m.d);
    let qr_conj = quat_conj(qr);
    let p_q = vec4<f32>(0.0, p.x, p.y, p.z);
    let p_rot = quat_mul(quat_mul(qr, p_q), qr_conj);
    let t_q = quat_mul(qd, qr_conj);
    const T_SCALE: f32 = 2.0;
    return vec3<f32>(
        p_rot.y + T_SCALE * t_q.y,
        p_rot.z + T_SCALE * t_q.z,
        p_rot.w + T_SCALE * t_q.w
    );
}

fn cluster_id_from_sigma(sigma: f32) -> u32 {
    let frac = fract(sigma);
    return u32(floor(frac * f32(CLUSTER_COUNT))) % CLUSTER_COUNT;
}

fn cluster_centroid_lattice(cluster_id: u32) -> vec3<f32> {
    let k = cluster_id % CLUSTER_COUNT;
    let angle = f32(k) * TWO_PI / f32(CLUSTER_COUNT);
    return vec3<f32>(ANCHOR_RING_RADIUS * cos(angle), 0.0, ANCHOR_RING_RADIUS * sin(angle));
}

// Phase 3 v-band: [0,1) Euclidean · [1,2) cyclic · [2,3) hyperbolic · [3,∞) boundary clique.
fn motor_v_band(v: f32, node: vec3<f32>, sigma: f32, time: f32, alpha: f32) -> Motor {
    let gain = clamp(alpha, 0.2, 1.0);
    if (v < 1.0) {
        return motor_identity();
    }
    if (v < 2.0) {
        let band = v - 1.0;
        let theta = band * TWO_PI * sin(time * 0.5 + sigma) * gain;
        return Motor(rotor_from_axis_angle(vec3<f32>(0.0, 1.0, 0.0), theta), vec4<f32>(0.0));
    }
    if (v < 3.0) {
        let band = v - 2.0;
        let len = max(length(node), 1e-4);
        let dir = node / len;
        let delta = T_RADIAL_GAIN * band * gain;
        return motor_translate(dir * delta);
    }
    let centroid = cluster_centroid_lattice(cluster_id_from_sigma(sigma));
    let blend = min(v - 3.0, 1.0) * gain;
    return motor_translate((centroid - node) * blend);
}

fn motor_rw(w: f32, alpha: f32) -> vec4<f32> {
    let theta_w = w * (TWO_PI / MANIFOLD_COUNT);
    let gain = clamp(alpha, 0.2, 1.0);
    return rotor_from_axis_angle(vec3<f32>(0.0, 1.0, 0.0), theta_w * gain);
}

// Phase 2a: Human-Centric standpoint gates on epistemic spin (quaternion slice retained).
// Vault → freeze R_q. Identifier (DID) → dampen θ_q by observer.epistemic_q (certainty aperture).
fn motor_rq(q: f32, sigma: f32, time: f32, alpha: f32, obs: ObserverStandpoint) -> vec4<f32> {
    if (obs.standpoint_class == STANDPOINT_VAULT) {
        return rotor_identity();
    }
    if (q <= Q_COLLAPSED_EPS) {
        return rotor_identity();
    }
    let gain = clamp(alpha, 0.2, 1.0);
    var theta_q = q * sin(time * 2.0 + sigma * TWO_PI) * gain;
    if (obs.standpoint_class == STANDPOINT_IDENTIFIER) {
        // Dampen spin amplitude — lower epistemic_q = slower, tighter orbital bound.
        theta_q = theta_q * clamp(obs.epistemic_q, 0.0, 1.0);
    }
    let ax = cos(sigma * TWO_PI);
    let az = sin(sigma * TWO_PI);
    let len = max(sqrt(ax * ax + az * az), 1e-4);
    let axis = vec3<f32>(ax / len, 0.0, az / len);
    return rotor_from_axis_angle(axis, theta_q);
}

fn semantic_motor_intrinsic(tensor: Tensor10D, time: f32, obs: ObserverStandpoint, node: vec3<f32>) -> Motor {
    let r_v = motor_v_band(tensor.v, node, tensor.sigma, time, tensor.alpha);
    let r_w = motor_rw(tensor.w, tensor.alpha);
    let r_q = motor_rq(tensor.q, tensor.sigma, time, tensor.alpha, obs);
    return motor_mul(Motor(r_w, vec4<f32>(0.0)), motor_mul(Motor(r_q, vec4<f32>(0.0)), r_v));
}

// Phase 2c: Ω = T_pull · (R_w · R_q) — subjective bilateral bias after intrinsic motors.
fn semantic_motor(tensor: Tensor10D, time: f32, obs: ObserverStandpoint, node: vec3<f32>, camera_eye: vec3<f32>) -> Motor {
    let r_intrinsic = semantic_motor_intrinsic(tensor, time, obs, node);
    var t_motor = motor_identity();
    if (bilateral_pull_active(tensor.mu, obs.standpoint_class)) {
        t_motor = motor_translate(pull_vector(node, camera_eye, tensor.alpha, obs.epistemic_q));
    }
    return motor_mul(t_motor, r_intrinsic);
}

fn apply_semantic_motor(local: vec3<f32>, tensor: Tensor10D, time: f32, obs: ObserverStandpoint, camera_eye: vec3<f32>) -> vec3<f32> {
    let omega = semantic_motor(tensor, time, obs, local, camera_eye);
    return sandwich_point(omega, local);
}

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );

    let tensor = tensors[instance_index];
    let frame_time = camera._padding[0];
    let camera_eye = vec3<f32>(camera._padding[1], camera._padding[2], camera._padding[3]);

    // Temporal scrub — discard vertices outside the observer's t_window band.
    let temporal_delta = abs(tensor.t - observer.t_slice);
    let outside_time = temporal_delta > observer.t_window;

    let local = vec3<f32>(tensor.x, tensor.y, tensor.z);
    let world_pos = apply_semantic_motor(local, tensor, frame_time, observer, camera_eye);
    let clip = camera.view_projection * vec4<f32>(world_pos, 1.0);

    let base_vertex = quad_vertices[vertex_index];
    let point_scale = 0.012 * (0.65 + tensor.alpha * 0.55);

    var output: VertexOutput;
    output.v_band = tensor.v;
    output.alpha_gain = tensor.alpha;
    if (outside_time) {
        output.clip_position = vec4<f32>(0.0, 0.0, 2.0, 1.0);
    } else {
        output.clip_position = vec4<f32>(
            clip.x + base_vertex.x * point_scale * clip.w,
            clip.y + base_vertex.y * point_scale * clip.w,
            clip.z,
            clip.w
        );
    }
    output.local_uv = base_vertex;
    let rgb = sigma_to_linear_rgb(tensor.sigma);
    let alpha = clamp(0.4 + tensor.alpha * 0.55, 0.25, 1.0);
    output.color = vec4<f32>(rgb, alpha);
    output.epistemic_q = tensor.q;
    output.pick_id = instance_index;
    return output;
}

// PR-C11: R32Uint picking pass — outputs tensor SOA index per pixel.
@fragment
fn picking_fragment_main(input: VertexOutput) -> @location(0) u32 {
    let dist = length(input.local_uv);
    if (dist > 1.0) {
        discard;
    }
    return input.pick_id;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(input.local_uv);
    let alpha = smoothstep(1.0, 0.0, dist);
    let collapsed = step(input.epistemic_q, 0.001);
    let sandbox = 1.0 - collapsed;
    let certainty_opacity = mix(0.35, 1.0, collapsed);
    let sandbox_pulse = 0.8 + 0.2 * sin(input.epistemic_q * 14.0);
    let epistemic_alpha = mix(certainty_opacity * sandbox_pulse, certainty_opacity, collapsed);
    let ring_boost = select(0.0, 0.4 * smoothstep(0.75, 1.0, dist), sandbox > 0.5);
    // HDR σ→CIE luminance: α gain + boundary-clique density boost for bloom threshold.
    var hdr_gain = (0.75 + input.alpha_gain * 1.1) * (1.0 + ring_boost);
    if (input.v_band >= 3.0) {
        hdr_gain = hdr_gain * (1.0 + 0.45 * min(input.v_band - 3.0, 1.0));
    }
    let rgb = input.color.rgb * hdr_gain;
    return vec4<f32>(rgb, input.color.a * alpha * epistemic_alpha);
}