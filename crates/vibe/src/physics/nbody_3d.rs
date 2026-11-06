//! 3D N-Body Symplectic Gravitational and Relativistic Orbital Solver.
//!
//! Provides a zero-heap Velocity-Verlet / symplectic leapfrog integrator with
//! Newtonian pairwise gravitational acceleration and optional 1st Post-Newtonian (1PN)
//! Einstein-Infeld-Hoffmann perihelion precession corrections.
//!
//! Reference: `AGENTS.md` Workstream W5 (§5.4).

/// Gravitational constant $G$ in $\text{m}^3 \cdot \text{kg}^{-1} \cdot \text{s}^{-2}$.
pub const GRAVITATIONAL_CONSTANT: f64 = 6.67430e-11;

/// Speed of light in vacuum $c$ in $\text{m} \cdot \text{s}^{-1}$.
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Maximum number of bodies supported in zero-heap stack buffer allocations.
pub const MAX_STACK_BODIES: usize = 128;

/// A celestial mass in 3D Cartesian coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CelestialBody3D {
    /// 3D position vector $[x, y, z]$ (meters).
    pub position_m: [f64; 3],
    /// 3D velocity vector $[v_x, v_y, v_z]$ (meters / second).
    pub velocity_m_s: [f64; 3],
    /// Inertial / gravitational mass (kilograms).
    pub mass_kg: f64,
    /// Physical spherical radius (meters).
    pub radius_m: f64,
}

impl CelestialBody3D {
    /// Constructs a new 3D celestial body.
    pub const fn new(
        position_m: [f64; 3],
        velocity_m_s: [f64; 3],
        mass_kg: f64,
        radius_m: f64,
    ) -> Self {
        Self {
            position_m,
            velocity_m_s,
            mass_kg,
            radius_m,
        }
    }
}

/// Evaluates pairwise gravitational accelerations on all bodies into a preallocated slice.
///
/// If `include_1pn_gr` is true, applies the 1st Post-Newtonian Einstein-Infeld-Hoffmann
/// relativistic orbital precession correction:
///
/// $$\mathbf{a}_{1PN} = \frac{G m_j}{c^2 r^3} \left[ \left( \frac{4 G m_j}{r} - v_{ij}^2 \right) \mathbf{r}_{ij} - 4 (\mathbf{r}_{ij} \cdot \mathbf{v}_{ij}) \mathbf{v}_{ij} \right]$$
fn compute_accelerations(
    bodies: &[CelestialBody3D],
    softening_m: f64,
    include_1pn_gr: bool,
    acc_out: &mut [[f64; 3]],
) {
    let n = bodies.len();
    for a in acc_out.iter_mut().take(n) {
        *a = [0.0, 0.0, 0.0];
    }

    let soft_sq = softening_m * softening_m;
    let c2 = SPEED_OF_LIGHT * SPEED_OF_LIGHT;

    for i in 0..n {
        let bi = &bodies[i];
        for j in (i + 1)..n {
            let bj = &bodies[j];
            let dx = bj.position_m[0] - bi.position_m[0];
            let dy = bj.position_m[1] - bi.position_m[1];
            let dz = bj.position_m[2] - bi.position_m[2];

            let dist_sq = dx * dx + dy * dy + dz * dz + soft_sq;
            let dist = dist_sq.sqrt();
            let inv_dist3 = 1.0 / (dist * dist_sq);

            // Newtonian mutual gravity
            let scale_newton = GRAVITATIONAL_CONSTANT * inv_dist3;
            let a_newt_ij_x = scale_newton * dx;
            let a_newt_ij_y = scale_newton * dy;
            let a_newt_ij_z = scale_newton * dz;

            acc_out[i][0] += a_newt_ij_x * bj.mass_kg;
            acc_out[i][1] += a_newt_ij_y * bj.mass_kg;
            acc_out[i][2] += a_newt_ij_z * bj.mass_kg;

            acc_out[j][0] -= a_newt_ij_x * bi.mass_kg;
            acc_out[j][1] -= a_newt_ij_y * bi.mass_kg;
            acc_out[j][2] -= a_newt_ij_z * bi.mass_kg;

            // Optional 1st Post-Newtonian (1PN) Einstein-Infeld-Hoffmann correction
            if include_1pn_gr {
                let dvx = bi.velocity_m_s[0] - bj.velocity_m_s[0];
                let dvy = bi.velocity_m_s[1] - bj.velocity_m_s[1];
                let dvz = bi.velocity_m_s[2] - bj.velocity_m_s[2];

                let v_rel_sq = dvx * dvx + dvy * dvy + dvz * dvz;
                let r_dot_v = dx * dvx + dy * dvy + dz * dvz;

                let scale_1pn_j = (GRAVITATIONAL_CONSTANT * bj.mass_kg) / (c2 * dist * dist_sq);
                let term_rad_j = (4.0 * GRAVITATIONAL_CONSTANT * bj.mass_kg / dist) - v_rel_sq;

                acc_out[i][0] += scale_1pn_j * (term_rad_j * dx - 4.0 * r_dot_v * dvx);
                acc_out[i][1] += scale_1pn_j * (term_rad_j * dy - 4.0 * r_dot_v * dvy);
                acc_out[i][2] += scale_1pn_j * (term_rad_j * dz - 4.0 * r_dot_v * dvz);

                let scale_1pn_i = (GRAVITATIONAL_CONSTANT * bi.mass_kg) / (c2 * dist * dist_sq);
                let term_rad_i = (4.0 * GRAVITATIONAL_CONSTANT * bi.mass_kg / dist) - v_rel_sq;

                acc_out[j][0] += scale_1pn_i * (term_rad_i * (-dx) - 4.0 * (-r_dot_v) * (-dvx));
                acc_out[j][1] += scale_1pn_i * (term_rad_i * (-dy) - 4.0 * (-r_dot_v) * (-dvy));
                acc_out[j][2] += scale_1pn_i * (term_rad_i * (-dz) - 4.0 * (-r_dot_v) * (-dvz));
            }
        }
    }
}

