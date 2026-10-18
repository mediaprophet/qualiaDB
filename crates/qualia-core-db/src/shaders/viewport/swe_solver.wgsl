// 2.5D Shallow Water Equations (SWE) Compute Kernel (`swe_solver.wgsl`)
//
// Solves non-linear depth-averaged Saint-Venant equations with:
// - Conservation of Mass: ∂h/∂t + ∂(hu)/∂x + ∂(hv)/∂y = Rain - Infiltration
// - Conservation of Momentum: ∂(hu)/∂t + ∂(hu² + 0.5gh²)/∂x + ∂(huv)/∂y = -gh(∂z_b/∂x) - τ_bx/ρ + ν∇²(hu)
// - Hydrostatic reconstruction (Kurganov-Petrova central-upwind flux) for well-balanced lake-at-rest state
// - Semi-implicit Manning friction regularization: τ_b = ρ * g * n² * |u| * u / h^(1/3)
// - Wet/dry interface desingularization with threshold ε_h = 1e-4 m

struct SweParams {
    grid_dim: vec2<u32>,        // Width, Height of grid
    dx: f32,                    // Spatial step in meters
    dt: f32,                    // Time step in seconds
    gravity: f32,               // 9.80665 m/s²
    manning_n: f32,             // Bed roughness coefficient (e.g. 0.035 for natural channel)
    rain_rate: f32,             // Net precipitation in m/s
    infiltration: f32,          // Soil infiltration loss in m/s
    viscosity: f32,             // Kinematic eddy viscosity m²/s
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> params: SweParams;
// State format: RGBA32F -> x: water depth h, y: momentum hu, z: momentum hv, w: passive tracer (CDOM/tannin)
@group(0) @binding(1) var state_curr: texture_2d<f32>;
// Bed elevation format: RG32F -> x: bed height z_b, y: local roughness multiplier
@group(0) @binding(2) var bed_topo: texture_2d<f32>;
// Output next state
@group(0) @binding(3) var state_next: texture_storage_2d<rgba32float, write>;

const EPSILON_H: f32 = 0.0001; // 0.1 mm wet-dry threshold

// Reads water state clamped to boundary
fn sample_state(coord: vec2<i32>) -> vec4<f32> {
    let clamped = clamp(coord, vec2<i32>(0), vec2<i32>(params.grid_dim) - vec2<i32>(1));
    return textureLoad(state_curr, clamped, 0);
}

// Reads bed topography clamped to boundary
fn sample_bed(coord: vec2<i32>) -> vec2<f32> {
    let clamped = clamp(coord, vec2<i32>(0), vec2<i32>(params.grid_dim) - vec2<i32>(1));
    return textureLoad(bed_topo, clamped, 0).xy;
}

// Computes 1D physical flux vector [F_h, F_hu, F_hv]
fn physical_flux_x(h: f32, hu: f32, hv: f32, g: f32) -> vec3<f32> {
    if (h <= EPSILON_H) {
        return vec3<f32>(0.0);
    }
    let u = hu / h;
    let v = hv / h;
    return vec3<f32>(
        hu,
        hu * u + 0.5 * g * h * h,
        hu * v
    );
}

fn physical_flux_y(h: f32, hu: f32, hv: f32, g: f32) -> vec3<f32> {
    if (h <= EPSILON_H) {
        return vec3<f32>(0.0);
    }
    let u = hu / h;
    let v = hv / h;
    return vec3<f32>(
        hv,
        hv * u,
        hv * v + 0.5 * g * h * h
    );
}

// Kurganov-Petrova central-upwind numerical flux with hydrostatic reconstruction
fn compute_interface_flux_x(
    h_L: f32, hu_L: f32, hv_L: f32, zb_L: f32,
    h_R: f32, hu_R: f32, hv_R: f32, zb_R: f32,
    g: f32
) -> vec3<f32> {
    // Hydrostatic reconstruction to maintain lake-at-rest stability
    let zb_edge = max(zb_L, zb_R);
    let h_L_star = max(0.0, h_L + zb_L - zb_edge);
    let h_R_star = max(0.0, h_R + zb_R - zb_edge);

    if (h_L_star <= EPSILON_H && h_R_star <= EPSILON_H) {
        return vec3<f32>(0.0);
    }

    let u_L = select(0.0, hu_L / max(h_L, EPSILON_H), h_L > EPSILON_H);
    let u_R = select(0.0, hu_R / max(h_R, EPSILON_H), h_R > EPSILON_H);

    let c_L = sqrt(g * h_L_star);
    let c_R = sqrt(g * h_R_star);

    // Wave propagation speeds
    let a_plus = max(0.0, max(u_L + c_L, u_R + c_R));
    let a_minus = min(0.0, min(u_L - c_L, u_R - c_R));
    let denom = a_plus - a_minus;

    if (denom <= 1e-6) {
        return vec3<f32>(0.0);
    }

    let hu_L_star = h_L_star * u_L;
    let hu_R_star = h_R_star * u_R;
    let hv_L_star = select(0.0, (h_L_star / max(h_L, EPSILON_H)) * hv_L, h_L > EPSILON_H);
    let hv_R_star = select(0.0, (h_R_star / max(h_R, EPSILON_H)) * hv_R, h_R > EPSILON_H);

    let F_L = physical_flux_x(h_L_star, hu_L_star, hv_L_star, g);
    let F_R = physical_flux_x(h_R_star, hu_R_star, hv_R_star, g);

    let U_L = vec3<f32>(h_L_star, hu_L_star, hv_L_star);
    let U_R = vec3<f32>(h_R_star, hu_R_star, hv_R_star);

    return (a_plus * F_L - a_minus * F_R + a_plus * a_minus * (U_R - U_L)) / denom;
}

fn compute_interface_flux_y(
    h_B: f32, hu_B: f32, hv_B: f32, zb_B: f32,
    h_T: f32, hu_T: f32, hv_T: f32, zb_T: f32,
    g: f32
) -> vec3<f32> {
    let zb_edge = max(zb_B, zb_T);
    let h_B_star = max(0.0, h_B + zb_B - zb_edge);
    let h_T_star = max(0.0, h_T + zb_T - zb_edge);

    if (h_B_star <= EPSILON_H && h_T_star <= EPSILON_H) {
        return vec3<f32>(0.0);
    }

    let v_B = select(0.0, hv_B / max(h_B, EPSILON_H), h_B > EPSILON_H);
    let v_T = select(0.0, hv_T / max(h_T, EPSILON_H), h_T > EPSILON_H);

    let c_B = sqrt(g * h_B_star);
    let c_T = sqrt(g * h_T_star);

    let a_plus = max(0.0, max(v_B + c_B, v_T + c_T));
    let a_minus = min(0.0, min(v_B - c_B, v_T - c_T));
    let denom = a_plus - a_minus;

    if (denom <= 1e-6) {
        return vec3<f32>(0.0);
    }

    let hu_B_star = select(0.0, (h_B_star / max(h_B, EPSILON_H)) * hu_B, h_B > EPSILON_H);
    let hu_T_star = select(0.0, (h_T_star / max(h_T, EPSILON_H)) * hu_T, h_T > EPSILON_H);
    let hv_B_star = h_B_star * v_B;
    let hv_T_star = h_T_star * v_T;

    let F_B = physical_flux_y(h_B_star, hu_B_star, hv_B_star, g);
    let F_T = physical_flux_y(h_T_star, hu_T_star, hv_T_star, g);

    let U_B = vec3<f32>(h_B_star, hu_B_star, hv_B_star);
    let U_T = vec3<f32>(h_T_star, hu_T_star, hv_T_star);

    return (a_plus * F_B - a_minus * F_T + a_plus * a_minus * (U_T - U_B)) / denom;
}

@compute @workgroup_size(16, 16)
fn swe_step_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    if (global_id.x >= params.grid_dim.x || global_id.y >= params.grid_dim.y) {
        return;
    }

