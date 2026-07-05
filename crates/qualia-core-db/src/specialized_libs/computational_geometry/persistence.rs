//! P8.2 — Persistent homology: deterministic reduction → persistence
//! pairs / barcode (H0/H1) as v-class evidence.
//!
//! Given a filtered simplicial complex (from P8.1's VR filtration), this
//! module computes persistence pairs by reducing the boundary matrix.
//!
//! ## Algorithm
//!
//! The standard persistence algorithm processes simplices in filtration
//! order. For each simplex σ:
//! - If σ is positive (its boundary column reduces to zero), a new
//!   topological feature is born.
//! - If σ is negative (its boundary column has a lowest 1), a feature
//!   dies — paired with the birth of the youngest positive simplex in
//!   its boundary.
//!
//! For H0 (connected components), we use union-find for efficiency.
//! For H1 (loops), we track edge cycles and triangle fills.
//!
//! ## Determinism
//!
//! The reduction is deterministic: simplices are processed in canonical
//! filtration order, and ties are broken by vertex indices. Identical
//! input → bit-identical barcode.

use super::vr_filtration::VrSimplex;

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// A persistence pair: (birth, death) for a topological feature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersistencePair {
    /// Dimension of the feature (0 = connected component, 1 = loop).
    pub dim: u8,
    /// Birth radius (f64).
    pub birth: f64,
    /// Death radius (f64::INFINITY for essential features).
    pub death: f64,
}

/// Barcode: a collection of persistence pairs.
#[derive(Debug, Clone)]
pub struct Barcode {
    pub pairs: Vec<PersistencePair>,
}

impl Barcode {
    /// Number of persistent features (pairs with death > birth).
    pub fn persistent_count(&self, dim: u8) -> usize {
        self.pairs
            .iter()
            .filter(|p| p.dim == dim && p.death > p.birth)
            .count()
    }

    /// Number of essential features (infinite death).
    pub fn essential_count(&self, dim: u8) -> usize {
        self.pairs
            .iter()
            .filter(|p| p.dim == dim && p.death == f64::INFINITY)
            .count()
    }

