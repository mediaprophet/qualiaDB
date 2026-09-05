//! Style geometry parsing, grid snapping, and focused unit tests.

use super::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(super) fn parse_position(style: &str) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("left: ") {
            x = val.trim_end_matches("px").parse().unwrap_or(0.0);
        } else if let Some(val) = part.strip_prefix("top: ") {
            y = val.trim_end_matches("px").parse().unwrap_or(0.0);
        }
    }
    (x, y)
}

pub(super) fn update_z_index(style: &str, z: u32) -> String {
    let mut found = false;
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("z-index:") || part.starts_with("z-index: ") {
            result.push_str(&format!("z-index: {}; ", z));
            found = true;
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !found {
        result.push_str(&format!("z-index: {}; ", z));
    }
    result
}

pub(super) fn px(value: f32) -> String {
    format!("{}px", value.round() as i32)
}

pub(super) fn update_position(style: &str, new_x: f32, new_y: f32) -> String {
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("left: ") {
            result.push_str(&format!("left: {}; ", px(new_x)));
        } else if part.starts_with("top: ") {
            result.push_str(&format!("top: {}; ", px(new_y)));
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !result.contains("left:") {
        result.push_str(&format!("left: {}; ", px(new_x)));
    }
    if !result.contains("top:") {
        result.push_str(&format!("top: {}; ", px(new_y)));
    }
    result
}

pub(super) fn parse_size(style: &str) -> (f32, f32) {
    let mut w = 400.0;
    let mut h = 300.0;
    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("width: ") {
            w = val.trim_end_matches("px").parse().unwrap_or(400.0);
        } else if let Some(val) = part.strip_prefix("height: ") {
            h = val.trim_end_matches("px").parse().unwrap_or(300.0);
        }
    }
    (w, h)
}

pub(super) fn update_size(style: &str, new_w: f32, new_h: f32) -> String {
    let mut result = String::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.starts_with("width: ") {
            result.push_str(&format!("width: {}px; ", new_w as u32));
        } else if part.starts_with("height: ") {
            result.push_str(&format!("height: {}px; ", new_h as u32));
        } else if !part.is_empty() {
            result.push_str(part);
            result.push_str("; ");
        }
    }
    if !result.contains("width:") {
        result.push_str(&format!("width: {}px; ", new_w as u32));
    }
    if !result.contains("height:") {
        result.push_str(&format!("height: {}px; ", new_h as u32));
    }
    result
}

// ---------------------------------------------------------------------------
// Grid snapping
// ---------------------------------------------------------------------------

/// Default grid size in pixels.
const GRID_SIZE: f32 = 8.0;

pub(super) fn current_canvas_zoom(document: &Document) -> f32 {
    document
        .get_element_by_id("manifold-canvas")
        .and_then(|canvas| canvas.get_attribute("data-zoom"))
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0)
}

/// Snap a value to the nearest grid point.
pub fn snap_to_grid(value: f32) -> f32 {
    (value / GRID_SIZE).round() * GRID_SIZE
}

/// Snap a (x, y) pair to the grid.
pub fn snap_point(x: f32, y: f32) -> (f32, f32) {
    (snap_to_grid(x), snap_to_grid(y))
}

/// Clamp a value between min and max.
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Snap a world position. The manifold is not a fixed box — coordinates
/// are not clamped to the current viewport.
pub fn snap_clamp_position(
    x: f32,
    y: f32,
    _canvas_w: f32,
    _canvas_h: f32,
    _elem_w: f32,
    _elem_h: f32,
) -> (f32, f32) {
    snap_point(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_to_grid() {
        assert_eq!(snap_to_grid(0.0), 0.0);
        assert_eq!(snap_to_grid(15.0), 16.0);
        assert_eq!(snap_to_grid(17.0), 16.0);
        assert_eq!(snap_to_grid(24.0), 24.0);
        assert_eq!(snap_to_grid(-3.0), 0.0);
        assert_eq!(snap_to_grid(48.0), 48.0);
    }

    #[test]
    fn test_snap_point() {
        let (x, y) = snap_point(15.0, 33.0);
        assert_eq!(x, 16.0);
        assert_eq!(y, 32.0);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_snap_clamp_position() {
        let (x, y) = snap_clamp_position(15.0, 33.0, 800.0, 600.0, 200.0, 150.0);
        assert_eq!(x, 16.0);
        assert_eq!(y, 32.0);

        let (x2, y2) = snap_clamp_position(-12.0, 700.0, 800.0, 600.0, 200.0, 150.0);
        assert_eq!(x2, -16.0);
        assert_eq!(y2, 704.0);
    }

    #[test]
    fn test_container_rect_overlaps() {
        let r1 = ContainerRect {
            x: 80.0,
            y: 60.0,
            w: 400.0,
            h: 300.0,
        };
        let r2_overlapping = ContainerRect {
            x: 120.0,
            y: 100.0,
            w: 400.0,
            h: 300.0,
        };
        let r3_separate = ContainerRect {
            x: 520.0,
            y: 60.0,
            w: 400.0,
            h: 300.0,
        };
        let r4_below = ContainerRect {
            x: 80.0,
            y: 400.0,
            w: 400.0,
            h: 300.0,
        };

        assert!(r1.overlaps(&r2_overlapping, 20.0));
        assert!(!r1.overlaps(&r3_separate, 20.0));
        assert!(!r1.overlaps(&r4_below, 20.0));
    }
}