    let coord = vec2<i32>(x, y);
    let center = sample_state(coord);
    let bed_c = sample_bed(coord);

    let h_C = center.x;
    let hu_C = center.y;
    let hv_C = center.z;
    let tracer_C = center.w;
    let zb_C = bed_c.x;

    // Neighboring cell states
    let state_W = sample_state(coord + vec2<i32>(-1, 0));
    let bed_W = sample_bed(coord + vec2<i32>(-1, 0));
    let state_E = sample_state(coord + vec2<i32>(1, 0));
    let bed_E = sample_bed(coord + vec2<i32>(1, 0));
    let state_S = sample_state(coord + vec2<i32>(0, -1));
    let bed_S = sample_bed(coord + vec2<i32>(0, -1));
    let state_N = sample_state(coord + vec2<i32>(0, 1));
    let bed_N = sample_bed(coord + vec2<i32>(0, 1));

    let g = params.gravity;
    let dt = params.dt;
    let dx = params.dx;
    let inv_dx = 1.0 / dx;

    // Numerical interface fluxes
    let flux_west = compute_interface_flux_x(
        state_W.x, state_W.y, state_W.z, bed_W.x,
        h_C, hu_C, hv_C, zb_C, g
    );
    let flux_east = compute_interface_flux_x(
        h_C, hu_C, hv_C, zb_C,
        state_E.x, state_E.y, state_E.z, bed_E.x, g
    );
    let flux_south = compute_interface_flux_y(
        state_S.x, state_S.y, state_S.z, bed_S.x,
        h_C, hu_C, hv_C, zb_C, g
    );
    let flux_north = compute_interface_flux_y(
        h_C, hu_C, hv_C, zb_C,
        state_N.x, state_N.y, state_N.z, bed_N.x, g
    );

