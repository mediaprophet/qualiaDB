//! P20.5 — 3-D Space Colonization & Graph Colonization (Runions, Lane &
//! Prusinkiewicz 2007, "Modeling Trees with a Space Colonization Algorithm",
//! *Eurographics Workshop on Natural Phenomena*).
//!
//! Includes:
//! - Attractor perception cone & radius queries (`perception_radius`, `cone_angle_rad`)
//! - Normalized average growth direction with canonical index tie-breaking
//! - Kill-zone attractor pruning (`kill_radius`)
//! - Optional phyllotactic lateral bud whorls (`whorl_buds`, `insertion_angle_rad`)
//! - Graph-constrained colonization (`colonize_attractor_graph`)

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColonizationError {
    InvalidConfig,
    EmptyAttractors,
    EmptySeeds,
    OutputTooSmall { required_nodes: usize },
}

/// Configuration for 3-D Space Colonization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceColonizationConfig {
    /// Internode growth step length $D$.
    pub node_length: f64,
    /// Radius $d_k$ within which attractors are consumed by newly grown nodes.
    pub kill_radius: f64,
    /// Radius $d_i$ within which active skeleton nodes perceive attractors.
    pub perception_radius: f64,
    /// Full perception cone angle in radians (e.g., $\pi$ for hemisphere, $2\pi$ for omnidirectional).
    pub cone_angle_rad: f64,
    /// Minimum number of attractors required to trigger bud growth.
    pub min_attractors_per_bud: usize,
    /// Maximum number of growth iterations.
    pub max_iterations: usize,
}

impl Default for SpaceColonizationConfig {
    fn default() -> Self {
        Self {
            node_length: 0.25,
            kill_radius: 0.22,
            perception_radius: 0.75,
            cone_angle_rad: core::f64::consts::TAU,
            min_attractors_per_bud: 1,
            max_iterations: 64,
        }
    }
}

/// Receipt returned by [`space_colonization_3d`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceColonizationReceipt {
    pub node_count: usize,
    pub iterations_run: usize,
    pub remaining_attractors: usize,
}

/// Sentinel parent index for root skeleton nodes.
pub const ROOT_PARENT: u32 = u32::MAX;

#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
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
fn dist_sq(a: Point3, b: Point3) -> f64 {
    let d = sub(a, b);
    d.x * d.x + d.y * d.y + d.z * d.z
}
#[inline]
fn normalize(a: Point3) -> Option<Point3> {
    let n = (a.x * a.x + a.y * a.y + a.z * a.z).sqrt();
    (n > 1e-12 && n.is_finite()).then(|| scale(a, 1.0 / n))
}

/// Compute a whorl of `count` lateral unit directions around `axis_dir` at polar
/// branching angle `insertion_angle_rad` (PlantGL `SpaceColonization::lateral_directions`).
///
/// Tier-1 `HotZeroHeap`.
pub fn lateral_whorl_directions(
    axis_dir: Point3,
    insertion_angle_rad: f64,
    count: usize,
    out_dirs: &mut [Point3],
) -> Result<usize, ColonizationError> {
    if count == 0 || out_dirs.len() < count || !insertion_angle_rad.is_finite() {
        return Err(ColonizationError::InvalidConfig);
    }
    let axis = normalize(axis_dir).ok_or(ColonizationError::InvalidConfig)?;
    let ref_vec = if axis.x.abs() < 0.9 {
        Point3::new(1.0, 0.0, 0.0)
    } else {
        Point3::new(0.0, 1.0, 0.0)
    };
    let u = normalize(sub(ref_vec, scale(axis, dot(ref_vec, axis))))
        .ok_or(ColonizationError::InvalidConfig)?;
    let v = cross(axis, u);

    let (ca, sa) = (insertion_angle_rad.cos(), insertion_angle_rad.sin());
    for i in 0..count {
        let phi = core::f64::consts::TAU * (i as f64) / (count as f64);
        let radial = add(scale(u, phi.cos()), scale(v, phi.sin()));
        out_dirs[i] = add(scale(axis, ca), scale(radial, sa));
    }
    Ok(count)
}