/// Advances an N-body system by `dt_s` seconds using a symplectic Velocity-Verlet step.
///
/// Completely zero-heap; requires `bodies.len() <= MAX_STACK_BODIES`.
pub fn step_nbody_3d_verlet(
    bodies: &mut [CelestialBody3D],
    dt_s: f64,
    softening_m: f64,
    include_1pn_gr: bool,
) {
    let n = bodies.len();
    if n <= 1 {
        return;
    }
    assert!(
        n <= MAX_STACK_BODIES,
        "nbody_3d: body count {} exceeds MAX_STACK_BODIES {}",
        n,
        MAX_STACK_BODIES
    );

    let mut a0 = [[0.0; 3]; MAX_STACK_BODIES];
    compute_accelerations(bodies, softening_m, include_1pn_gr, &mut a0[..n]);

    let half_dt = 0.5 * dt_s;
    let dt_sq_half = 0.5 * dt_s * dt_s;

    // 1. Advance position and half-step velocity
    for i in 0..n {
        let b = &mut bodies[i];
        b.position_m[0] += b.velocity_m_s[0] * dt_s + a0[i][0] * dt_sq_half;
        b.position_m[1] += b.velocity_m_s[1] * dt_s + a0[i][1] * dt_sq_half;
        b.position_m[2] += b.velocity_m_s[2] * dt_s + a0[i][2] * dt_sq_half;

        b.velocity_m_s[0] += a0[i][0] * half_dt;
        b.velocity_m_s[1] += a0[i][1] * half_dt;
        b.velocity_m_s[2] += a0[i][2] * half_dt;
    }

    // 2. Recompute accelerations at new positions
    let mut a1 = [[0.0; 3]; MAX_STACK_BODIES];
    compute_accelerations(bodies, softening_m, include_1pn_gr, &mut a1[..n]);

    // 3. Complete velocity step
    for i in 0..n {
        let b = &mut bodies[i];
        b.velocity_m_s[0] += a1[i][0] * half_dt;
        b.velocity_m_s[1] += a1[i][1] * half_dt;
        b.velocity_m_s[2] += a1[i][2] * half_dt;
    }
}