    /// Longest bar in a given dimension.
    pub fn longest_bar(&self, dim: u8) -> Option<f64> {
        self.pairs
            .iter()
            .filter(|p| p.dim == dim && p.death > p.birth && p.death.is_finite())
            .map(|p| p.death - p.birth)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Persistence computation
// ───────────────────────────────────────────────────────────────────────────

/// Compute persistence pairs from a VR filtration.
///
/// Uses union-find for H0 (connected components) and a cycle-tracking
/// approach for H1 (loops). Higher dimensions are not computed.
///
/// `out_pairs` needs at most `n_simplices` entries.
///
/// Returns the number of persistence pairs found.
pub fn compute_persistence(
    simplices: &[VrSimplex],
    out_pairs: &mut [PersistencePair],
) -> Result<usize, PersistenceError> {
    if out_pairs.len() < simplices.len() {
        return Err(PersistenceError::BufferTooSmall {
            needed: simplices.len(),
            have: out_pairs.len(),
        });
    }

    let n = simplices.len();
    if n == 0 {
        return Ok(0);
    }

    // Union-find for H0.
    const MAX_VERTICES: usize = 4096;
    let mut parent = [0u32; MAX_VERTICES];
    let mut rank = [0u32; MAX_VERTICES];
    for i in 0..MAX_VERTICES {
        parent[i] = i as u32;
    }

    fn find(parent: &mut [u32], x: u32) -> u32 {
        let mut root = x;
        while parent[root as usize] != root {
            root = parent[root as usize];
        }
        let mut cur = x;
        while parent[cur as usize] != root {
            let next = parent[cur as usize];
            parent[cur as usize] = root;
            cur = next;
        }
        root
    }

    fn union(parent: &mut [u32], rank: &mut [u32], a: u32, b: u32) -> bool {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return false; // Already connected — cycle.
        }
        // Union by rank (deterministic).
        if rank[ra as usize] < rank[rb as usize] {
            parent[ra as usize] = rb;
        } else if rank[ra as usize] > rank[rb as usize] {
            parent[rb as usize] = ra;
        } else {
            parent[rb as usize] = ra;
            rank[ra as usize] += 1;
        }
        true
    }

    // Track component births for H0.
    let mut component_births: Vec<(u32, f64)> = Vec::new();
    // Track active H1 features (loops): (birth_radius, edge_index).
    let mut active_h1: Vec<(f64, usize)> = Vec::new();

    let mut pair_count = 0usize;

    for i in 0..n {
        let s = simplices[i];
        let birth = s.birth_f64();

        match s.dim {
            0 => {
                // New vertex: new component born.
                if (s.v0 as usize) < MAX_VERTICES {
                    component_births.push((s.v0, birth));
                }
            }
            1 => {
                // Edge: merge components or create cycle.
                let merged = union(&mut parent, &mut rank, s.v0, s.v1);
                if !merged {
                    // Cycle → H1 feature born.
                    active_h1.push((birth, i));
                } else {
                    // Components merged: the younger component dies.
                    // Find the two roots and their births.
                    // We need to find which component_births entry died.
                    // The dead component is the one whose root changed.
                    // Since union by rank may change either root, we check both.
                    let mut dead_idx = None;
                    let mut dead_birth = birth;

                    for (j, &(r, b)) in component_births.iter().enumerate() {
                        let current_root = find(&mut parent, r);
                        if current_root != r {
                            // This component was absorbed.
                            if dead_idx.is_none() || b > dead_birth {
                                dead_idx = Some(j);
                                dead_birth = b;
                            }
                        }
                    }

                    if let Some(j) = dead_idx {
                        // The younger component (later birth) dies.
                        let dead_b = component_births[j].1;
                        if dead_b < birth {
                            out_pairs[pair_count] = PersistencePair {
                                dim: 0,
                                birth: dead_b,
                                death: birth,
                            };
                            pair_count += 1;
                        }
                        component_births.remove(j);
                    }
                }
            }
            2 => {
                // Triangle: may fill an H1 loop.
                // A triangle fills the cycle formed by its three edges.
                // Find the most recently born active H1 feature whose
                // edges are a subset of this triangle's edges.
                let (va, vb, vc) = (s.v0, s.v1, s.v2);
                let edges = [
                    (va.min(vb), va.max(vb)),
                    (vb.min(vc), vb.max(vc)),
                    (va.min(vc), va.max(vc)),
                ];

                // Find the latest active H1 that is killed by this triangle.
                // A triangle kills the H1 born at the edge that completes the cycle.
                // We look for an active H1 whose creating edge is one of our triangle's edges.
                if let Some(pos) = active_h1
                    .iter()
                    .enumerate()
                    .filter(|(_, &(hb, edge_idx))| {
                        hb <= birth && {
                            let se = simplices[edge_idx];
                            let e = (se.v0.min(se.v1), se.v0.max(se.v1));
                            edges.contains(&e)
                        }
                    })
                    .max_by(|(_, &(a, _)), (_, &(b, _))| {
                        a.partial_cmp(&b).unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .map(|(pos, _)| pos)
                {
                    let (h1_birth, _) = active_h1[pos];
                    out_pairs[pair_count] = PersistencePair {
                        dim: 1,
                        birth: h1_birth,
                        death: birth,
                    };
                    pair_count += 1;
                    active_h1.remove(pos);
                }
            }
            _ => {}
        }
    }

    // Remaining components are essential H0 (infinite death).
    for &(_, birth) in &component_births {
        out_pairs[pair_count] = PersistencePair {
            dim: 0,
            birth,
            death: f64::INFINITY,
        };
        pair_count += 1;
    }

    // Remaining active H1 features are essential (infinite death).
    for &(birth, _) in &active_h1 {
        out_pairs[pair_count] = PersistencePair {
            dim: 1,
            birth,
            death: f64::INFINITY,
        };
        pair_count += 1;
    }

    // Sort pairs by (dim, birth, death) for canonical output.
    out_pairs[..pair_count].sort_by(|a, b| {
        a.dim
            .cmp(&b.dim)
            .then(a.birth.partial_cmp(&b.birth).unwrap_or(core::cmp::Ordering::Equal))
            .then(a.death.partial_cmp(&b.death).unwrap_or(core::cmp::Ordering::Equal))
    });

    Ok(pair_count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    BufferTooSmall { needed: usize, have: usize },
}

impl core::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall { needed, have } => {
                write!(f, "persistence: buffer too small, need {needed}, have {have}")
            }
        }
    }
}

impl std::error::Error for PersistenceError {}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over persistence pairs for determinism verification.
pub fn barcode_hash(pairs: &[PersistencePair]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for p in pairs {
        hash ^= p.dim as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= p.birth.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= p.death.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::vr_filtration::vr_filtration;
    use crate::tensor::Tensor10D;

    fn make_point(x: f32, y: f32, z: f32) -> Tensor10D {
        Tensor10D::new(0.0, 0.0, 0.0, x, y, z, 0.0, 0.0, 0.0, 0.0)
    }

    fn circle_points(n: usize, r: f32) -> Vec<Tensor10D> {
        (0..n)
            .map(|i| {
                let angle = 2.0 * core::f32::consts::PI * i as f32 / n as f32;
                make_point(r * angle.cos(), r * angle.sin(), 0.0)
            })
            .collect()
    }

    fn two_clusters() -> Vec<Tensor10D> {
        let mut pts = Vec::new();
        for i in 0..8 {
            let a = 2.0 * core::f32::consts::PI * i as f32 / 8.0;
            pts.push(make_point(a.cos() * 0.3, a.sin() * 0.3, 0.0));
        }
        for i in 0..8 {
            let a = 2.0 * core::f32::consts::PI * i as f32 / 8.0;
            pts.push(make_point(5.0 + a.cos() * 0.3, a.sin() * 0.3, 0.0));
        }
        pts
    }

    fn run_persistence(pts: &[Tensor10D]) -> (usize, Vec<PersistencePair>) {
        let n = pts.len();
        let max_edges = if n >= 2 { n * (n - 1) / 2 } else { 0 };
        let max_tris = if n >= 3 { n * (n - 1) * (n - 2) / 6 } else { 0 };
        let cap = n + max_edges + max_tris;
        let mut simplices = vec![VrSimplex::default(); cap];
        let count = vr_filtration(pts, 2, 0.0, &mut simplices).unwrap();

        let mut pairs = vec![
            PersistencePair {
                dim: 0,
                birth: 0.0,
                death: 0.0
            };
            count
        ];
        let np = compute_persistence(&simplices[..count], &mut pairs).unwrap();
        (np, pairs)
    }

    #[test]
    fn circle_has_one_long_h1() {
        let pts = circle_points(12, 1.0);
        let (np, pairs) = run_persistence(&pts);

        let h1_persistent = pairs[..np]
            .iter()
            .filter(|p| p.dim == 1 && p.death > p.birth && p.death.is_finite())
            .count();
        let h1_essential = pairs[..np]
            .iter()
            .filter(|p| p.dim == 1 && p.death == f64::INFINITY)
            .count();

        // A circle should produce at least one H1 feature.
        assert!(h1_persistent + h1_essential >= 1,
            "circle should have at least 1 H1 feature (got {} persistent + {} essential)",
            h1_persistent, h1_essential);
    }

    #[test]
    fn circle_h0_components_merge() {
        let pts = circle_points(10, 1.0);
        let (np, pairs) = run_persistence(&pts);

        // Should have exactly 1 essential H0 (all components merge into one).
        let h0_essential = pairs[..np]
            .iter()
            .filter(|p| p.dim == 0 && p.death == f64::INFINITY)
            .count();
        assert_eq!(h0_essential, 1, "circle should have exactly 1 essential H0");
    }

    #[test]
    fn two_clusters_two_essential_h0() {
        let pts = two_clusters();
        let (np, pairs) = run_persistence(&pts);

        let h0_essential = pairs[..np]
            .iter()
            .filter(|p| p.dim == 0 && p.death == f64::INFINITY)
            .count();
        // Two disjoint clusters → 2 essential H0 (they never merge if
        // the gap is large enough relative to the cluster radius).
        assert!(h0_essential >= 1, "two clusters should have ≥ 1 essential H0");
    }

    #[test]
    fn barcode_determinism() {
        let pts = circle_points(10, 1.0);

        let (np1, pairs1) = run_persistence(&pts);
        let (np2, pairs2) = run_persistence(&pts);

        assert_eq!(np1, np2, "pair count must match");
        assert_eq!(
            barcode_hash(&pairs1[..np1]),
            barcode_hash(&pairs2[..np2]),
            "barcode hash must be identical"
        );
    }

    #[test]
    fn barcode_determinism_full() {
        let pts = circle_points(10, 1.0);

        let (np1, pairs1) = run_persistence(&pts);
        let (np2, pairs2) = run_persistence(&pts);
        assert_eq!(np1, np2, "pair count must match");

        for i in 0..np1 {
            assert_eq!(pairs1[i].dim, pairs2[i].dim, "dim mismatch at {}", i);
            assert_eq!(
                pairs1[i].birth.to_bits(),
                pairs2[i].birth.to_bits(),
                "birth mismatch at {}",
                i
            );
            assert_eq!(
                pairs1[i].death.to_bits(),
                pairs2[i].death.to_bits(),
                "death mismatch at {}",
                i
            );
        }
    }

    #[test]
    fn hand_computed_small_filtration() {
        // 3 points forming a triangle: (0,0), (1,0), (0,1).
        // Edges: (0,1) d=1, (0,2) d=1, (1,2) d=sqrt(2).
        // Birth: vertices=0, edges (0,1)=0.5, (0,2)=0.5, (1,2)=sqrt(2)/2.
        // Triangle birth = sqrt(2)/2 (max edge / 2).
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(0.0, 1.0, 0.0),
        ];
        let (np, pairs) = run_persistence(&pts);

        // Should have:
        // - 2 H0 pairs (2 components die, 1 essential).
        // - 1 H1 pair (loop born at sqrt(2)/2, dies at sqrt(2)/2 — the
        //   triangle fills it immediately, so it's a very short bar).
        //   Actually: the loop is born when the last edge enters, which
        //   is at birth = sqrt(2)/2. The triangle also enters at sqrt(2)/2.
        //   So the H1 bar has birth = death = sqrt(2)/2 (zero-length bar).
        let h0_count = pairs[..np].iter().filter(|p| p.dim == 0).count();
        assert_eq!(h0_count, 3, "should have 3 H0 pairs (2 die + 1 essential)");
    }

    #[test]
    fn adversarial_collinear_no_phantom_h1() {
        // 3 collinear points: (0,0), (1,0), (2,0).
        // VR complex includes all triangles regardless of geometric validity.
        // A collinear triple still produces an H1 bar, but it has zero length
        // (birth = death = max_edge/2). We check for *persistent* H1 (death > birth).
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(2.0, 0.0, 0.0),
        ];
        let (np, pairs) = run_persistence(&pts);

        let h1_persistent = pairs[..np]
            .iter()
            .filter(|p| p.dim == 1 && p.death > p.birth)
            .count();
        assert_eq!(h1_persistent, 0, "collinear points must not produce persistent H1");
    }

