use core::cmp::Ordering;

use crate::tensor::Tensor10D;

use super::kernel::{FilteredF64Kernel, GeometryKernel};
use super::primitives::{Orientation, Point2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HullError {
    TooManyPoints,
    ScratchTooSmall { required: usize },
    OutputTooSmall { required: usize },
    NonFiniteCoordinate { index: usize },
}

#[inline]
fn point_cmp(a: (f64, f64), b: (f64, f64)) -> Ordering {
    a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1))
}

fn hull_indices_by<K, F>(
    kernel: &K,
    count: usize,
    xy: F,
    scratch: &mut [u32],
    out: &mut [u32],
) -> Result<usize, HullError>
where
    K: GeometryKernel,
    F: Fn(usize) -> (f64, f64) + Copy,
{
    if count > u32::MAX as usize {
        return Err(HullError::TooManyPoints);
    }
    let scratch_required = count.saturating_mul(3);
    if scratch.len() < scratch_required {
        return Err(HullError::ScratchTooSmall {
            required: scratch_required,
        });
    }
    if out.len() < count {
        return Err(HullError::OutputTooSmall { required: count });
    }
    if count == 0 {
        return Ok(0);
    }

    // Split: order (n for sorting) + stack (2n for Andrew's monotone chain).
    // The stack needs 2n because the upper hull phase can temporarily hold
    // up to 2n-1 entries (n from the lower hull + n-1 from the upper hull
    // before the cross-product check pops duplicates). With only n stack
    // slots, a convex polygon input (all points on hull) overflows.
    let (order, stack) = scratch[..scratch_required].split_at_mut(count);
    for (i, slot) in order.iter_mut().enumerate() {
        let p = xy(i);
        if !p.0.is_finite() || !p.1.is_finite() {
            return Err(HullError::NonFiniteCoordinate { index: i });
        }
        *slot = i as u32;
    }
    order.sort_unstable_by(|a, b| point_cmp(xy(*a as usize), xy(*b as usize)));

    // Deduplicate coincident input points in place.
    let mut unique = 1usize;
    for read in 1..count {
        if xy(order[read] as usize) != xy(order[unique - 1] as usize) {
            order[unique] = order[read];
            unique += 1;
        }
    }
    if unique == 1 {
        out[0] = order[0];
        return Ok(1);
    }

    let turn = |ia: u32, ib: u32, ic: u32| {
        let a = xy(ia as usize);
        let b = xy(ib as usize);
        let c = xy(ic as usize);
        kernel.orientation_2(
            Point2::new(a.0, a.1),
            Point2::new(b.0, b.1),
            Point2::new(c.0, c.1),
        )
    };

    let mut len = 0usize;
    for &index in order[..unique].iter() {
        while len >= 2
            && turn(stack[len - 2], stack[len - 1], index) != Orientation::CounterClockwise
        {
            len -= 1;
        }
        stack[len] = index;
        len += 1;
    }

    let lower_len = len;
    for &index in order[..unique - 1].iter().rev() {
        while len > lower_len
            && turn(stack[len - 2], stack[len - 1], index) != Orientation::CounterClockwise
        {
            len -= 1;
        }
        if index == order[0] {
            break;
        }
        stack[len] = index;
        len += 1;
    }
    out[..len].copy_from_slice(&stack[..len]);
    Ok(len)
}

/// Compute the CCW convex-hull vertex indices with no heap allocation, using
/// the default [`FilteredF64Kernel`].
///
/// `scratch` requires `3 * points.len()` entries and `out` requires
/// `points.len()` entries. Collinear interior points and duplicates are omitted.
pub fn convex_hull_indices_2(
    points: &[Point2],
    scratch: &mut [u32],
    out: &mut [u32],
) -> Result<usize, HullError> {
    convex_hull_indices_2_with_kernel(&FilteredF64Kernel::default(), points, scratch, out)
}

/// Kernel-generic variant of [`convex_hull_indices_2`] — the algorithm runs
/// unchanged over any [`GeometryKernel`] (filtered `f64` today, exact
/// arithmetic in P1.7). This is the seam where the kernel is swapped.
pub fn convex_hull_indices_2_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Point2],
    scratch: &mut [u32],
    out: &mut [u32],
) -> Result<usize, HullError> {
    hull_indices_by(
        kernel,
        points.len(),
        |i| (points[i].x, points[i].y),
        scratch,
        out,
    )
}

