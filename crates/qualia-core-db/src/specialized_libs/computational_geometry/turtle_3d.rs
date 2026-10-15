//! P20.4 — 3-D L-system geometric turtle (`Turtle3D`) with bounded state stack,
//! gravitropism/phototropism bending, generalized cylinder welding, and polygon
//! leaf construction.
//!
//! Reference: Prusinkiewicz & Lindenmayer (1990), *The Algorithmic Beauty of
//! Plants*, Ch. 1–2 (3D turtle interpretation and tropism torque).
//!
//! The turtle evaluator (`execute_turtle_3d`) is Tier-1 `HotZeroHeap`: it uses
//! a fixed stack `[TurtleState; MAX_TURTLE_STACK]` and writes vertices and
//! triangles directly into caller-owned slices with zero heap allocation.

use super::primitives::Point3;

/// Maximum push/pop nesting depth for [`Turtle3D`].
pub const MAX_TURTLE_STACK: usize = 64;
/// Maximum vertices in a single turtle polygon (`startPolygon`..`stopPolygon`).
pub const MAX_POLYGON_VERTS: usize = 32;
/// Maximum circumferential slices for generalized cylinder rings.
pub const MAX_RING_SLICES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurtleError {
    StackOverflow,
    StackUnderflow,
    PolygonOverflow,
    InvalidParameters,
    OutputTooSmall {
        required_vertices: usize,
        required_triangles: usize,
    },
}

/// 3-D turtle state frame $(\mathbf{P}, \mathbf{H}, \mathbf{L}, \mathbf{U})$
/// plus morphological parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurtleState {
    pub position: Point3,
    /// Heading unit vector $\mathbf{H}$.
    pub heading: Point3,
    /// Left unit vector $\mathbf{L}$.
    pub left: Point3,
    /// Up unit vector $\mathbf{U} = \mathbf{H} \times \mathbf{L}$.
    pub up: Point3,
    pub width: f64,
    pub step_length: f64,
    pub angle_deg: f64,
    pub tropism: Point3,
    pub elasticity: f64,
    pub color_id: u32,
}

impl Default for TurtleState {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            heading: Point3::new(0.0, 0.0, 1.0),
            left: Point3::new(-1.0, 0.0, 0.0),
            up: Point3::new(0.0, 1.0, 0.0),
            width: 0.1,
            step_length: 1.0,
            angle_deg: 25.0,
            tropism: Point3::new(0.0, 0.0, 1.0),
            elasticity: 0.0,
            color_id: 0,
        }
    }
}