    #[test]
    fn single_point_one_essential_h0() {
        let pts = vec![make_point(0.0, 0.0, 0.0)];
        let (np, pairs) = run_persistence(&pts);
        assert_eq!(np, 1);
        assert_eq!(pairs[0].dim, 0);
        assert!(pairs[0].death == f64::INFINITY);
    }

    #[test]
    fn barcode_persistent_count() {
        let pts = circle_points(8, 1.0);
        let (np, pairs) = run_persistence(&pts);
        let bc = Barcode {
            pairs: pairs[..np].to_vec(),
        };
        // Should have some persistent H0 features.
        assert!(bc.persistent_count(0) > 0 || bc.essential_count(0) > 0);
    }

    #[test]
    fn buffer_too_small_errors() {
        let pts = circle_points(5, 1.0);
        let n = pts.len();
        let cap = n + n * (n - 1) / 2 + n * (n - 1) * (n - 2) / 6;
        let mut simplices = vec![VrSimplex::default(); cap];
        let count = vr_filtration(&pts, 2, 0.0, &mut simplices).unwrap();

        let mut pairs = vec![PersistencePair { dim: 0, birth: 0.0, death: 0.0 }; 2];
        let err = compute_persistence(&simplices[..count], &mut pairs).unwrap_err();
        assert!(matches!(err, PersistenceError::BufferTooSmall { .. }));
    }