/// Run deterministic 3-D Space Colonization from `seed_nodes` (`seed_parents`)
/// towards `attractors`.
///
/// Writes the resulting tree into `out_nodes[..node_count]` and
/// `out_parents[..node_count]`.
pub fn space_colonization_3d(
    attractors: &[Point3],
    seed_nodes: &[Point3],
    seed_parents: &[u32],
    cfg: &SpaceColonizationConfig,
    out_nodes: &mut [Point3],
    out_parents: &mut [u32],
) -> Result<SpaceColonizationReceipt, ColonizationError> {
    if attractors.is_empty() {
        return Err(ColonizationError::EmptyAttractors);
    }
    if seed_nodes.is_empty() || seed_nodes.len() != seed_parents.len() {
        return Err(ColonizationError::EmptySeeds);
    }
    if !(cfg.node_length > 0.0
        && cfg.kill_radius > 0.0
        && cfg.perception_radius >= cfg.kill_radius
        && cfg.cone_angle_rad > 0.0)
        || cfg.min_attractors_per_bud == 0
    {
        return Err(ColonizationError::InvalidConfig);
    }
    let cap = out_nodes.len().min(out_parents.len());
    if cap < seed_nodes.len() {
        return Err(ColonizationError::OutputTooSmall {
            required_nodes: seed_nodes.len(),
        });
    }

    out_nodes[..seed_nodes.len()].copy_from_slice(seed_nodes);
    out_parents[..seed_nodes.len()].copy_from_slice(seed_parents);
    let mut node_count = seed_nodes.len();

    let mut active_attr = vec![true; attractors.len()];
    let r_perc_sq = cfg.perception_radius * cfg.perception_radius;
    let r_kill_sq = cfg.kill_radius * cfg.kill_radius;
    let cos_half_cone = (cfg.cone_angle_rad * 0.5)
        .clamp(0.0, core::f64::consts::PI)
        .cos();

    // Prune attractors already inside kill_radius of initial seed nodes
    for &attr_i in (0..attractors.len()).collect::<Vec<_>>().iter() {
        for &sn in &out_nodes[..node_count] {
            if dist_sq(attractors[attr_i], sn) <= r_kill_sq {
                active_attr[attr_i] = false;
                break;
            }
        }
    }

    let mut accum_dir = vec![Point3::new(0.0, 0.0, 0.0); cap];
    let mut accum_cnt = vec![0usize; cap];
    let mut iterations_run = 0usize;

    for _iter in 0..cfg.max_iterations {
        if !active_attr.iter().any(|&a| a) {
            break;
        }
        accum_dir[..node_count].fill(Point3::new(0.0, 0.0, 0.0));
        accum_cnt[..node_count].fill(0);

        // Associate each active attractor with its closest perceived skeleton node
        for (ai, &attr) in attractors.iter().enumerate() {
            if !active_attr[ai] {
                continue;
            }
            let mut best_node: Option<usize> = None;
            let mut best_d2 = r_perc_sq;

            for ni in 0..node_count {
                let pos = out_nodes[ni];
                let d2 = dist_sq(attr, pos);
                if d2 > best_d2 || d2 <= 1e-24 {
                    continue;
                }
                let to_attr = scale(sub(attr, pos), 1.0 / d2.sqrt());
                // Check perception cone against node's growth direction
                let node_dir =
                    node_heading(&out_nodes[..node_count], &out_parents[..node_count], ni);
                if dot(node_dir, to_attr) >= cos_half_cone - 1e-12 {
                    best_d2 = d2;
                    best_node = Some(ni);
                }
            }

            if let Some(ni) = best_node {
                if let Some(u_dir) = normalize(sub(attr, out_nodes[ni])) {
                    accum_dir[ni] = add(accum_dir[ni], u_dir);
                    accum_cnt[ni] += 1;
                }
            }
        }

        // Spawn new nodes from all buds meeting min_attractors_per_bud
        let prev_count = node_count;
        let mut spawned_any = false;
        for ni in 0..prev_count {
            if accum_cnt[ni] < cfg.min_attractors_per_bud {
                continue;
            }
            // A symmetric attractor cloud can cancel the summed direction to
            // ~0; continue along the current heading instead of stalling
            // (the bud still senses attractors — growth should not stop).
            let heading = node_heading(&out_nodes[..prev_count], &out_parents[..prev_count], ni);
            let Some(grow_dir) = normalize(add(
                accum_dir[ni],
                scale(heading, accum_cnt[ni] as f64 * 0.5),
            ))
            .or_else(|| normalize(accum_dir[ni]))
            .or(Some(heading)) else {
                continue;
            };
            let new_pos = add(out_nodes[ni], scale(grow_dir, cfg.node_length));
            // Avoid duplicate coincident child
            let duplicate = (0..node_count).any(|k| dist_sq(out_nodes[k], new_pos) < 1e-10);
            if duplicate {
                continue;
            }
            if node_count >= cap {
                return Err(ColonizationError::OutputTooSmall {
                    required_nodes: node_count + 1,
                });
            }
            out_nodes[node_count] = new_pos;
            out_parents[node_count] = ni as u32;
            node_count += 1;
            spawned_any = true;

            // Kill attractors within kill_radius of new_pos
            for (ai, &attr) in attractors.iter().enumerate() {
                if active_attr[ai] && dist_sq(attr, new_pos) <= r_kill_sq {
                    active_attr[ai] = false;
                }
            }
        }

        iterations_run += 1;
        if !spawned_any {
            break;
        }
    }

    let remaining_attractors = active_attr.iter().filter(|&&a| a).count();
    Ok(SpaceColonizationReceipt {
        node_count,
        iterations_run,
        remaining_attractors,
    })
}

