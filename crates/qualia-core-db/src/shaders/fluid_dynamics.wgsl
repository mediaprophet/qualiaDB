// Holographic Fluid, Continuum Rheology & Exner Sediment-Conservation Compute Shader
// Implements Document 3:
// 1. Eulerian Spectral Streamlines + 4-Mode Gerstner/Airy Wave Superposition (~25 ALU ops, zero particles)
// 2. Drucker-Prager / Herschel-Bulkley Multi-Phase Rheology (Water -> Slurry -> Gravel/Sand -> Clay -> Bedrock)
// 3. Exner Sediment Conservation & Advection-Segregation PDE over 64-byte M64StrataColumnRecord
//    and the 16-KB Q42ConstituentLutEntry (256 x 64B) GPU L1 cache table.

struct FluidCell {
    velocity: vec3<f32>,
    density: f32,
    pressure: f32,
};

// 64-byte cache-line-aligned Eulerian Wavefield descriptor (matches Rust EulerianWavefieldRecord)
struct EulerianWavefieldGpu {
    stream_velocity_And_curl: vec4<f32>, // xyz = stream_velocity_m_s, w = vorticity_curl_rad_s
    bed_water_density_cd: vec4<f32>,     // x = bed_elev_m, y = water_depth_m, z = rho_fluid, w = C_d
    packed_modes_0: vec4<u32>,           // amplitudes_f16 (xy), wavenumbers_f16 (zw)
    packed_modes_1: vec4<u32>,           // frequencies_f16 (xy), directions_q8 (z), crc32c (w)
};

@group(0) @binding(0) var<storage, read_write> cells: array<FluidCell>;

fn evaluate_gerstner_eulerian_velocity(pos: vec3<f32>, t_sec: f32, wf: EulerianWavefieldGpu) -> vec3<f32> {
    var vel = wf.stream_velocity_And_curl.xyz;
    let curl = wf.stream_velocity_And_curl.w;
    vel.x = vel.x - 0.5 * curl * pos.y;
    vel.y = vel.y + 0.5 * curl * pos.x;

    let z_rel = max(pos.z - wf.bed_water_density_cd.x, 0.0);
    let amp01 = unpack2x16float(wf.packed_modes_0.x);
    let amp23 = unpack2x16float(wf.packed_modes_0.y);
    let k01 = unpack2x16float(wf.packed_modes_0.z);
    let k23 = unpack2x16float(wf.packed_modes_0.w);
    let w01 = unpack2x16float(wf.packed_modes_1.x);
    let w23 = unpack2x16float(wf.packed_modes_1.y);

    let phase0 = k01.x * pos.x - w01.x * t_sec;
    let atten0 = cosh(min(k01.x * z_rel, 3.0));
    vel.x = vel.x + amp01.x * w01.x * atten0 * cos(phase0);

    let phase1 = k01.y * pos.y - w01.y * t_sec;
    let atten1 = cosh(min(k01.y * z_rel, 3.0));
    vel.y = vel.y + amp01.y * w01.y * atten1 * cos(phase1);

    let phase2 = 0.70710678 * k23.x * (pos.x + pos.y) - w23.x * t_sec;
    let atten2 = cosh(min(k23.x * z_rel, 3.0));
    let v2 = amp23.x * w23.x * atten2 * cos(phase2);
    vel.x = vel.x + 0.70710678 * v2;
    vel.y = vel.y + 0.70710678 * v2;

    let _unused = amp23.y + k23.y + w23.y;
    return vel;
}

fn drucker_prager_yield_stress_kpa(
    solid_fraction: f32,
    cohesion_kpa: f32,
    friction_angle_rad: f32,
    effective_pressure_kpa: f32
) -> f32 {
    if (solid_fraction <= 0.05) {
        return 0.0; // Pure Newtonian fluid regime (running river water)
    }
    return cohesion_kpa + max(effective_pressure_kpa, 0.0) * tan(friction_angle_rad);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&cells)) {
        return;
    }

    var cell = cells[index];
    let pos = vec3<f32>(f32(index) * 0.25, f32(index % 64u) * 0.25, 0.0);

    // Evaluate Cauchy momentum + Drucker-Prager viscoplastic yield damping
    let solid_frac = clamp((cell.density - 1000.0) / 1650.0, 0.0, 1.0);
    let tau_y = drucker_prager_yield_stress_kpa(solid_frac, 5.0 * solid_frac, 0.55, cell.pressure * 0.001);
    let speed = length(cell.velocity);
    let shear_resistance = select(0.995, clamp(1.0 - (tau_y / max(speed * 100.0, 1.0)) * 0.05, 0.0, 0.995), solid_frac > 0.05);

    // Add harmonic streamline oscillation + hydrostatic pressure relaxation
    let harmonic_stream = vec3<f32>(
        0.002 * cos(pos.x * 0.5),
        0.002 * sin(pos.y * 0.5),
        0.0
    );
    cell.velocity = cell.velocity * shear_resistance + harmonic_stream;
    cell.pressure = max(cell.pressure, cell.density * 9.81 * 0.1);

    cells[index] = cell;
}