/// Compute CCW convex-hull points into a caller-owned output slice, using
/// the default [`FilteredF64Kernel`].
pub fn convex_hull_2(
    points: &[Point2],
    scratch: &mut [u32],
    out: &mut [Point2],
) -> Result<usize, HullError> {
    convex_hull_2_with_kernel(&FilteredF64Kernel::default(), points, scratch, out)
}

/// Kernel-generic variant of [`convex_hull_2`].
pub fn convex_hull_2_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Point2],
    scratch: &mut [u32],
    out: &mut [Point2],
) -> Result<usize, HullError> {
    if out.len() < points.len() {
        return Err(HullError::OutputTooSmall {
            required: points.len(),
        });
    }
    let required = points.len().saturating_mul(3);
    if scratch.len() < required {
        return Err(HullError::ScratchTooSmall { required });
    }
    // Split: order (n for sorting) + hull (2n for Andrew's monotone chain
    // stack — see hull_indices_by for the 2n requirement).
    let (order, hull) = scratch[..required].split_at_mut(points.len());
    let count = hull_indices_by_local(kernel, points, order, hull)?;
    for i in 0..count {
        out[i] = points[hull[i] as usize];
    }
    Ok(count)
}

fn hull_indices_by_local<K: GeometryKernel>(
    kernel: &K,
    points: &[Point2],
    order: &mut [u32],
    stack: &mut [u32],
) -> Result<usize, HullError> {
    let count = points.len();
    if count == 0 {
        return Ok(0);
    }
    for (i, slot) in order.iter_mut().enumerate() {
        let p = points[i];
        if !p.x.is_finite() || !p.y.is_finite() {
            return Err(HullError::NonFiniteCoordinate { index: i });
        }
        *slot = i as u32;
    }
    order.sort_unstable_by(|a, b| {
        point_cmp(
            (points[*a as usize].x, points[*a as usize].y),
            (points[*b as usize].x, points[*b as usize].y),
        )
    });
    let mut unique = 1usize;
    for read in 1..count {
        if points[order[read] as usize] != points[order[unique - 1] as usize] {
            order[unique] = order[read];
            unique += 1;
        }
    }
    if unique == 1 {
        stack[0] = order[0];
        return Ok(1);
    }
    let mut len = 0usize;
    for &index in order[..unique].iter() {
        while len >= 2
            && kernel.orientation_2(
                points[stack[len - 2] as usize],
                points[stack[len - 1] as usize],
                points[index as usize],
            ) != Orientation::CounterClockwise
        {
            len -= 1;
        }
        stack[len] = index;
        len += 1;
    }
    let lower_len = len;
    for &index in order[..unique - 1].iter().rev() {
        while len > lower_len
            && kernel.orientation_2(
                points[stack[len - 2] as usize],
                points[stack[len - 1] as usize],
                points[index as usize],
            ) != Orientation::CounterClockwise
        {
            len -= 1;
        }
        if index == order[0] {
            break;
        }
        stack[len] = index;
        len += 1;
    }
    Ok(len)
}

/// Convex hull of the spatial `(x,y)` projection of 10D manifold nodes, using
/// the default [`FilteredF64Kernel`].
///
/// The returned indices continue to address the original tensors, preserving
/// their q/v/w/t/spectral coordinates for graph and reasoning consumers.
pub fn convex_hull_tensor_xy(
    points: &[Tensor10D],
    scratch: &mut [u32],
    out: &mut [u32],
) -> Result<usize, HullError> {
    convex_hull_tensor_xy_with_kernel(&FilteredF64Kernel::default(), points, scratch, out)
}

/// Kernel-generic variant of [`convex_hull_tensor_xy`].
pub fn convex_hull_tensor_xy_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Tensor10D],
    scratch: &mut [u32],
    out: &mut [u32],
) -> Result<usize, HullError> {
    hull_indices_by(
        kernel,
        points.len(),
        |i| (points[i].x as f64, points[i].y as f64),
        scratch,
        out,
    )
}

/// Check that a polygon is CCW and strongly convex, using the default
/// [`FilteredF64Kernel`].
pub fn is_ccw_strongly_convex_2(points: &[Point2]) -> bool {
    is_ccw_strongly_convex_2_with_kernel(&FilteredF64Kernel::default(), points)
}