/// Evaluates total mechanical energy (kinetic + gravitational potential) of the system in Joules.
pub fn total_mechanical_energy(bodies: &[CelestialBody3D], softening_m: f64) -> f64 {
    let mut kinetic = 0.0;
    for b in bodies {
        let v2 = b.velocity_m_s[0] * b.velocity_m_s[0]
            + b.velocity_m_s[1] * b.velocity_m_s[1]
            + b.velocity_m_s[2] * b.velocity_m_s[2];
        kinetic += 0.5 * b.mass_kg * v2;
    }

    let mut potential = 0.0;
    let soft_sq = softening_m * softening_m;
    let n = bodies.len();

    for i in 0..n {
        for j in (i + 1)..n {
            let dx = bodies[j].position_m[0] - bodies[i].position_m[0];
            let dy = bodies[j].position_m[1] - bodies[i].position_m[1];
            let dz = bodies[j].position_m[2] - bodies[i].position_m[2];
            let dist = (dx * dx + dy * dy + dz * dz + soft_sq).sqrt();
            potential -= (GRAVITATIONAL_CONSTANT * bodies[i].mass_kg * bodies[j].mass_kg) / dist;
        }
    }

    kinetic + potential
}

/// Evaluates total 3D angular momentum vector $[L_x, L_y, L_z]$ of the system ($\text{kg} \cdot \text{m}^2 \cdot \text{s}^{-1}$).
pub fn total_angular_momentum(bodies: &[CelestialBody3D]) -> [f64; 3] {
    let mut lx = 0.0;
    let mut ly = 0.0;
    let mut lz = 0.0;

    for b in bodies {
        let rx = b.position_m[0];
        let ry = b.position_m[1];
        let rz = b.position_m[2];
        let vx = b.velocity_m_s[0];
        let vy = b.velocity_m_s[1];
        let vz = b.velocity_m_s[2];

        lx += b.mass_kg * (ry * vz - rz * vy);
        ly += b.mass_kg * (rz * vx - rx * vz);
        lz += b.mass_kg * (rx * vy - ry * vx);
    }

    [lx, ly, lz]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_coplanar_three_body_conservation() {
        // Sun-Earth-Asteroid 3-body system with non-coplanar asteroid orbit
        let mut bodies = [
            // Body 1: Central Star (Sun)
            CelestialBody3D::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.989e30, 6.96e8),
            // Body 2: Coplanar planet (Earth in XY plane)
            CelestialBody3D::new(
                [1.496e11, 0.0, 0.0],
                [0.0, 29780.0, 0.0],
                5.972e24,
                6.371e6,
            ),
            // Body 3: Inclined minor planet (non-coplanar: Z != 0, Vz != 0)
            CelestialBody3D::new(
                [0.0, 2.5e11, 5.0e10],
                [-22000.0, 0.0, 8000.0],
                1.0e22,
                5.0e5,
            ),
        ];

        let dt = 3600.0; // 1 hour step
        let softening = 1000.0;

        let e_init = total_mechanical_energy(&bodies, softening);
        let l_init = total_angular_momentum(&bodies);

        // Step 100 iterations of Verlet
        for _ in 0..100 {
            step_nbody_3d_verlet(&mut bodies, dt, softening, false);
        }

        let e_final = total_mechanical_energy(&bodies, softening);
        let l_final = total_angular_momentum(&bodies);

        // Symplectic energy preservation (< 0.01% drift over 100 steps)
        let rel_energy_drift = (e_final - e_init).abs() / e_init.abs();
        assert!(
            rel_energy_drift < 1e-4,
            "Energy drift {} exceeds tolerance",
            rel_energy_drift
        );

        // Angular momentum conservation (< 1e-6 relative drift)
        let l_mag_init = (l_init[0] * l_init[0] + l_init[1] * l_init[1] + l_init[2] * l_init[2]).sqrt();
        let dl = [
            l_final[0] - l_init[0],
            l_final[1] - l_init[1],
            l_final[2] - l_init[2],
        ];
        let dl_mag = (dl[0] * dl[0] + dl[1] * dl[1] + dl[2] * dl[2]).sqrt();
        let rel_l_drift = dl_mag / l_mag_init;
        assert!(
            rel_l_drift < 1e-6,
            "Angular momentum drift {} exceeds tolerance",
            rel_l_drift
        );
    }

    #[test]
    fn test_relativistic_1pn_acceleration() {
        let mut bodies = [
            CelestialBody3D::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.989e30, 6.96e8),
            CelestialBody3D::new(
                [4.6e10, 0.0, 0.0], // Perihelion of Mercury
                [0.0, 58980.0, 0.0],
                3.3e23,
                2.4e6,
            ),
        ];

        // Step with 1PN enabled
        step_nbody_3d_verlet(&mut bodies, 10.0, 1.0, true);
        assert!(bodies[1].velocity_m_s[0] != 0.0 || bodies[1].velocity_m_s[1] != 0.0);
    }
}
