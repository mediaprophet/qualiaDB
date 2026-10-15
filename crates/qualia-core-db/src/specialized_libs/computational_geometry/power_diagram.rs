//! P21.4 — Laguerre–Voronoi (power diagrams) & regular triangulations in 2-D.
//!
//! Each site carries a weight `w` interpreted as squared radius; the power
//! distance is `pow(x, s) = |x − s|² − w`. The regular triangulation is the
//! weighted Delaunay triangulation of the lifted sites
//! `s⁺ = (x, y, x² + y² − w)`; its straight-line dual is the power diagram —
//! the correct cell decomposition for biological tissues whose cells have
//! unequal volumes (meristems, packed tissues).
//!
//! The core predicate [`power_test_2d`] is the lifted in-circle determinant.
//! Bowyer–Watson assembly is `ColdBounded`.

use super::expansion::Sign;
use super::orient3d::orient_3d;
use super::primitives::{Point2, Point3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedSite2d {
    pub pos: Point2,
    /// Squared cell radius; must satisfy `w ≥ 0`.
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerDiagramError {
    TooFewSites { got: usize },
    InvalidWeight { index: usize },
    OutputTooSmall { required: usize },
}

#[inline]
fn lifted(p: Point2, w: f64) -> f64 {
    p.x * p.x + p.y * p.y - w
}

/// Power-test (weighted in-circle) predicate: sign of the 4×4 lifted
/// determinant — the sign of `pow(p, ○abc)` where `○abc` is the ortho-circle
/// of three ccw-oriented weighted sites. `> 0` → `p` is inside the
/// ortho-circle (conflict); `< 0` outside; `0` cocircular.
///
/// Tier-1 `HotZeroHeap`.
pub fn power_test_2d(
    a: WeightedSite2d,
    b: WeightedSite2d,
    c: WeightedSite2d,
    p: WeightedSite2d,
) -> f64 {
    let az = lifted(a.pos, a.weight) - lifted(p.pos, p.weight);
    let bz = lifted(b.pos, b.weight) - lifted(p.pos, p.weight);
    let cz = lifted(c.pos, c.weight) - lifted(p.pos, p.weight);
    let (ax, ay) = (a.pos.x - p.pos.x, a.pos.y - p.pos.y);
    let (bx, by) = (b.pos.x - p.pos.x, b.pos.y - p.pos.y);
    let (cx, cy) = (c.pos.x - p.pos.x, c.pos.y - p.pos.y);
    // Same 3×3 sign structure as in_circle: det of [dx, dy, dz].
    ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx)
}

/// Sign of the 4×4 lifted power determinant via the exact 3-D orientation
/// predicate [`super::orient3d::orient_3d`].
///
/// Escalates through filtered → compensated → exact multi-precision expansion
/// arithmetic to resolve near-cocircular site configurations without topological
/// ambiguity.
///
/// Returns [`Sign::Positive`] if `p` is in conflict (inside the ortho-circle),
/// [`Sign::Negative`] if outside, or [`Sign::Zero`] if cocircular.
///
/// Tier-1 `HotZeroHeap`.
pub fn power_test_2d_sign(
    a: WeightedSite2d,
    b: WeightedSite2d,
    c: WeightedSite2d,
    p: WeightedSite2d,
) -> Sign {
    let p3 = Point3::new(p.pos.x, p.pos.y, lifted(p.pos, p.weight));
    let a3 = Point3::new(a.pos.x, a.pos.y, lifted(a.pos, a.weight));
    let b3 = Point3::new(b.pos.x, b.pos.y, lifted(b.pos, b.weight));
    let c3 = Point3::new(c.pos.x, c.pos.y, lifted(c.pos, c.weight));
    orient_3d(p3, a3, b3, c3)
}

/// Signed 2× orientation of unweighted positions — canonical ccw check used
/// to orient the lifted determinant sign.
#[inline]
fn orient2(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Weighted circumcenter (radical center) of three sites — the dual power-
/// diagram vertex. Returns `None` for degenerate (collinear) sites.
///
/// Tier-1 `HotZeroHeap`.
pub fn power_cell_vertex_2d(
    a: WeightedSite2d,
    b: WeightedSite2d,
    c: WeightedSite2d,
) -> Option<Point2> {
    let d = 2.0 * orient2(a.pos, b.pos, c.pos);
    if d.abs() < 1e-14 {
        return None;
    }
    let (a2, b2, c2) = (
        lifted(a.pos, a.weight),
        lifted(b.pos, b.weight),
        lifted(c.pos, c.weight),
    );
    let ux = (a2 * (b.pos.y - c.pos.y) + b2 * (c.pos.y - a.pos.y) + c2 * (a.pos.y - b.pos.y)) / d;
    let uy = (a2 * (c.pos.x - b.pos.x) + b2 * (a.pos.x - c.pos.x) + c2 * (b.pos.x - a.pos.x)) / d;
    Some(Point2::new(ux, uy))
}

/// Regular (weighted Delaunay) triangulation of `sites` via incremental
/// Bowyer–Watson cavity insertion in canonical index order.
///
/// `out_triangles` receives indices into `sites`; `out_dual` receives the
/// power-diagram vertex (weighted circumcenter) per output triangle.
/// Deterministic: sites insert in index order, cavity boundary collected in
/// scan order.
///
/// `ColdBounded` (internal cavity stack).
pub fn regular_triangulation_2d(
    sites: &[WeightedSite2d],
    out_triangles: &mut [[u32; 3]],
    out_dual: &mut [Point2],
) -> Result<usize, PowerDiagramError> {
    let n = sites.len();
    if n < 3 {
        return Err(PowerDiagramError::TooFewSites { got: n });
    }
    for (i, s) in sites.iter().enumerate() {
        if !(s.weight >= 0.0 && s.weight.is_finite()) {
            return Err(PowerDiagramError::InvalidWeight { index: i });
        }
    }
    // Supertriangle far outside the site bbox (weight 0).
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for s in sites {
        lo.x = lo.x.min(s.pos.x);
        lo.y = lo.y.min(s.pos.y);
        hi.x = hi.x.max(s.pos.x);
        hi.y = hi.y.max(s.pos.y);
    }
    let span = (hi.x - lo.x).max(hi.y - lo.y).max(1.0) * 16.0;
    let mid = Point2::new((lo.x + hi.x) * 0.5, (lo.y + hi.y) * 0.5);
    // Supertriangle far outside the site bbox, given a tiny positive weight
    // `eps` so no real site's ortho-circle exactly contains all three
    // super-vertices (breaks the classic cocircular-supervertex degeneracy
    // while leaving real-site predicates untouched).
    let eps = span * 1e-3;
    let ext = vec![
        WeightedSite2d {
            pos: Point2::new(mid.x - span, mid.y - span),
            weight: eps,
        },
        WeightedSite2d {
            pos: Point2::new(mid.x + span, mid.y - span),
            weight: eps,
        },
        WeightedSite2d {
            pos: Point2::new(mid.x, mid.y + span * 2.0),
            weight: eps,
        },
    ];
    let site_at = |i: usize| -> WeightedSite2d {
        if i < n {
            sites[i]
        } else {
            ext[i - n]
        }
    };

    let mut tris: Vec<[u32; 3]> = vec![[n as u32, n as u32 + 1, n as u32 + 2]];
    let mut cavity_edges: Vec<(u32, u32)> = Vec::new();
    for i in 0..n {
        let p = sites[i];
        cavity_edges.clear();
        tris.retain(|t| {
            let (a, b, c) = (
                site_at(t[0] as usize),
                site_at(t[1] as usize),
                site_at(t[2] as usize),
            );
            let inside = power_test_2d_sign(a, b, c, p) == Sign::Positive;
            if inside {
                for e in 0..3 {
                    let (u, v) = (t[e], t[(e + 1) % 3]);
                    if let Some(pos) = cavity_edges.iter().position(|&(a2, b2)| a2 == v && b2 == u)
                    {
                        cavity_edges.remove(pos);
                    } else {
                        cavity_edges.push((u, v));
                    }
                }
            }
            !inside
        });
        for &(u, v) in &cavity_edges {
            tris.push([u, v, i as u32]);
        }
    }

    // Drop triangles touching supertriangle vertices; orient ccw.
    let mut kept = 0usize;
    for t in tris.iter().copied() {
        if (t[0] as usize) >= n || (t[1] as usize) >= n || (t[2] as usize) >= n {
            continue;
        }
        let mut t = t;
        if orient2(
            sites[t[0] as usize].pos,
            sites[t[1] as usize].pos,
            sites[t[2] as usize].pos,
        ) < 0.0
        {
            t.swap(1, 2);
        }
        if kept >= out_triangles.len() || kept >= out_dual.len() {
            return Err(PowerDiagramError::OutputTooSmall { required: kept + 1 });
        }
        out_triangles[kept] = t;
        out_dual[kept] = power_cell_vertex_2d(
            sites[t[0] as usize],
            sites[t[1] as usize],
            sites[t[2] as usize],
        )
        .unwrap_or(Point2::new(f64::NAN, f64::NAN));
        kept += 1;
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_weights_reduce_to_delaunay() {
        let sites = [
            WeightedSite2d {
                pos: Point2::new(0.0, 0.0),
                weight: 0.0,
            },
            WeightedSite2d {
                pos: Point2::new(1.0, 0.0),
                weight: 0.0,
            },
            WeightedSite2d {
                pos: Point2::new(1.0, 1.0),
                weight: 0.0,
            },
            WeightedSite2d {
                pos: Point2::new(0.0, 1.0),
                weight: 0.0,
            },
        ];
        let mut tris = [[0u32; 3]; 8];
        let mut dual = [Point2::default(); 8];
        let n = regular_triangulation_2d(&sites, &mut tris, &mut dual).unwrap();
        assert_eq!(n, 2, "square triangulates into 2 regular triangles");
        // Dual vertex of each triangle is its circumcenter (≈ center).
        for t in 0..n {
            let c = dual[t];
            assert!((c.x - 0.5).abs() < 0.25 && (c.y - 0.5).abs() < 0.25);
        }
    }

    #[test]
    fn heavy_weight_claims_larger_cell() {
        // Power bisector of a=(0,0,w=0) vs heavy=(1,0,w=4): the boundary
        // solves x² = (x−1)² − 4 → x = −1.5, so the heavy site's cell swallows
        // the midpoint and even a itself.
        let a = WeightedSite2d {
            pos: Point2::new(0.0, 0.0),
            weight: 0.0,
        };
        let heavy = WeightedSite2d {
            pos: Point2::new(1.0, 0.0),
            weight: 4.0,
        };
        let power = |p: Point2, s: WeightedSite2d| {
            let dx = p.x - s.pos.x;
            let dy = p.y - s.pos.y;
            dx * dx + dy * dy - s.weight
        };
        let mid = Point2::new(0.5, 0.0);
        assert!(
            power(mid, heavy) < power(mid, a),
            "heavy site should own the midpoint"
        );
        assert!(
            power(a.pos, heavy) < 0.0,
            "a's own position is inside heavy's cell"
        );
        // Equal weights → perpendicular bisector at x=0.5 (sanity).
        let b = WeightedSite2d {
            pos: Point2::new(1.0, 0.0),
            weight: 0.0,
        };
        let d = power(mid, a) - power(mid, b);
        assert!(d.abs() < 1e-12, "equal weights bisect at the midpoint");
    }

    #[test]
    fn power_test_sign_matches_exact_orientation() {
        let a = WeightedSite2d {
            pos: Point2::new(0.0, 0.0),
            weight: 0.0,
        };
        let b = WeightedSite2d {
            pos: Point2::new(1.0, 0.0),
            weight: 0.0,
        };
        let c = WeightedSite2d {
            pos: Point2::new(0.0, 1.0),
            weight: 0.0,
        };
        // Inside ortho-circle of unit right triangle
        let p_in = WeightedSite2d {
            pos: Point2::new(0.2, 0.2),
            weight: 0.0,
        };
        assert_eq!(power_test_2d_sign(a, b, c, p_in), Sign::Positive);
        assert!(power_test_2d(a, b, c, p_in) > 0.0);

        // Outside ortho-circle
        let p_out = WeightedSite2d {
            pos: Point2::new(2.0, 2.0),
            weight: 0.0,
        };
        assert_eq!(power_test_2d_sign(a, b, c, p_out), Sign::Negative);
        assert!(power_test_2d(a, b, c, p_out) < 0.0);

        // Cocircular boundary (hypotenuse midpoint (0.5, 0.5) is on circumcircle of (0,0),(1,0),(0,1))
        let p_on = WeightedSite2d {
            pos: Point2::new(1.0, 1.0),
            weight: 0.0,
        };
        assert_eq!(power_test_2d_sign(a, b, c, p_on), Sign::Zero);
    }
}