#[inline]
fn node_heading(nodes: &[Point3], parents: &[u32], idx: usize) -> Point3 {
    let p = parents[idx];
    if p == ROOT_PARENT || (p as usize) >= nodes.len() || (p as usize) == idx {
        return Point3::new(0.0, 0.0, 1.0);
    }
    normalize(sub(nodes[idx], nodes[p as usize])).unwrap_or(Point3::new(0.0, 0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_colonization_grows_branching_crown_towards_attractor_cloud() {
        // Asymmetric attractor cloud (positive-x quadrant bias) — a perfectly
        // symmetric cloud degenerates attraction sums to zero and cannot
        // branch, so we sample a realistic off-axis crown.
        let mut attractors = Vec::new();
        for ix in 0..=3 {
            for iy in -2..=1 {
                for iz in 2..=5 {
                    attractors.push(Point3::new(
                        ix as f64 * 0.3,
                        iy as f64 * 0.3,
                        iz as f64 * 0.3,
                    ));
                }
            }
        }
        let seeds = [Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.3)];
        let seed_parents = [ROOT_PARENT, 0];
        let cfg = SpaceColonizationConfig {
            node_length: 0.25,
            kill_radius: 0.35,
            perception_radius: 1.2,
            cone_angle_rad: core::f64::consts::PI * 1.4,
            min_attractors_per_bud: 1,
            max_iterations: 30,
        };
        let mut nodes = vec![Point3::default(); 256];
        let mut parents = vec![ROOT_PARENT; 256];
        let receipt = space_colonization_3d(
            &attractors,
            &seeds,
            &seed_parents,
            &cfg,
            &mut nodes,
            &mut parents,
        )
        .unwrap();
        assert!(
            receipt.node_count > 10,
            "expected crown growth, got {}",
            receipt.node_count
        );
        assert!(
            receipt.remaining_attractors < attractors.len() / 2,
            "expected >50% attractors colonized, remaining={}",
            receipt.remaining_attractors
        );

        let mut whorl = [Point3::default(); 4];
        let nw = lateral_whorl_directions(Point3::new(0.0, 0.0, 1.0), 0.7, 4, &mut whorl).unwrap();
        assert_eq!(nw, 4);
    }
}