/// Kernel-generic variant of [`is_ccw_strongly_convex_2`].
pub fn is_ccw_strongly_convex_2_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Point2],
) -> bool {
    if points.len() < 3 {
        return false;
    }
    for i in 0..points.len() {
        if kernel.orientation_2(
            points[i],
            points[(i + 1) % points.len()],
            points[(i + 2) % points.len()],
        ) != Orientation::CounterClockwise
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_hull_omits_duplicate_and_interior_points() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 0.5),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.0, 0.0),
        ];
        let mut scratch = [0u32; 18];
        let mut out = [0u32; 6];
        let n = convex_hull_indices_2(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(n, 4);
        let hull: Vec<Point2> = out[..n]
            .iter()
            .map(|&index| points[index as usize])
            .collect();
        assert_eq!(
            hull,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ]
        );
        assert!(is_ccw_strongly_convex_2(&hull));
    }

    #[test]
    fn collinear_hull_contains_only_extremes() {
        let points = [
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
        ];
        let mut scratch = [0u32; 9];
        let mut out = [0u32; 3];
        let n = convex_hull_indices_2(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(points[out[0] as usize], Point2::new(0.0, 0.0));
        assert_eq!(points[out[1] as usize], Point2::new(2.0, 0.0));
    }

    #[test]
    fn ten_dimensional_hull_preserves_source_indices() {
        let mut points = [Tensor10D::default(); 5];
        for (point, xy) in
            points
                .iter_mut()
                .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]])
        {
            point.x = xy[0];
            point.y = xy[1];
        }
        points[2].q = 7.0;
        let mut scratch = [0u32; 15];
        let mut out = [0u32; 5];
        let n = convex_hull_tensor_xy(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(&out[..n], &[0, 1, 2, 3]);
        assert_eq!(points[out[2] as usize].q, 7.0);
    }

    #[test]
    fn reports_caller_buffer_requirements() {
        let points = [Point2::new(0.0, 0.0); 4];
        let mut scratch = [0u32; 11];
        let mut out = [0u32; 4];
        assert_eq!(
            convex_hull_indices_2(&points, &mut scratch, &mut out),
            Err(HullError::ScratchTooSmall { required: 12 })
        );
    }

    #[test]
    fn kernel_generic_path_matches_default_path() {
        // The P1.2 contract: the same algorithm over any GeometryKernel
        // produces identical output. The default (FilteredF64Kernel) and the
        // explicit kernel-generic call must agree byte-for-byte on the hull
        // indices.
        use super::FilteredF64Kernel;

        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 0.5),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.25, 0.75),
        ];
        let mut scratch_a = [0u32; 18];
        let mut out_a = [0u32; 6];
        let n_a = convex_hull_indices_2(&points, &mut scratch_a, &mut out_a).unwrap();

        let mut scratch_b = [0u32; 18];
        let mut out_b = [0u32; 6];
        let n_b = convex_hull_indices_2_with_kernel(
            &FilteredF64Kernel::default(),
            &points,
            &mut scratch_b,
            &mut out_b,
        )
        .unwrap();

        assert_eq!(n_a, n_b);
        assert_eq!(&out_a[..n_a], &out_b[..n_b]);
    }

    #[test]
    fn strongly_convex_check_matches_through_kernel() {
        use super::FilteredF64Kernel;

        let hull = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        assert!(is_ccw_strongly_convex_2(&hull));
        assert!(is_ccw_strongly_convex_2_with_kernel(
            &FilteredF64Kernel::default(),
            &hull
        ));

        let non_convex = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 0.5),
            Point2::new(1.0, 1.0),
        ];
        assert!(!is_ccw_strongly_convex_2(&non_convex));
        assert!(!is_ccw_strongly_convex_2_with_kernel(
            &FilteredF64Kernel::default(),
            &non_convex
        ));
    }

    /// Regression test: all points on the convex hull must not overflow the
    /// stack buffer. Before the fix, the upper hull phase of Andrew's
    /// monotone chain could temporarily hold 2n entries, overflowing the
    /// n-element stack. This hexagon (all 6 vertices on the hull) reproduces
    /// the original panic.
    #[test]
    fn convex_polygon_all_points_on_hull_no_overflow() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(3.0, 2.0),
            Point2::new(2.0, 4.0),
            Point2::new(0.0, 4.0),
            Point2::new(-1.0, 2.0),
        ];
        let mut scratch = [0u32; 18]; // 3 * 6
        let mut out = [0u32; 6];
        let n = convex_hull_indices_2(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(n, 6, "all 6 vertices should be on the hull");
        let hull: Vec<Point2> = out[..n].iter().map(|&i| points[i as usize]).collect();
        assert!(is_ccw_strongly_convex_2(&hull));
    }
}
