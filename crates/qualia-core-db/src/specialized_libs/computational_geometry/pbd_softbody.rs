//! P21.8 — Position-Based Dynamics / XPBD soft-body stepper
//! (Müller, Heidelberger, Hennix & Ratcliff 2007; Macklin, Müller & Chentanez
//! 2016 XPBD compliance).
//!
//! Verlet integration + deterministic Gauss–Seidel projection over distance
//! and tetrahedral-volume constraints, with a ground-plane collision clamp —
//! the minimal stable kernel for muscle/fat jiggle and foliage response.
//!
//! Tier-1 `HotZeroHeap`: all state lives in caller-owned slices.

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PbdConstraint {
    /// Stretch/shear spring: keep |p_b − p_a| = rest.
    Distance { a: u32, b: u32, rest: f64 },
    /// Tetrahedral volume preservation: keep vol(a,b,c,d) = rest.
    Volume {
        a: u32,
        b: u32,
        c: u32,
        d: u32,
        rest: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PbdReceipt {
    pub iterations_run: usize,
    pub max_residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbdError {
    DimensionMismatch,
    InvalidParameters,
    IndexOutOfBounds { constraint: usize },
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[inline]
fn tet_volume(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    dot(cross(sub(b, a), sub(c, a)), sub(d, a)) / 6.0
}

/// One XPBD substep:
///   predict:  `p ← p + (p − p_prev) · (1 − damping) + g·dt²`
///   project:  `iterations` Gauss–Seidel sweeps over `constraints`
///   collide:  clamp `p.z ≥ ground_z` when `ground_z` is finite.
///
/// `positions` and `previous` swap roles each call; `inv_masses[i] = 0` pins
/// a particle. Deterministic scan order over constraints.
#[allow(clippy::too_many_arguments)]
pub fn pbd_step(
    positions: &mut [Point3],
    previous: &mut [Point3],
    inv_masses: &[f64],
    constraints: &[PbdConstraint],
    gravity: Point3,
    damping: f64,
    dt: f64,
    iterations: usize,
    ground_z: f64,
) -> Result<PbdReceipt, PbdError> {
    let n = positions.len();
    if previous.len() < n || inv_masses.len() < n {
        return Err(PbdError::DimensionMismatch);
    }
    if !(dt > 0.0 && dt.is_finite()) || iterations == 0 {
        return Err(PbdError::InvalidParameters);
    }
    let damp = (1.0 - damping.clamp(0.0, 1.0)).max(0.0);
    let g2 = dt * dt;

    // Predict (Verlet).
    for i in 0..n {
        if inv_masses[i] == 0.0 {
            previous[i] = positions[i];
            continue;
        }
        let vel = scale(sub(positions[i], previous[i]), damp);
        previous[i] = positions[i];
        positions[i] = add(
            add(positions[i], vel),
            Point3::new(gravity.x * g2, gravity.y * g2, gravity.z * g2),
        );
    }

    let mut max_res = 0.0f64;
    for _ in 0..iterations {
        for (ci, con) in constraints.iter().enumerate() {
            match *con {
                PbdConstraint::Distance { a, b, rest } => {
                    let (ia, ib) = (a as usize, b as usize);
                    if ia >= n || ib >= n {
                        return Err(PbdError::IndexOutOfBounds { constraint: ci });
                    }
                    let (wa, wb) = (inv_masses[ia], inv_masses[ib]);
                    let wsum = wa + wb;
                    if wsum <= 0.0 {
                        continue;
                    }
                    let d = sub(positions[ib], positions[ia]);
                    let len = dot(d, d).sqrt();
                    if len < 1e-12 {
                        continue;
                    }
                    let err = len - rest;
                    max_res = max_res.max(err.abs());
                    let corr = scale(d, err / (len * wsum));
                    positions[ia] = add(positions[ia], scale(corr, wa));
                    positions[ib] = add(positions[ib], scale(corr, -wb));
                }
                PbdConstraint::Volume { a, b, c, d, rest } => {
                    let (ia, ib, ic, id) = (a as usize, b as usize, c as usize, d as usize);
                    if ia >= n || ib >= n || ic >= n || id >= n {
                        return Err(PbdError::IndexOutOfBounds { constraint: ci });
                    }
                    let (pa, pb, pc, pd) =
                        (positions[ia], positions[ib], positions[ic], positions[id]);
                    let v = tet_volume(pa, pb, pc, pd);
                    let err = v - rest;
                    max_res = max_res.max(err.abs());
                    // Signed-volume gradients for V = det[b−a, c−a, d−a]/6:
                    //   ∇_b V = (c−a)×(d−a)/6, ∇_c V = (d−a)×(b−a)/6,
                    //   ∇_d V = (b−a)×(c−a)/6, ∇_a V = −(∇_b+∇_c+∇_d).
                    let (c1, c2, c3) = (sub(pb, pa), sub(pc, pa), sub(pd, pa));
                    let x23 = cross(c2, c3);
                    let x31 = cross(c3, c1);
                    let x12 = cross(c1, c2);
                    let g = [
                        scale(add(add(x23, x31), x12), -1.0 / 6.0),
                        scale(x23, 1.0 / 6.0),
                        scale(x31, 1.0 / 6.0),
                        scale(x12, 1.0 / 6.0),
                    ];
                    let w = [
                        inv_masses[ia],
                        inv_masses[ib],
                        inv_masses[ic],
                        inv_masses[id],
                    ];
                    let mut denom = 0.0;
                    for k in 0..4 {
                        denom += w[k] * dot(g[k], g[k]);
                    }
                    if denom < 1e-15 {
                        continue;
                    }
                    let lambda = err / denom;
                    for (k, idx) in [ia, ib, ic, id].iter().enumerate() {
                        positions[*idx] = add(positions[*idx], scale(g[k], -lambda * w[k]));
                    }
                }
            }
        }
    }

    // Ground-plane collision.
    if ground_z.is_finite() {
        for p in positions.iter_mut() {
            if p.z < ground_z {
                p.z = ground_z;
            }
        }
    }
    Ok(PbdReceipt {
        iterations_run: iterations,
        max_residual: max_res,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_constraint_restores_rest_length() {
        let mut pos = [Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)];
        let mut prev = pos;
        let inv = [1.0, 1.0];
        let cons = [PbdConstraint::Distance {
            a: 0,
            b: 1,
            rest: 1.0,
        }];
        pbd_step(
            &mut pos,
            &mut prev,
            &inv,
            &cons,
            Point3::default(),
            0.0,
            0.016,
            8,
            f64::NAN,
        )
        .unwrap();
        let d = sub(pos[1], pos[0]);
        let len = dot(d, d).sqrt();
        assert!((len - 1.0).abs() < 1e-6, "rest length violated: {len}");
    }

    #[test]
    fn gravity_pulls_particle_to_ground() {
        // dt=0.5, g=−9.8 → Δz = −9.8·0.25 = −2.45 → clamps to z=0.
        let mut pos = [Point3::new(0.0, 0.0, 1.0)];
        let mut prev = pos;
        let inv = [1.0];
        pbd_step(
            &mut pos,
            &mut prev,
            &inv,
            &[],
            Point3::new(0.0, 0.0, -9.8),
            0.0,
            0.5,
            4,
            0.0,
        )
        .unwrap();
        assert!(pos[0].z <= 0.0 + 1e-9, "ground clamp failed: {}", pos[0].z);
    }

    #[test]
    fn volume_constraint_resists_tet_compression() {
        let mut pos = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 0.5),
        ];
        let rest = tet_volume(pos[0], pos[1], pos[2], pos[3]);
        let mut prev = pos;
        // Squash vertex 3 toward the base plane.
        pos[3].z = 0.05;
        prev[3] = pos[3];
        let inv = [1.0; 4];
        let cons = [PbdConstraint::Volume {
            a: 0,
            b: 1,
            c: 2,
            d: 3,
            rest,
        }];
        let rec = pbd_step(
            &mut pos,
            &mut prev,
            &inv,
            &cons,
            Point3::default(),
            1.0,
            0.016,
            20,
            f64::NAN,
        )
        .unwrap();
        let v = tet_volume(pos[0], pos[1], pos[2], pos[3]);
        assert!(
            (v - rest).abs() < rec.max_residual + 1e-6 || (v - rest).abs() < 1e-4,
            "volume not restored: {v} vs {rest}"
        );
        assert!((v - rest).abs() < 1e-3);
    }
}
