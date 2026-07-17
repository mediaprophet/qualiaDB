//! Lite shape descriptors for binary masks (2D and sparse 3D voxel lists).
//!
//! 2D: area, perimeter (4-connected edge count), circularity proxy, max diameter.
//! 3D: volume (voxel count × spacing), surface proxy, sphericity proxy, max diameter
//! from voxel AABB / pairwise sample.

use super::first_order_stats::RadiomicsError;

/// 2D shape features from a binary mask (nonzero = foreground).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape2dFeatures {
    /// Foreground pixel count (area in pixels).
    pub area: f64,
    /// Approximate perimeter (count of 4-connected edge transitions / 2-ish heuristic).
    pub perimeter: f64,
    /// 4π·area / perimeter² (circle → 1). 0 if perimeter is zero.
    pub circularity: f64,
    /// Max Euclidean distance between foreground pixels (or AABB diagonal if > max pairs).
    pub max_diameter: f64,
}

/// 3D shape features from a sparse voxel list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape3dFeatures {
    pub volume: f64,
    /// Approximate surface area from face-exposure count × spacing².
    pub surface_area: f64,
    /// (π^(1/3) · (6V)^(2/3)) / A  — sphere → 1.
    pub sphericity: f64,
    pub max_diameter: f64,
    pub voxel_count: usize,
}

