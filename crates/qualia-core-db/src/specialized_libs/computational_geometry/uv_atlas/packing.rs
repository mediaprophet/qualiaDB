use super::super::{orientation_2, Orientation, Point2};
use super::{invalid, probe, AtlasChart, Result};
fn nonpositive(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    orientation_2(
        Point2::new(a[0], a[1]),
        Point2::new(b[0], b[1]),
        Point2::new(c[0], c[1]),
    ) != Orientation::CounterClockwise
}
pub(super) fn pack(charts: &mut [AtlasChart], side: u32) -> Result<()> {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut height = 0u32;
    // Stable source-chart order, no rotation or silent density reduction.
    for c in charts {
        let [_, _, w, h] = c.rect_pixels;
        if w > side || h > side {
            return Err(invalid("chart exceeds atlas"));
        }
        if x + w > side {
            x = 0;
            y += height;
            height = 0;
        }
        if y + h > side {
            return Err(invalid("atlas cannot fit requested density/gutter; enlarge resolution, reduce density or change seams"));
        }
        c.rect_pixels = [x, y, w, h];
        x += w;
        height = height.max(h);
    }
    Ok(())
}
fn overlaps(a: [[f64; 2]; 3], b: [[f64; 2]; 3]) -> bool {
    // Separating axis theorem, allowing shared boundary but no positive area
    // overlap. Both triangles have already passed positive orientation checks.
    for t in [a, b] {
        for i in 0..3 {
            let p = t[i];
            let q = t[(i + 1) % 3];
            if a.iter().all(|&v| nonpositive(p, q, v)) || b.iter().all(|&v| nonpositive(p, q, v)) {
                return false;
            }
        }
    }
    true
}
pub(super) fn no_overlap(
    uv: &[[f64; 2]],
    triangles: &[[u32; 3]],
    cancelled: &dyn Fn() -> bool,
) -> Result<()> {
    for (i, a) in triangles.iter().enumerate() {
        probe(cancelled)?;
        let a = a.map(|v| uv[v as usize]);
        for b in &triangles[i + 1..] {
            let b = b.map(|v| uv[v as usize]);
            if overlaps(a, b) {
                return Err(invalid("positive-area UV triangle overlap"));
            }
        }
    }
    Ok(())
}