    // Bed slope source term via centered difference
    let dzb_dx = (bed_E.x - bed_W.x) * (0.5 * inv_dx);
    let dzb_dy = (bed_N.x - bed_S.x) * (0.5 * inv_dx);
    let source_slope_x = -g * h_C * dzb_dx;
    let source_slope_y = -g * h_C * dzb_dy;

    // Kinematic eddy diffusion laplacian: ν * ∇²(hu)
    let laplacian_hu = (state_W.y + state_E.y + state_S.y + state_N.y - 4.0 * hu_C) * (inv_dx * inv_dx);
    let laplacian_hv = (state_W.z + state_E.z + state_S.z + state_N.z - 4.0 * hv_C) * (inv_dx * inv_dx);

    // Explicit update for depth and raw intermediate momentum
    let div_flux_h = (flux_east.x - flux_west.x + flux_north.x - flux_south.x) * inv_dx;
    let net_source_mass = params.rain_rate - params.infiltration;
    var h_new = max(0.0, h_C - dt * div_flux_h + dt * net_source_mass);

    var hu_raw = hu_C - dt * ((flux_east.y - flux_west.y + flux_north.y - flux_south.y) * inv_dx)
        + dt * source_slope_x + dt * params.viscosity * laplacian_hu;
    var hv_raw = hv_C - dt * ((flux_east.z - flux_west.z + flux_north.z - flux_south.z) * inv_dx)
        + dt * source_slope_y + dt * params.viscosity * laplacian_hv;

    // Semi-implicit Manning bed friction to eliminate numerical instability at shallow depths
    var hu_new = 0.0;
    var hv_new = 0.0;

    if (h_new > EPSILON_H) {
        let u_raw = hu_raw / h_new;
        let v_raw = hv_raw / h_new;
        let speed = sqrt(u_raw * u_raw + v_raw * v_raw);

        let local_n = params.manning_n * max(0.1, bed_c.y);
        let h_pow_4_3 = pow(h_new, 4.0 / 3.0);
        let friction_drag = g * (local_n * local_n) * speed / max(h_pow_4_3, 1e-4);
        let dampening = 1.0 / (1.0 + dt * friction_drag);

        hu_new = hu_raw * dampening;
        hv_new = hv_raw * dampening;
    }

    // Passive tracer advection (e.g. tannin concentration from leaf litter)
    let div_tracer = (
        select(0.0, flux_east.x * tracer_C, flux_east.x >= 0.0) + select(0.0, flux_east.x * state_E.w, flux_east.x < 0.0)
        - (select(0.0, flux_west.x * state_W.w, flux_west.x >= 0.0) + select(0.0, flux_west.x * tracer_C, flux_west.x < 0.0))
        + select(0.0, flux_north.x * tracer_C, flux_north.x >= 0.0) + select(0.0, flux_north.x * state_N.w, flux_north.x < 0.0)
        - (select(0.0, flux_south.x * state_S.w, flux_south.x >= 0.0) + select(0.0, flux_south.x * tracer_C, flux_south.x < 0.0))
    ) * inv_dx;

    let tracer_new = select(0.0, max(0.0, (tracer_C * h_C - dt * div_tracer) / max(h_new, EPSILON_H)), h_new > EPSILON_H);

    textureStore(state_next, coord, vec4<f32>(h_new, hu_new, hv_new, tracer_new));
}
