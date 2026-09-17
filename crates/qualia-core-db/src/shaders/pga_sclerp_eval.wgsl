// PGA ScLERP Batch Compute Kernel (3D Projective Geometric Algebra)
// Evaluates parallel Screw Linear Interpolation for N instances across SE(3) geodesics.

struct Motor {
    r_w: f32,
    r_x: f32,
    r_y: f32,
    r_z: f32,
    d_w: f32,
    d_x: f32,
    d_y: f32,
    d_z: f32,
};

struct ScLerpParams {
    count: u32,
    t: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> params: ScLerpParams;
@group(0) @binding(1) var<storage, read> motors_start: array<Motor>;
@group(0) @binding(2) var<storage, read> motors_end: array<Motor>;
@group(0) @binding(3) var<storage, read_write> motors_out: array<Motor>;

fn motor_conjugate(m: Motor) -> Motor {
    return Motor(m.r_w, -m.r_x, -m.r_y, -m.r_z, m.d_w, -m.d_x, -m.d_y, -m.d_z);
}

fn motor_mul(a: Motor, b: Motor) -> Motor {
    let rw = a.r_w * b.r_w - a.r_x * b.r_x - a.r_y * b.r_y - a.r_z * b.r_z;
    let rx = a.r_w * b.r_x + a.r_x * b.r_w + a.r_y * b.r_z - a.r_z * b.r_y;
    let ry = a.r_w * b.r_y - a.r_x * b.r_z + a.r_y * b.r_w + a.r_z * b.r_x;
    let rz = a.r_w * b.r_z + a.r_x * b.r_y - a.r_y * b.r_x + a.r_z * b.r_w;

    let dw = a.r_w * b.d_w - a.r_x * b.d_x - a.r_y * b.d_y - a.r_z * b.d_z
           + a.d_w * b.r_w - a.d_x * b.r_x - a.d_y * b.r_y - a.d_z * b.r_z;
    let dx = a.r_w * b.d_x + a.r_x * b.d_w + a.r_y * b.d_z - a.r_z * b.d_y
           + a.d_w * b.r_x + a.d_x * b.r_w + a.d_y * b.r_z - a.d_z * b.r_y;
    let dy = a.r_w * b.d_y - a.r_x * b.d_z + a.r_y * b.d_w + a.r_z * b.d_x
           + a.d_w * b.r_y - a.d_x * b.r_z + a.d_y * b.r_w + a.d_z * b.r_x;
    let dz = a.r_w * b.d_z + a.r_x * b.d_y - a.r_y * b.d_x + a.r_z * b.d_w
           + a.d_w * b.r_z + a.d_x * b.r_y - a.d_y * b.r_x + a.d_z * b.r_w;

    return Motor(rw, rx, ry, rz, dw, dx, dy, dz);
}

@compute @workgroup_size(64)
fn pga_sclerp_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.count) {
        return;
    }

    let m0 = motors_start[idx];
    let m1 = motors_end[idx];
    let t = params.t;

    let m0_inv = motor_conjugate(m0);
    var delta = motor_mul(m0_inv, m1);

    if (delta.r_w < 0.0) {
        delta = Motor(-delta.r_w, -delta.r_x, -delta.r_y, -delta.r_z,
                      -delta.d_w, -delta.d_x, -delta.d_y, -delta.d_z);
    }

    // Logarithm -> Scale by t -> Exponential
    let sin_sq = delta.r_x * delta.r_x + delta.r_y * delta.r_y + delta.r_z * delta.r_z;
    var res_motor: Motor;

    if (sin_sq < 1e-8) {
        // Pure translation limit
        let scaled_t = Motor(1.0, 0.0, 0.0, 0.0,
                             0.0, delta.d_x * t, delta.d_y * t, delta.d_z * t);
        res_motor = motor_mul(m0, scaled_t);
    } else {
        let sin_theta = sqrt(sin_sq);
        let theta = atan2(sin_theta, delta.r_w);
        let inv_sin = 1.0 / sin_theta;

        let pitch = -delta.d_w * inv_sin;
        let axis = vec3<f32>(delta.r_x * inv_sin, delta.r_y * inv_sin, delta.r_z * inv_sin);

        let scaled_theta = theta * t;
        let sin_scaled = sin(scaled_theta);
        let cos_scaled = cos(scaled_theta);

        let exp_rw = cos_scaled;
        let exp_rx = axis.x * sin_scaled;
        let exp_ry = axis.y * sin_scaled;
        let exp_rz = axis.z * sin_scaled;

        let d_trans = vec3<f32>(delta.d_x - pitch * delta.r_x,
                                delta.d_y - pitch * delta.r_y,
                                delta.d_z - pitch * delta.r_z) * t;

        let exp_motor = Motor(exp_rw, exp_rx, exp_ry, exp_rz,
                              0.0, d_trans.x, d_trans.y, d_trans.z);
        res_motor = motor_mul(m0, exp_motor);
    }

    motors_out[idx] = res_motor;
}
