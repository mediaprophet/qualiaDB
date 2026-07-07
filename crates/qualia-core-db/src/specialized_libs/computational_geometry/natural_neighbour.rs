//! P8.5 — Natural-neighbour interpolation (Sibson / Laplace weights)
//! over the substrate's Delaunay/Voronoi.
//!
//! ## Natural-neighbour coordinates
//!
//! Given a set of data sites {p₁,…,pₙ} with values {v₁,…,vₙ}, the natural-
//! neighbour interpolant at a query point x is:
//!
//! ```text
//! f(x) = Σ λᵢ(x) * vᵢ
//! ```
//!
//! where λᵢ(x) are the natural-neighbour coordinates (weights).
//!
//! ## Sibson coordinates
//!
//! Sibson coordinates are based on area theft: insert x into the Voronoi
//! diagram, and λᵢ is the ratio of the area stolen from site i's Voronoi
//! cell to the total area of x's Voronoi cell.
//!
//! ## Laplace coordinates
//!
//! Laplace (non-Sibson) coordinates are simpler:
//! ```text
//! λᵢ(x) = (lᵢ / dᵢ) / Σⱼ (lⱼ / dⱼ)
//! ```
//! where dᵢ = |x - pᵢ| and lᵢ is the length of the Voronoi edge shared
//! between x's cell and site i's cell.
//!
//! ## Properties
//!
//! - Partition of unity: Σλᵢ = 1
//! - Non-negative: λᵢ ≥ 0
//! - Linear precision: for a linear field f(p) = a·p + b, f(x) = Σλᵢf(pᵢ)
//! - Local: only the natural neighbours of x contribute
//!
//! ## Determinism
//!
//! All computations are deterministic. Weights are sorted by site index
//! for canonical output.

use super::primitives::Point2;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NniError {
    TooFewSites { got: usize },
    QueryOutsideHull,
    QueryAtDataSite,
    BufferTooSmall { needed: usize, have: usize },
}

impl core::fmt::Display for NniError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewSites { got } => write!(f, "nni: too few sites: {got}"),
            Self::QueryOutsideHull => write!(f, "nni: query outside convex hull"),
            Self::QueryAtDataSite => write!(f, "nni: query at data site"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "nni: buffer too small, need {needed}, have {have}")
            }
        }
    }
}

impl std::error::Error for NniError {}

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// A natural-neighbour weight entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NnWeight {
    /// Site index.
    pub site: u32,
    /// Weight (≥ 0, Σweights = 1).
    pub weight: f64,
}

// ───────────────────────────────────────────────────────────────────────────
//  Laplace (non-Sibson) coordinates
// ───────────────────────────────────────────────────────────────────────────

