//! Pure geometry for dragging instrument-graph nodes.
//!
//! Centres stay on-canvas so the node body never clips. Hit testing is an
//! axis-aligned box around each centre. No Document, no Host IDs.

pub const CANVAS_W: f32 = 520.0;
pub const CANVAS_H: f32 = 160.0;
pub const NODE_W: f32 = 96.0;
pub const NODE_H: f32 = 36.0;

#[inline]
fn half_w() -> f32 {
    NODE_W * 0.5
}

#[inline]
fn half_h() -> f32 {
    NODE_H * 0.5
}

/// Keep the node fully inside the canvas (centre inset by NODE_W/2, NODE_H/2).
pub fn clamp_center(x: f32, y: f32) -> (f32, f32) {
    let hx = half_w();
    let hy = half_h();
    (x.clamp(hx, CANVAS_W - hx), y.clamp(hy, CANVAS_H - hy))
}

/// Translate a centre. Does not clamp; compose with [`clamp_center`].
pub fn apply_delta(x: f32, y: f32, dx: f32, dy: f32) -> (f32, f32) {
    (x + dx, y + dy)
}

/// First node whose axis-aligned box around `centre` contains `(px, py)`.
pub fn hit_index(points: &[(f32, f32)], px: f32, py: f32) -> Option<usize> {
    let hx = half_w();
    let hy = half_h();
    points
        .iter()
        .position(|&(cx, cy)| (px - cx).abs() <= hx && (py - cy).abs() <= hy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_corners() {
        assert_eq!(clamp_center(0.0, 0.0), (half_w(), half_h()));
        assert_eq!(
            clamp_center(CANVAS_W, CANVAS_H),
            (CANVAS_W - half_w(), CANVAS_H - half_h())
        );
        assert_eq!(clamp_center(-40.0, -20.0), (half_w(), half_h()));
        assert_eq!(
            clamp_center(CANVAS_W + 80.0, CANVAS_H + 40.0),
            (CANVAS_W - half_w(), CANVAS_H - half_h())
        );
        assert_eq!(clamp_center(260.0, 80.0), (260.0, 80.0));
    }

    #[test]
    fn apply_delta_then_clamp() {
        let (x, y) = apply_delta(260.0, 80.0, 1000.0, 1000.0);
        assert_eq!((x, y), (1260.0, 1080.0));
        assert_eq!(
            clamp_center(x, y),
            (CANVAS_W - half_w(), CANVAS_H - half_h())
        );
        let (x, y) = apply_delta(260.0, 80.0, -1000.0, -1000.0);
        assert_eq!(clamp_center(x, y), (half_w(), half_h()));
    }

    #[test]
    fn hit_index_hit() {
        let points = [(100.0, 50.0), (300.0, 90.0)];
        assert_eq!(hit_index(&points, 100.0, 50.0), Some(0));
        assert_eq!(hit_index(&points, 100.0 + half_w(), 50.0), Some(0));
        assert_eq!(hit_index(&points, 300.0, 90.0), Some(1));
        assert_eq!(hit_index(&points, 300.0 - half_w(), 90.0 + half_h()), Some(1));
    }

    #[test]
    fn hit_index_miss() {
        let points = [(100.0, 50.0), (300.0, 90.0)];
        assert_eq!(hit_index(&points, 0.0, 0.0), None);
        assert_eq!(hit_index(&points, 100.0 + half_w() + 1.0, 50.0), None);
        assert_eq!(hit_index(&points, 100.0, 50.0 + half_h() + 1.0), None);
    }

    #[test]
    fn hit_index_empty_is_none() {
        assert_eq!(hit_index(&[], 80.0, 40.0), None);
    }
}