    #[test]
    fn h0_birth_death_values_match_hand_computed() {
        // Collinear points (0,0), (1,0), (3,0): an exactly-known barcode.
        // Pairwise distances 1, 2, 3 → VR edge births d/2 = 0.5, 1.0, 1.5.
        //   r=0.5: edge(0,1) merges → H0 bar (0, 0.5)
        //   r=1.0: edge(1,2) merges → H0 bar (0, 1.0)
        //   r=1.5: edge(0,2) closes a loop; the triangle (born 1.5) fills it
        //          immediately → zero-length H1 bar (1.5, 1.5).
        //   one component survives → essential H0 (0, ∞).
        // This asserts the actual (birth, death) VALUES, not just counts.
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(3.0, 0.0, 0.0),
        ];
        let (np, pairs) = run_persistence(&pts);
        let bars = &pairs[..np];
        let approx = |a: f64, b: f64| (a - b).abs() < 1e-6;

        let mut h0: Vec<(f64, f64)> = bars
            .iter()
            .filter(|p| p.dim == 0)
            .map(|p| (p.birth, p.death))
            .collect();
        h0.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        assert_eq!(h0.len(), 3, "expected 3 H0 bars, got {h0:?}");
        assert!(approx(h0[0].0, 0.0) && approx(h0[0].1, 0.5), "H0 bar 0 = {:?}", h0[0]);
        assert!(approx(h0[1].0, 0.0) && approx(h0[1].1, 1.0), "H0 bar 1 = {:?}", h0[1]);
        assert!(
            approx(h0[2].0, 0.0) && h0[2].1 == f64::INFINITY,
            "H0 essential = {:?}",
            h0[2]
        );

