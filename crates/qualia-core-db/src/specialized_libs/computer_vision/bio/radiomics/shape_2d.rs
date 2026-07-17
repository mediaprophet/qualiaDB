//! 2D shape features from a binary mask (compatibility wrapper).
//!
//! Prefer [`super::shape_3d_lite::shape_2d_from_mask`] for the full descriptor set.
//! This module keeps a compact `Shape2d` / `shape_2d_features` surface used by
//! histopathology / tracking call sites.

use super::first_order_stats::RadiomicsError;
use super::shape_3d_lite::shape_2d_from_mask;

/// Compact 2D shape summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape2d {
    pub area: f64,
    pub perimeter: f64,
    /// Circularity proxy (4πA/P²); named sphericity in some call sites.
    pub sphericity: f64,
    pub max_diameter: f64,
}

/// Extract 2D shape from a binary mask (`nonzero` = object).
pub fn shape_2d_features(
    mask: &[u8],
    width: usize,
    height: usize,
) -> Result<Shape2d, RadiomicsError> {
    let s = shape_2d_from_mask(mask, width, height)?;
    Ok(Shape2d {
        area: s.area,
        perimeter: s.perimeter,
        sphericity: s.circularity,
        max_diameter: s.max_diameter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_block() {
        let mut m = [0u8; 16];
        m[5] = 1;
        m[6] = 1;
        m[9] = 1;
        m[10] = 1;
        let s = shape_2d_features(&m, 4, 4).unwrap();
        assert!((s.area - 4.0).abs() < 1e-9);
        assert!(s.max_diameter > 0.0);
    }
}