/// Typed 3-D L-system turtle command stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TurtleCmd {
    /// `F(length, top_radius)`: advance by `length` along heading and emit a
    /// cylindrical/frustum segment (or weld to active generalized cylinder).
    ForwardDraw { length: f64, top_width: Option<f64> },
    /// `f(length)`: advance by `length` without emitting geometry.
    ForwardMove(f64),
    /// `+(angle_deg)` / `-(angle_deg)`: rotate around Up axis ($\mathbf{U}$).
    TurnLeft(f64),
    /// `&(angle_deg)` / `^(angle_deg)`: pitch around Left axis ($\mathbf{L}$).
    PitchDown(f64),
    /// `\(angle_deg)` / `/(angle_deg)`: roll around Heading axis ($\mathbf{H}$).
    RollLeft(f64),
    /// `$`: roll around $\mathbf{H}$ so $\mathbf{L}$ is horizontal ($\mathbf{V} \times \mathbf{H}$).
    RollToVert,
    /// `[`: push current turtle state onto the bounded stack.
    Push,
    /// `]`: pop turtle state from the bounded stack.
    Pop,
    /// `!(width)`: set current branch diameter/radius width.
    SetWidth(f64),
    /// Set tropism unit target vector and bending elasticity $\alpha$.
    SetTropism { target: Point3, elasticity: f64 },
    /// `{`: begin recording a planar leaf polygon.
    StartPolygon,
    /// `.`: record current turtle position as a polygon vertex.
    PolygonVertex,
    /// `}`: close and fan-triangulate the recorded polygon.
    StopPolygon,
    /// Begin welding consecutive `ForwardDraw` rings into a smooth generalized cylinder.
    StartGeneralizedCylinder,
    /// End generalized cylinder welding.
    StopGeneralizedCylinder,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurtleMeshReceipt {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub final_state: TurtleState,
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
fn norm(a: Point3) -> f64 {
    dot(a, a).sqrt()
}
#[inline]
fn normalize(a: Point3) -> Option<Point3> {
    let n = norm(a);
    (n > 1e-12 && n.is_finite()).then(|| scale(a, 1.0 / n))
}

/// Rotate vector `v` around unit `axis` by `angle_rad` (Rodrigues' formula).
#[inline]
fn rotate_around_axis(v: Point3, axis: Point3, angle_rad: f64) -> Point3 {
    let (c, s) = (angle_rad.cos(), angle_rad.sin());
    let k_dot_v = dot(axis, v);
    let k_cross_v = cross(axis, v);
    add(
        add(scale(v, c), scale(k_cross_v, s)),
        scale(axis, k_dot_v * (1.0 - c)),
    )
}

fn rotate_frame(state: &mut TurtleState, axis: Point3, angle_rad: f64) {
    if let Some(ax) = normalize(axis) {
        state.heading =
            normalize(rotate_around_axis(state.heading, ax, angle_rad)).unwrap_or(state.heading);
        state.left = normalize(rotate_around_axis(state.left, ax, angle_rad)).unwrap_or(state.left);
        state.up = cross(state.heading, state.left);
    }
}

fn apply_tropism(state: &mut TurtleState) {
    if state.elasticity.abs() <= 1e-12 {
        return;
    }
    if let Some(t_dir) = normalize(state.tropism) {
        let torque = cross(state.heading, t_dir);
        let sin_theta = norm(torque);
        if sin_theta > 1e-12 {
            let bend_angle = state.elasticity * sin_theta;
            rotate_frame(state, torque, bend_angle);
        }
    }
}

/// Execute a 3-D turtle command program into caller-owned `out_vertices` and
/// `out_triangles` buffers.
///
/// Tier-1 `HotZeroHeap`: zero heap allocations.
pub fn execute_turtle_3d(
    commands: &[TurtleCmd],
    initial: TurtleState,
    ring_slices: usize,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<TurtleMeshReceipt, TurtleError> {
    if !(3..=MAX_RING_SLICES).contains(&ring_slices) {
        return Err(TurtleError::InvalidParameters);
    }

    let mut state = initial;
    let mut stack = [TurtleState::default(); MAX_TURTLE_STACK];
    let mut sp = 0usize;

    let mut poly_verts = [Point3::default(); MAX_POLYGON_VERTS];
    let mut poly_len = 0usize;
    let mut in_polygon = false;

    let mut gc_active = false;
    let mut gc_prev_ring_base: Option<u32> = None;
    let mut gc_ring_stack = [None; MAX_TURTLE_STACK];

    let mut v_count = 0usize;
    let mut t_count = 0usize;

    let emit_ring = |pos: Point3,
                     left: Point3,
                     up: Point3,
                     radius: f64,
                     v_cnt: &mut usize,
                     verts: &mut [Point3]|
     -> Result<u32, TurtleError> {
        if *v_cnt + ring_slices > verts.len() {
            return Err(TurtleError::OutputTooSmall {
                required_vertices: *v_cnt + ring_slices,
                required_triangles: 0,
            });
        }
        let base = *v_cnt as u32;
        for s in 0..ring_slices {
            let a = core::f64::consts::TAU * (s as f64) / (ring_slices as f64);
            let offset = add(scale(left, radius * a.cos()), scale(up, radius * a.sin()));
            verts[*v_cnt + s] = add(pos, offset);
        }
        *v_cnt += ring_slices;
        Ok(base)
    };

    for &cmd in commands {
        match cmd {
            TurtleCmd::ForwardMove(len) => {
                if !len.is_finite() {
                    return Err(TurtleError::InvalidParameters);
                }
                state.position = add(state.position, scale(state.heading, len));
                apply_tropism(&mut state);
            }
            TurtleCmd::ForwardDraw { length, top_width } => {
                if !length.is_finite() || length <= 0.0 || state.width <= 0.0 {
                    return Err(TurtleError::InvalidParameters);
                }
                let r0 = state.width * 0.5;
                let w1 = top_width.unwrap_or(state.width);
                let r1 = (w1 * 0.5).max(0.0);

                let base0 = if gc_active {
                    if let Some(prev) = gc_prev_ring_base {
                        prev
                    } else {
                        emit_ring(
                            state.position,
                            state.left,
                            state.up,
                            r0,
                            &mut v_count,
                            out_vertices,
                        )?
                    }
                } else {
                    emit_ring(
                        state.position,
                        state.left,
                        state.up,
                        r0,
                        &mut v_count,
                        out_vertices,
                    )?
                };

                state.position = add(state.position, scale(state.heading, length));
                apply_tropism(&mut state);
                state.width = w1;

                let base1 = emit_ring(
                    state.position,
                    state.left,
                    state.up,
                    r1,
                    &mut v_count,
                    out_vertices,
                )?;
                if gc_active {
                    gc_prev_ring_base = Some(base1);
                }

                if t_count + ring_slices * 2 > out_triangles.len() {
                    return Err(TurtleError::OutputTooSmall {
                        required_vertices: v_count,
                        required_triangles: t_count + ring_slices * 2,
                    });
                }
                for s in 0..ring_slices {
                    let ns = (s + 1) % ring_slices;
                    let a = base0 + s as u32;
                    let b = base0 + ns as u32;
                    let c = base1 + ns as u32;
                    let d = base1 + s as u32;
                    out_triangles[t_count] = [a, b, c];
                    out_triangles[t_count + 1] = [a, c, d];
                    t_count += 2;
                }

                if in_polygon {
                    if poly_len >= MAX_POLYGON_VERTS {
                        return Err(TurtleError::PolygonOverflow);
                    }
                    poly_verts[poly_len] = state.position;
                    poly_len += 1;
                }
            }
            TurtleCmd::TurnLeft(deg) => {
                let axis = state.up;
                rotate_frame(&mut state, axis, deg.to_radians());
            }
            TurtleCmd::PitchDown(deg) => {
                let axis = state.left;
                rotate_frame(&mut state, axis, deg.to_radians());
            }
            TurtleCmd::RollLeft(deg) => {
                let axis = state.heading;
                rotate_frame(&mut state, axis, deg.to_radians());
            }
            TurtleCmd::RollToVert => {
                let world_up = Point3::new(0.0, 0.0, 1.0);
                if let Some(new_left) = normalize(cross(world_up, state.heading)) {
                    state.left = new_left;
                    state.up = cross(state.heading, state.left);
                }
            }
            TurtleCmd::Push => {
                if sp >= MAX_TURTLE_STACK {
                    return Err(TurtleError::StackOverflow);
                }
                stack[sp] = state;
                gc_ring_stack[sp] = gc_prev_ring_base;
                sp += 1;
            }
            TurtleCmd::Pop => {
                if sp == 0 {
                    return Err(TurtleError::StackUnderflow);
                }
                sp -= 1;
                state = stack[sp];
                gc_prev_ring_base = gc_ring_stack[sp];
            }
            TurtleCmd::SetWidth(w) => {
                if !w.is_finite() || w <= 0.0 {
                    return Err(TurtleError::InvalidParameters);
                }
                state.width = w;
            }
            TurtleCmd::SetTropism { target, elasticity } => {
                state.tropism = target;
                state.elasticity = elasticity;
            }
            TurtleCmd::StartPolygon => {
                in_polygon = true;
                poly_len = 0;
            }
            TurtleCmd::PolygonVertex => {
                if !in_polygon || poly_len >= MAX_POLYGON_VERTS {
                    return Err(TurtleError::PolygonOverflow);
                }
                poly_verts[poly_len] = state.position;
                poly_len += 1;
            }
            TurtleCmd::StopPolygon => {
                if in_polygon && poly_len >= 3 {
                    let need_t = poly_len - 2;
                    if v_count + poly_len > out_vertices.len()
                        || t_count + need_t > out_triangles.len()
                    {
                        return Err(TurtleError::OutputTooSmall {
                            required_vertices: v_count + poly_len,
                            required_triangles: t_count + need_t,
                        });
                    }
                    let base = v_count as u32;
                    out_vertices[v_count..v_count + poly_len]
                        .copy_from_slice(&poly_verts[..poly_len]);
                    v_count += poly_len;
                    for i in 1..poly_len - 1 {
                        out_triangles[t_count] = [base, base + i as u32, base + (i + 1) as u32];
                        t_count += 1;
                    }
                }
                in_polygon = false;
                poly_len = 0;
            }
            TurtleCmd::StartGeneralizedCylinder => {
                gc_active = true;
                gc_prev_ring_base = None;
            }
            TurtleCmd::StopGeneralizedCylinder => {
                gc_active = false;
                gc_prev_ring_base = None;
            }
        }
    }

    Ok(TurtleMeshReceipt {
        vertex_count: v_count,
        triangle_count: t_count,
        final_state: state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_branching_and_tropism_bend_towards_target() {
        let cmds = [
            TurtleCmd::StartGeneralizedCylinder,
            TurtleCmd::SetTropism {
                target: Point3::new(1.0, 0.0, 0.0),
                elasticity: 0.35,
            },
            TurtleCmd::ForwardDraw {
                length: 1.0,
                top_width: Some(0.08),
            },
            TurtleCmd::Push,
            TurtleCmd::TurnLeft(30.0),
            TurtleCmd::ForwardDraw {
                length: 0.8,
                top_width: Some(0.04),
            },
            TurtleCmd::Pop,
            TurtleCmd::ForwardDraw {
                length: 1.0,
                top_width: Some(0.05),
            },
            TurtleCmd::StopGeneralizedCylinder,
        ];
        let mut verts = [Point3::default(); 64];
        let mut tris = [[0u32; 3]; 128];
        let receipt =
            execute_turtle_3d(&cmds, TurtleState::default(), 8, &mut verts, &mut tris).unwrap();
        // Generalized-cylinder welding reuses the previous ring: seg1 emits
        // 2 rings (16 verts); the branch welds onto seg1's top ring and adds
        // 1 ring (8); Pop restores the trunk weld point so seg2 also welds
        // onto that ring and adds 1 ring (8) → 4 rings = 32 verts total.
        assert_eq!(receipt.vertex_count, 32);
        assert_eq!(receipt.triangle_count, 48);
        // Phototropism towards +X must deflect the final X position positively.
        assert!(
            receipt.final_state.position.x > 0.1,
            "expected positive +X tropism deflection, got {:?}",
            receipt.final_state.position
        );
    }
}