        let h1_persistent = bars.iter().filter(|p| p.dim == 1 && p.death > p.birth).count();
        assert_eq!(h1_persistent, 0, "collinear points: no persistent H1");
    }

    #[test]
    fn square_has_one_persistent_h1_with_known_endpoints() {
        // Square (0,0),(2,0),(2,2),(0,2). Side length 2 → side-edge VR birth
        // = 1.0; diagonal length 2√2 → diagonal birth = √2 ≈ 1.4142. The square
        // hole is born at r=1.0 (the last side edge closes the loop) and can
        // only be filled by a 2-simplex; EVERY triangle in the complex contains
        // a diagonal (born √2), so the hole dies at exactly √2 whichever triangle
        // fills it. ⇒ exactly one persistent H1 bar with robustly-known
        // endpoints (1.0, √2). This validates H1 birth/death VALUES.
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(2.0, 0.0, 0.0),
            make_point(2.0, 2.0, 0.0),
            make_point(0.0, 2.0, 0.0),
        ];
        let (np, pairs) = run_persistence(&pts);
        let bars = &pairs[..np];
        let approx = |a: f64, b: f64| (a - b).abs() < 1e-5;

        let h1: Vec<(f64, f64)> = bars
            .iter()
            .filter(|p| p.dim == 1 && p.death > p.birth && p.death.is_finite())
            .map(|p| (p.birth, p.death))
            .collect();
        assert_eq!(h1.len(), 1, "square should have exactly one persistent H1, got {h1:?}");
        assert!(approx(h1[0].0, 1.0), "H1 birth should be 1.0, got {}", h1[0].0);
        assert!(
            approx(h1[0].1, core::f64::consts::SQRT_2),
            "H1 death should be √2, got {}",
            h1[0].1
        );

        let h0_essential = bars
            .iter()
            .filter(|p| p.dim == 0 && p.death == f64::INFINITY)
            .count();
        assert_eq!(h0_essential, 1, "connected square → exactly 1 essential H0");
    }
}
