//! Membership functions for fuzzy `FILTER`s — map a crisp numeric value (e.g. an age,
//! a distance, a similarity score bound to a query variable) to a degree in `[0, 1]`.
//! These produce the per-solution degree that the [`super::solution`] algebra then
//! composes. They are query-layer primitives (value → degree); the *logic* operators
//! that combine degrees are reused from [`crate::modalities::fuzzy`].

#[inline]
fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Rising ramp: `0` at/below `a`, `1` at/above `b`, linear between (`a < b`). Use for
/// "at least about `b`" predicates. If `a >= b`, behaves as a crisp step at `a`.
pub fn ramp_up(x: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return if x >= a { 1.0 } else { 0.0 };
    }
    clamp01((x - a) / (b - a))
}

/// Falling ramp: `1` at/below `a`, `0` at/above `b` (`a < b`). "At most about `a`".
pub fn ramp_down(x: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return if x <= a { 1.0 } else { 0.0 };
    }
    clamp01((b - x) / (b - a))
}

/// Triangular membership: peak `1` at `m`, falling to `0` at `a` (left) and `b`
/// (right), `0` outside `[a, b]`. Requires `a <= m <= b`.
pub fn triangular(x: f64, a: f64, m: f64, b: f64) -> f64 {
    if x <= a || x >= b {
        0.0
    } else if (x - m).abs() < f64::EPSILON {
        1.0
    } else if x < m {
        clamp01((x - a) / (m - a))
    } else {
        clamp01((b - x) / (b - m))
    }
}

/// Trapezoidal membership: `0` below `a`, rising on `[a, b]`, plateau `1` on `[b, c]`,
/// falling on `[c, d]`, `0` above `d`. Requires `a <= b <= c <= d`.
pub fn trapezoidal(x: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    if x <= a || x >= d {
        0.0
    } else if x < b {
        clamp01((x - a) / (b - a))
    } else if x <= c {
        1.0
    } else {
        clamp01((d - x) / (d - c))
    }
}

/// "≈ `target`" with tolerance `tol`: a symmetric triangle peaking at `target`,
/// reaching `0` at `target ± tol`. `tol <= 0` is a crisp equality.
pub fn approximately(x: f64, target: f64, tol: f64) -> f64 {
    if tol <= 0.0 {
        return if (x - target).abs() < f64::EPSILON { 1.0 } else { 0.0 };
    }
    triangular(x, target - tol, target, target + tol)
}

/// "Much greater than `reference`": rises from `0` at `reference` to `1` at
/// `reference + spread`.
pub fn much_greater_than(x: f64, reference: f64, spread: f64) -> f64 {
    ramp_up(x, reference, reference + spread.max(0.0))
}

/// "Much less than `reference`": falls from `1` at `reference − spread` to `0` at
/// `reference`.
pub fn much_less_than(x: f64, reference: f64, spread: f64) -> f64 {
    ramp_down(x, reference - spread.max(0.0), reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn ramps_endpoints_and_interior() {
        assert!((ramp_up(0.0, 1.0, 3.0)).abs() < EPS);
        assert!((ramp_up(2.0, 1.0, 3.0) - 0.5).abs() < EPS);
        assert!((ramp_up(9.0, 1.0, 3.0) - 1.0).abs() < EPS);
        assert!((ramp_down(0.0, 1.0, 3.0) - 1.0).abs() < EPS);
        assert!((ramp_down(2.0, 1.0, 3.0) - 0.5).abs() < EPS);
    }

    #[test]
    fn triangle_peaks_at_m() {
        assert!((triangular(30.0, 20.0, 30.0, 40.0) - 1.0).abs() < EPS);
        assert!((triangular(25.0, 20.0, 30.0, 40.0) - 0.5).abs() < EPS);
        assert!(triangular(45.0, 20.0, 30.0, 40.0).abs() < EPS);
    }

    #[test]
    fn trapezoid_plateau() {
        assert!((trapezoidal(5.0, 1.0, 4.0, 6.0, 9.0) - 1.0).abs() < EPS); // on plateau
        assert!((trapezoidal(2.5, 1.0, 4.0, 6.0, 9.0) - 0.5).abs() < EPS); // rising
        assert!((trapezoidal(7.5, 1.0, 4.0, 6.0, 9.0) - 0.5).abs() < EPS); // falling
    }

    #[test]
    fn approximately_is_symmetric() {
        assert!((approximately(30.0, 30.0, 5.0) - 1.0).abs() < EPS);
        assert!((approximately(32.5, 30.0, 5.0) - 0.5).abs() < EPS);
        assert!((approximately(27.5, 30.0, 5.0) - 0.5).abs() < EPS);
        assert!(approximately(40.0, 30.0, 5.0).abs() < EPS);
    }

    #[test]
    fn comparators() {
        assert!((much_greater_than(100.0, 50.0, 50.0) - 1.0).abs() < EPS);
        assert!((much_greater_than(50.0, 50.0, 50.0)).abs() < EPS);
        assert!((much_less_than(0.0, 50.0, 50.0) - 1.0).abs() < EPS);
    }
}