/// Extract 2D shape from a binary mask (`nonzero` = object).
///
/// `mask` is row-major `width * height` of `u8`.
pub fn shape_2d_from_mask(
    mask: &[u8],
    width: usize,
    height: usize,
) -> Result<Shape2dFeatures, RadiomicsError> {
    if width == 0 || height == 0 || mask.len() < width * height {
        return Err(RadiomicsError::DimensionMismatch);
    }

    // Collect foreground coords (bounded: use two-pass — count then stack limit).
    // For general size we iterate without storing all points for area/perimeter,
    // and only store coords for diameter if count is small.
    let mut area = 0usize;
    let mut edge_steps = 0usize;

    for y in 0..height {
        for x in 0..width {
            if mask[y * width + x] == 0 {
                continue;
            }
            area += 1;
            // 4-neighbour exterior edges
            if x == 0 || mask[y * width + x - 1] == 0 {
                edge_steps += 1;
            }
            if x + 1 >= width || mask[y * width + x + 1] == 0 {
                edge_steps += 1;
            }
            if y == 0 || mask[(y - 1) * width + x] == 0 {
                edge_steps += 1;
            }
            if y + 1 >= height || mask[(y + 1) * width + x] == 0 {
                edge_steps += 1;
            }
        }
    }

    if area == 0 {
        return Err(RadiomicsError::EmptyInput);
    }

    // Perimeter ≈ number of unit exterior edges (grid metric).
    let perimeter = edge_steps as f64;
    let area_f = area as f64;
    let circularity = if perimeter > 1e-15 {
        (4.0 * core::f64::consts::PI * area_f) / (perimeter * perimeter)
    } else {
        0.0
    };

    // Max diameter: AABB diagonal of foreground (O(N) cheap proxy).
    // For small masks also refine with pairwise if ≤ MAX_PAIR_POINTS.
    let mut min_x = width;
    let mut max_x = 0usize;
    let mut min_y = height;
    let mut max_y = 0usize;
    // Collect up to MAX_PAIR_POINTS for exact diameter refinement.
    const MAX_PAIR: usize = 256;
    let mut pts_x = [0usize; MAX_PAIR];
    let mut pts_y = [0usize; MAX_PAIR];
    let mut n_pts = 0usize;

    for y in 0..height {
        for x in 0..width {
            if mask[y * width + x] == 0 {
                continue;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            if n_pts < MAX_PAIR {
                pts_x[n_pts] = x;
                pts_y[n_pts] = y;
                n_pts += 1;
            }
        }
    }

    let dx = (max_x - min_x) as f64;
    let dy = (max_y - min_y) as f64;
    let mut max_d = (dx * dx + dy * dy).sqrt();

    // Exact pairwise if we captured all points
    if n_pts == area && area > 1 {
        max_d = 0.0;
        for i in 0..n_pts {
            for j in (i + 1)..n_pts {
                let ddx = pts_x[i] as f64 - pts_x[j] as f64;
                let ddy = pts_y[i] as f64 - pts_y[j] as f64;
                let d = (ddx * ddx + ddy * ddy).sqrt();
                if d > max_d {
                    max_d = d;
                }
            }
        }
    }

    Ok(Shape2dFeatures {
        area: area_f,
        perimeter,
        circularity,
        max_diameter: max_d,
    })
}

/// Extract 3D shape from a sparse list of voxel coordinates.
///
/// `voxels` is flat `[x0,y0,z0, x1,y1,z1, …]` (length = 3 * N).
/// `spacing` is (sx, sy, sz) physical size of one voxel.
pub fn shape_3d_from_voxels(
    voxels: &[i32],
    spacing: (f64, f64, f64),
) -> Result<Shape3dFeatures, RadiomicsError> {
    if voxels.len() % 3 != 0 || voxels.is_empty() {
        return Err(if voxels.is_empty() {
            RadiomicsError::EmptyInput
        } else {
            RadiomicsError::InvalidParameter
        });
    }
    let (sx, sy, sz) = spacing;
    if sx <= 0.0 || sy <= 0.0 || sz <= 0.0 {
        return Err(RadiomicsError::InvalidParameter);
    }

    let n = voxels.len() / 3;
    let vox_vol = sx * sy * sz;
    let volume = n as f64 * vox_vol;

    // Surface: count exposed faces via neighbour lookup in a small hash-free set.
    // For N ≤ 512, O(N²) neighbour check is fine; for larger N use face count approx
    // from AABB surface (honest lite path).
    const MAX_SPARSE: usize = 512;
    let surface_area = if n <= MAX_SPARSE {
        let mut exposed = 0u32;
        for i in 0..n {
            let x = voxels[i * 3];
            let y = voxels[i * 3 + 1];
            let z = voxels[i * 3 + 2];
            // 6-neighbour
            let neigh = [
                (x + 1, y, z),
                (x - 1, y, z),
                (x, y + 1, z),
                (x, y - 1, z),
                (x, y, z + 1),
                (x, y, z - 1),
            ];
            for (nx, ny, nz) in neigh {
                let mut found = false;
                for j in 0..n {
                    if voxels[j * 3] == nx
                        && voxels[j * 3 + 1] == ny
                        && voxels[j * 3 + 2] == nz
                    {
                        found = true;
                        break;
                    }
                }
                if !found {
                    exposed += 1;
                }
            }
        }
        // Each exposed face has area depending on orientation; use average face area.
        let face_area = (sx * sy + sy * sz + sz * sx) / 3.0;
        exposed as f64 * face_area
    } else {
        // AABB surface proxy for large sets
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        let mut min_z = i32::MAX;
        let mut max_z = i32::MIN;
        for i in 0..n {
            let x = voxels[i * 3];
            let y = voxels[i * 3 + 1];
            let z = voxels[i * 3 + 2];
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            min_z = min_z.min(z);
            max_z = max_z.max(z);
        }
        let lx = (max_x - min_x + 1) as f64 * sx;
        let ly = (max_y - min_y + 1) as f64 * sy;
        let lz = (max_z - min_z + 1) as f64 * sz;
        2.0 * (lx * ly + ly * lz + lz * lx)
    };

    // Sphericity = π^(1/3) * (6V)^(2/3) / A
    let sphericity = if surface_area > 1e-15 {
        let six_v = 6.0 * volume;
        (core::f64::consts::PI.powf(1.0 / 3.0) * six_v.powf(2.0 / 3.0)) / surface_area
    } else {
        0.0
    };

    // Max diameter in physical units (AABB diagonal, refine pairwise if small).
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut min_z = i32::MAX;
    let mut max_z = i32::MIN;
    for i in 0..n {
        let x = voxels[i * 3];
        let y = voxels[i * 3 + 1];
        let z = voxels[i * 3 + 2];
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    let dx = (max_x - min_x) as f64 * sx;
    let dy = (max_y - min_y) as f64 * sy;
    let dz = (max_z - min_z) as f64 * sz;
    let mut max_diameter = (dx * dx + dy * dy + dz * dz).sqrt();

    if n <= 64 && n > 1 {
        max_diameter = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let ddx = (voxels[i * 3] - voxels[j * 3]) as f64 * sx;
                let ddy = (voxels[i * 3 + 1] - voxels[j * 3 + 1]) as f64 * sy;
                let ddz = (voxels[i * 3 + 2] - voxels[j * 3 + 2]) as f64 * sz;
                let d = (ddx * ddx + ddy * ddy + ddz * ddz).sqrt();
                if d > max_diameter {
                    max_diameter = d;
                }
            }
        }
    }

    Ok(Shape3dFeatures {
        volume,
        surface_area,
        sphericity,
        max_diameter,
        voxel_count: n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_square_2d() {
        // 3×3 full mask
        let mask = [1u8; 9];
        let s = shape_2d_from_mask(&mask, 3, 3).unwrap();
        assert!((s.area - 9.0).abs() < 1e-9);
        assert!(s.perimeter > 0.0);
        assert!(s.circularity > 0.0);
        // Corners (0,0) and (2,2) → diameter √8
        assert!((s.max_diameter - (8.0f64).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn empty_mask_errors() {
        let mask = [0u8; 4];
        assert_eq!(
            shape_2d_from_mask(&mask, 2, 2).unwrap_err(),
            RadiomicsError::EmptyInput
        );
    }

    #[test]
    fn single_voxel_3d() {
        let vox = [0i32, 0, 0];
        let s = shape_3d_from_voxels(&vox, (1.0, 1.0, 1.0)).unwrap();
        assert!((s.volume - 1.0).abs() < 1e-12);
        assert_eq!(s.voxel_count, 1);
        assert!(s.surface_area > 0.0); // 6 exposed faces
        assert!((s.max_diameter).abs() < 1e-12);
    }

    #[test]
    fn two_voxel_line_3d() {
        let vox = [0i32, 0, 0, 1, 0, 0];
        let s = shape_3d_from_voxels(&vox, (2.0, 1.0, 1.0)).unwrap();
        assert!((s.volume - 4.0).abs() < 1e-12); // 2 * 2*1*1
        assert!((s.max_diameter - 2.0).abs() < 1e-12);
        assert!(s.sphericity > 0.0 && s.sphericity <= 1.0 + 0.05);
    }

    #[test]
    fn bad_spacing() {
        let vox = [0i32, 0, 0];
        assert_eq!(
            shape_3d_from_voxels(&vox, (0.0, 1.0, 1.0)).unwrap_err(),
            RadiomicsError::InvalidParameter
        );
    }
}