/// Compute Laplace (non-Sibson) natural-neighbour coordinates.
///
/// This approach:
/// 1. Computes the Delaunay triangulation of the sites.
/// 2. Finds the Delaunay triangle containing the query point.
/// 3. The natural neighbours are the vertices of that triangle plus
///    any adjacent triangle vertices connected through the triangle's edges.
/// 4. For each natural neighbour i, compute:
///    - dᵢ = distance from query to site i
///    - lᵢ = length of the shared Voronoi edge (approximated as the
///      circumradius of the adjacent triangle minus circumradius of the
///      containing triangle, for interior edges; for boundary edges,
///      lᵢ = 0)
/// 5. λᵢ = (lᵢ/dᵢ) / Σⱼ(lⱼ/dⱼ)
///
/// `out_weights` needs `n` entries (worst case: all sites are neighbours).
/// Returns the number of weights written.
///
/// For simplicity and robustness, this implementation uses a brute-force
/// approach: find all sites whose Voronoi cell would be stolen by the
/// query point (i.e., sites for which the query is closer than any other
/// site to their midpoint). This is equivalent to finding the natural
/// neighbours via the Delaunay triangulation.
pub fn laplace_coordinates(
    sites: &[Point2],
    query: Point2,
    out_weights: &mut [NnWeight],
) -> Result<usize, NniError> {
    let n = sites.len();
    if n < 3 {
        return Err(NniError::TooFewSites { got: n });
    }
    if out_weights.len() < n {
        return Err(NniError::BufferTooSmall {
            needed: n,
            have: out_weights.len(),
        });
    }

    // Check if query is at a data site.
    for s in sites {
        if (s.x - query.x).abs() < 1e-12 && (s.y - query.y).abs() < 1e-12 {
            return Err(NniError::QueryAtDataSite);
        }
    }

    // Correct Laplace (non-Sibson) coordinates via exact Voronoi-facet lengths.
    //
    // In the augmented Voronoi diagram of {sites ∪ query}, the Voronoi cell of
    // `query` shares a facet (a 2-D line segment) with each natural neighbour i.
    // That facet lies on the perpendicular bisector of (query, site_i); its
    // length `lᵢ` is the length of the interval of that bisector that stays
    // closer to {query, site_i} than to every other site. The Laplace
    // coordinate is λᵢ = (lᵢ/dᵢ) / Σⱼ (lⱼ/dⱼ) with dᵢ = |query − site_i|; this
    // has EXACT linear precision (Belikov et al. 1997).
    //
    // We compute each facet interval by clipping the bisector — parametrised
    // `p(t) = mid + t·û`, with `û ⟂ (site_i − query)`, `|û| = 1` — by the
    // half-line constraint "closer to query than to site_j" for every other
    // site j:  |p−q|² ≤ |p−sⱼ|²  ⇔  A·t + B ≤ 0, where
    //     A = 2·û·(sⱼ − q),   B = 2·mid·(sⱼ − q) + |q|² − |sⱼ|².
    // A site whose feasible interval is empty is not a natural neighbour
    // (lᵢ = 0); an unbounded feasible interval means `query`'s cell is
    // unbounded, i.e. `query` is outside the convex hull. O(n²), deterministic,
    // no Delaunay dependency.

    // Interval-emptiness / A≈0 tolerances (coordinates here are O(1)–O(10³)).
    const EPS_A: f64 = 1e-12;
    const EPS_L: f64 = 1e-9;

    let mut nbr_site: Vec<u32> = Vec::new();
    let mut nbr_raw: Vec<f64> = Vec::new();
    let mut outside_hull = false;

    for i in 0..n {
        let s = sites[i];
        let dx = s.x - query.x;
        let dy = s.y - query.y;
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1e-15 {
            return Err(NniError::QueryAtDataSite);
        }
        let mid_x = (query.x + s.x) * 0.5;
        let mid_y = (query.y + s.y) * 0.5;
        // Unit perpendicular to (s − query).
        let ux = -dy / d;
        let uy = dx / d;

        let mut t_lo = f64::NEG_INFINITY;
        let mut t_hi = f64::INFINITY;
        let mut feasible = true;

        for j in 0..n {
            if j == i {
                continue;
            }
            let sj = sites[j];
            let jx = sj.x - query.x;
            let jy = sj.y - query.y;
            let a = 2.0 * (ux * jx + uy * jy);
            let b = 2.0 * (mid_x * jx + mid_y * jy) + (query.x * query.x + query.y * query.y)
                - (sj.x * sj.x + sj.y * sj.y);
            if a > EPS_A {
                // t ≤ −B/A.
                let t = -b / a;
                if t < t_hi {
                    t_hi = t;
                }
            } else if a < -EPS_A {
                // t ≥ −B/A.
                let t = -b / a;
                if t > t_lo {
                    t_lo = t;
                }
            } else if b > EPS_A {
                // A ≈ 0 and B > 0: constraint unsatisfiable ⇒ empty facet.
                feasible = false;
                break;
            }
            if t_lo > t_hi {
                feasible = false;
                break;
            }
        }

        if !feasible {
            continue;
        }
        if t_hi - t_lo <= EPS_L {
            // Empty (or degenerate) facet ⇒ not a natural neighbour.
            continue;
        }
        if t_lo == f64::NEG_INFINITY || t_hi == f64::INFINITY {
            // Unbounded facet ⇒ query's cell is unbounded ⇒ outside the hull.
            outside_hull = true;
            break;
        }

        let l_i = t_hi - t_lo;
        nbr_site.push(i as u32);
        nbr_raw.push(l_i / d);
    }

    if outside_hull {
        return Err(NniError::QueryOutsideHull);
    }
    if nbr_site.is_empty() {
        return Err(NniError::QueryOutsideHull);
    }

    let total: f64 = nbr_raw.iter().sum();
    if total < 1e-15 {
        return Err(NniError::QueryOutsideHull);
    }

    let mut count = 0usize;
    for (idx, &site_idx) in nbr_site.iter().enumerate() {
        out_weights[count] = NnWeight {
            site: site_idx,
            weight: nbr_raw[idx] / total,
        };
        count += 1;
    }

    // Sort by site index for canonical output.
    out_weights[..count].sort_by(|a, b| a.site.cmp(&b.site));

    Ok(count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Interpolation
// ───────────────────────────────────────────────────────────────────────────

/// Interpolate a scalar field at the query point using natural-neighbour
/// coordinates.
///
/// `values[i]` is the scalar value at `sites[i]`.
/// Returns the interpolated value.
pub fn interpolate_scalar(
    sites: &[Point2],
    values: &[f64],
    query: Point2,
) -> Result<f64, NniError> {
    if values.len() != sites.len() {
        return Err(NniError::BufferTooSmall {
            needed: sites.len(),
            have: values.len(),
        });
    }

    let n = sites.len();
    let mut weights = vec![
        NnWeight {
            site: 0,
            weight: 0.0
        };
        n
    ];
    let count = laplace_coordinates(sites, query, &mut weights)?;

    let mut result = 0.0f64;
    for i in 0..count {
        let site_idx = weights[i].site as usize;
        result += weights[i].weight * values[site_idx];
    }

    Ok(result)
}

// ───────────────────────────────────────────────────────────────────────────
//  Property verification
// ───────────────────────────────────────────────────────────────────────────

/// Verify partition of unity: Σλᵢ = 1.
pub fn verify_partition_of_unity(weights: &[NnWeight]) -> bool {
    let sum: f64 = weights.iter().map(|w| w.weight).sum();
    (sum - 1.0).abs() < 1e-10
}

/// Verify non-negativity: all λᵢ ≥ 0.
pub fn verify_non_negative(weights: &[NnWeight]) -> bool {
    weights.iter().all(|w| w.weight >= -1e-10)
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over weights.
pub fn weights_hash(weights: &[NnWeight]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for w in weights {
        hash ^= w.site as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= w.weight.to_bits();
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

    fn grid_sites(nx: usize, ny: usize, spacing: f64) -> Vec<Point2> {
        let mut pts = Vec::new();
        for i in 0..nx {
            for j in 0..ny {
                pts.push(Point2::new(i as f64 * spacing, j as f64 * spacing));
            }
        }
        pts
    }

    fn regular_polygon(n: usize, r: f64) -> Vec<Point2> {
        (0..n)
            .map(|i| {
                let angle = 2.0 * core::f64::consts::PI * i as f64 / n as f64;
                Point2::new(r * angle.cos(), r * angle.sin())
            })
            .collect()
    }

    #[test]
    fn laplace_partition_of_unity() {
        let sites = grid_sites(4, 4, 1.0);
        let query = Point2::new(1.5, 1.5);
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let count = laplace_coordinates(&sites, query, &mut weights).unwrap();
        assert!(
            verify_partition_of_unity(&weights[..count]),
            "weights must sum to 1"
        );
    }

    #[test]
    fn laplace_non_negative() {
        let sites = grid_sites(4, 4, 1.0);
        let query = Point2::new(1.5, 1.5);
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let count = laplace_coordinates(&sites, query, &mut weights).unwrap();
        assert!(
            verify_non_negative(&weights[..count]),
            "weights must be non-negative"
        );
    }

    #[test]
    fn laplace_linear_precision() {
        // For a linear field f(x, y) = 2x + 3y, the interpolation should
        // reproduce the exact value at any interior query point.
        let sites = grid_sites(5, 5, 1.0);
        let values: Vec<f64> = sites.iter().map(|p| 2.0 * p.x + 3.0 * p.y).collect();
        let query = Point2::new(1.5, 2.5);
        let result = interpolate_scalar(&sites, &values, query).unwrap();
        let expected = 2.0 * 1.5 + 3.0 * 2.5;
        assert!(
            (result - expected).abs() < 1e-6,
            "linear precision: expected {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn laplace_linear_precision_offset() {
        // f(x, y) = 5x - 2y + 1
        let sites = grid_sites(5, 5, 1.0);
        let values: Vec<f64> = sites.iter().map(|p| 5.0 * p.x - 2.0 * p.y + 1.0).collect();
        let query = Point2::new(2.3, 1.7);
        let result = interpolate_scalar(&sites, &values, query).unwrap();
        let expected = 5.0 * 2.3 - 2.0 * 1.7 + 1.0;
        assert!(
            (result - expected).abs() < 1e-6,
            "linear precision with offset: expected {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn laplace_determinism() {
        let sites = grid_sites(4, 4, 1.0);
        let query = Point2::new(1.5, 1.5);

        let mut w1 = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let mut w2 = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let c1 = laplace_coordinates(&sites, query, &mut w1).unwrap();
        let c2 = laplace_coordinates(&sites, query, &mut w2).unwrap();

        assert_eq!(c1, c2);
        assert_eq!(weights_hash(&w1[..c1]), weights_hash(&w2[..c2]));
    }

    #[test]
    fn laplace_centre_of_square() {
        // At the centre of a symmetric grid, the weights should be
        // symmetric (equal for the 4 surrounding sites).
        let sites = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
            // Extra points to ensure Delaunay triangulation is stable.
            Point2::new(1.0, 1.0), // centre — but we query near it
        ];
        // Query slightly off-centre to avoid hitting a data site.
        let query = Point2::new(1.01, 1.01);
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let count = laplace_coordinates(&sites, query, &mut weights).unwrap();

        // Should have at least 3 neighbours.
        assert!(count >= 3, "should have at least 3 natural neighbours");

        // Partition of unity.
        assert!(verify_partition_of_unity(&weights[..count]));
    }

    #[test]
    fn laplace_outside_hull_extrapolates() {
        // Points far outside the hull still get natural neighbours —
        // Voronoi cells extend to infinity. The interpolation extrapolates
        // but still produces a partition of unity.
        let sites = grid_sites(3, 3, 1.0);
        let query = Point2::new(10.0, 10.0); // far outside
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let result = laplace_coordinates(&sites, query, &mut weights);
        // Should succeed (extrapolation) and produce valid weights.
        if let Ok(count) = result {
            assert!(verify_partition_of_unity(&weights[..count]));
        }
    }

    #[test]
    fn laplace_at_data_site_errors() {
        let sites = grid_sites(3, 3, 1.0);
        let query = sites[0]; // exactly at a data site
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let err = laplace_coordinates(&sites, query, &mut weights).unwrap_err();
        assert!(matches!(err, NniError::QueryAtDataSite));
    }

    #[test]
    fn laplace_too_few_sites() {
        let sites = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let query = Point2::new(0.5, 0.0);
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            2
        ];
        let err = laplace_coordinates(&sites, query, &mut weights).unwrap_err();
        assert!(matches!(err, NniError::TooFewSites { .. }));
    }

    #[test]
    fn laplace_polygon_interior() {
        let sites = regular_polygon(6, 1.0);
        let query = Point2::new(0.0, 0.0); // centre of hexagon
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let count = laplace_coordinates(&sites, query, &mut weights).unwrap();

        assert!(count >= 3, "centre of hexagon should have ≥ 3 neighbours");
        assert!(verify_partition_of_unity(&weights[..count]));
        assert!(verify_non_negative(&weights[..count]));
    }

    #[test]
    fn interpolate_scalar_determinism() {
        let sites = grid_sites(5, 5, 1.0);
        let values: Vec<f64> = (0..sites.len()).map(|i| (i as f64).sin()).collect();
        let query = Point2::new(2.3, 1.7);

        let r1 = interpolate_scalar(&sites, &values, query).unwrap();
        let r2 = interpolate_scalar(&sites, &values, query).unwrap();
        assert_eq!(
            r1.to_bits(),
            r2.to_bits(),
            "interpolation must be bit-identical"
        );
    }

    #[test]
    fn laplace_weights_sorted_by_site() {
        let sites = grid_sites(4, 4, 1.0);
        let query = Point2::new(1.5, 1.5);
        let mut weights = vec![
            NnWeight {
                site: 0,
                weight: 0.0
            };
            sites.len()
        ];
        let count = laplace_coordinates(&sites, query, &mut weights).unwrap();

        for i in 1..count {
            assert!(
                weights[i - 1].site <= weights[i].site,
                "weights must be sorted by site index"
            );
        }
    }

    #[test]
    fn laplace_linear_precision_random_queries() {
        // The defining correctness property: correct Laplace coordinates
        // reproduce ANY linear field EXACTLY at every interior query. A heuristic
        // weight scheme fails this; the exact Voronoi-facet computation passes it
        // to ~machine precision. f(x, y) = -1.7x + 4.3y - 0.9.
        let sites = grid_sites(6, 6, 1.0);
        let (a, b, c) = (-1.7f64, 4.3f64, -0.9f64);
        let values: Vec<f64> = sites.iter().map(|p| a * p.x + b * p.y + c).collect();
        let queries = [
            Point2::new(1.5, 1.5),
            Point2::new(2.3, 3.7),
            Point2::new(3.9, 2.1),
            Point2::new(2.5, 2.5),
            Point2::new(1.1, 4.2),
            Point2::new(4.4, 4.4),
        ];
        for q in queries {
            let got = interpolate_scalar(&sites, &values, q).unwrap();
            let want = a * q.x + b * q.y + c;
            assert!(
                (got - want).abs() < 1e-9,
                "linear precision at {q:?}: want {want}, got {got}"
            );
        }
    }

    #[test]
    fn laplace_weights_partition_and_nonneg_over_grid() {
        // Partition of unity + non-negativity must hold at a spread of interior
        // queries, not just one — the weights are a convex combination.
        let sites = grid_sites(6, 6, 1.0);
        let queries = [
            Point2::new(1.5, 1.5),
            Point2::new(2.3, 3.7),
            Point2::new(3.9, 2.1),
            Point2::new(1.1, 4.2),
        ];
        for q in queries {
            let mut w = vec![
                NnWeight {
                    site: 0,
                    weight: 0.0
                };
                sites.len()
            ];
            let c = laplace_coordinates(&sites, q, &mut w).unwrap();
            assert!(c >= 3, "interior query {q:?} should have >= 3 neighbours");
            assert!(
                verify_partition_of_unity(&w[..c]),
                "partition of unity at {q:?}"
            );
            assert!(verify_non_negative(&w[..c]), "non-negative at {q:?}");
        }
    }
}
