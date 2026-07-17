//! SR1 — plan overlapping tiles for bounded SR (VRAM/RAM-aware).

use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Default edge-friendly tile size (pixels on input side).
pub const DEFAULT_TILE: u32 = 256;
/// Default overlap (pixels on input side).
pub const DEFAULT_OVERLAP: u32 = 32;

/// Tiling policy for classical and learned SR backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilePolicy {
    pub tile_w: u32,
    pub tile_h: u32,
    pub overlap: u32,
    /// Fail closed if the plan would exceed this count.
    pub max_tiles: u32,
}

impl Default for TilePolicy {
    fn default() -> Self {
        Self {
            tile_w: DEFAULT_TILE,
            tile_h: DEFAULT_TILE,
            overlap: DEFAULT_OVERLAP,
            max_tiles: 4096,
        }
    }
}

/// Axis-aligned tile on the **input** image (pre-scale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Plan a grid of overlapping tiles covering `[0,width) × [0,height)`.
///
/// Step along each axis is `tile - overlap` (must be ≥ 1). Edge tiles are
/// clamped to the image border (no padding).
pub fn plan_tiles(width: u32, height: u32, policy: TilePolicy) -> Result<Vec<TileRect>, CvError> {
    if width == 0 || height == 0 {
        return Err(CvError::EmptyInput);
    }
    if policy.tile_w == 0 || policy.tile_h == 0 {
        return Err(CvError::InvalidParameter);
    }
    if policy.overlap >= policy.tile_w || policy.overlap >= policy.tile_h {
        return Err(CvError::InvalidParameter);
    }
    let step_x = policy.tile_w - policy.overlap;
    let step_y = policy.tile_h - policy.overlap;

    // Single tile if the whole frame fits.
    if width <= policy.tile_w && height <= policy.tile_h {
        return Ok(vec![TileRect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        }]);
    }

    let mut xs = axis_starts(width, policy.tile_w, step_x);
    let mut ys = axis_starts(height, policy.tile_h, step_y);

    // Ensure last start reaches the far edge.
    ensure_edge_start(&mut xs, width, policy.tile_w);
    ensure_edge_start(&mut ys, height, policy.tile_h);

    let mut out = Vec::with_capacity(xs.len() * ys.len());
    for &y0 in &ys {
        let h = policy.tile_h.min(height - y0);
        for &x0 in &xs {
            let w = policy.tile_w.min(width - x0);
            out.push(TileRect {
                x: x0,
                y: y0,
                w,
                h,
            });
            if out.len() as u32 > policy.max_tiles {
                return Err(CvError::InvalidParameter);
            }
        }
    }
    Ok(out)
}

fn axis_starts(len: u32, tile: u32, step: u32) -> Vec<u32> {
    let mut v = Vec::new();
    let mut p = 0u32;
    while p < len {
        v.push(p);
        if p + tile >= len {
            break;
        }
        let next = p.saturating_add(step);
        if next <= p {
            break;
        }
        p = next;
    }
    v
}

fn ensure_edge_start(starts: &mut Vec<u32>, len: u32, tile: u32) {
    if len == 0 {
        return;
    }
    let last = len.saturating_sub(tile.min(len));
    if starts.last().copied() != Some(last) {
        // Avoid duplicate if already covering.
        if starts.iter().all(|&s| s != last) {
            starts.push(last);
        }
    }
}

/// How many tiles would be planned.
pub fn estimate_tile_count(width: u32, height: u32, policy: TilePolicy) -> Result<u32, CvError> {
    Ok(plan_tiles(width, height, policy)?.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_image_one_tile() {
        let p = TilePolicy {
            tile_w: 64,
            tile_h: 64,
            overlap: 8,
            max_tiles: 16,
        };
        let t = plan_tiles(32, 32, p).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].w, 32);
        assert_eq!(t[0].h, 32);
    }

    #[test]
    fn covers_large_frame() {
        let p = TilePolicy {
            tile_w: 64,
            tile_h: 64,
            overlap: 16,
            max_tiles: 256,
        };
        let t = plan_tiles(200, 150, p).unwrap();
        assert!(t.len() > 1);
        let max_x = t.iter().map(|r| r.x + r.w).max().unwrap();
        let max_y = t.iter().map(|r| r.y + r.h).max().unwrap();
        assert_eq!(max_x, 200);
        assert_eq!(max_y, 150);
        // Every pixel covered by at least one tile.
        for y in 0..150u32 {
            for x in 0..200u32 {
                assert!(
                    t.iter()
                        .any(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h),
                    "uncovered ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn max_tiles_fail_closed() {
        let p = TilePolicy {
            tile_w: 32,
            tile_h: 32,
            overlap: 0,
            max_tiles: 2,
        };
        assert!(plan_tiles(128, 128, p).is_err());
    }

    #[test]
    fn bad_overlap_rejected() {
        let p = TilePolicy {
            tile_w: 32,
            tile_h: 32,
            overlap: 32,
            max_tiles: 8,
        };
        assert!(plan_tiles(64, 64, p).is_err());
    }
}
