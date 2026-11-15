use super::super::authoring::{loft_convex_sections_into, AuthoringError, ClosedLoftReceipt};
#[derive(Clone, Copy)]
pub struct WheelPanel<'a> {
    pub center_x: f32,
    pub thickness: f32,
    pub z: [f32; 2],
    pub bottom: f32,
    pub top: [f32; 2],
    pub wheel_y: f32,
    pub clearance_radius: f32,
    pub axles: &'a [f32],
    pub segments: u16,
}
/// Constructs a closed thin panel above authored wheel openings. At most 193
/// four-point sections bound the cold scratch to under 32 KiB, excluding the
/// existing bounded loft kernel. Output capacity is 10*(segments+1) vertices
/// and 8*(segments+1) triangles. All parameters are checked before output writes.
pub fn wheel_clearance_panel_into(
    recipe: WheelPanel<'_>,
    positions: &mut [[f32; 3]],
    triangles: &mut [[u32; 3]],
) -> Result<ClosedLoftReceipt, AuthoringError> {
    let invalid = || AuthoringError::InvalidParameters("invalid wheel-clearance panel");
    if !(8..=192).contains(&recipe.segments)
        || recipe.axles.len() > 4
        || recipe.axles.is_empty()
        || [
            recipe.center_x,
            recipe.thickness,
            recipe.z[0],
            recipe.z[1],
            recipe.bottom,
            recipe.top[0],
            recipe.top[1],
            recipe.wheel_y,
            recipe.clearance_radius,
        ]
        .iter()
        .chain(recipe.axles.iter())
        .any(|v| !v.is_finite())
        || recipe.thickness <= 0.0
        || recipe.clearance_radius <= 0.0
        || recipe.z[1] - recipe.z[0] <= 1e-5
        || recipe.top.iter().any(|top| *top <= recipe.bottom + 0.001)
    {
        return Err(invalid());
    }
    let sections = recipe.segments as usize + 1;
    if positions.len() < sections * 10 {
        return Err(AuthoringError::BufferTooSmall {
            needed: sections * 10,
            have: positions.len(),
        });
    }
    if triangles.len() < sections * 8 {
        return Err(AuthoringError::BufferTooSmall {
            needed: sections * 8,
            have: triangles.len(),
        });
    }
    let mut rings = Vec::with_capacity(sections);
    for index in 0..sections {
        let t = index as f32 / recipe.segments as f32;
        let z = recipe.z[0] + (recipe.z[1] - recipe.z[0]) * t;
        let top = recipe.top[0] + (recipe.top[1] - recipe.top[0]) * t;
        let mut bottom = recipe.bottom;
        for axle in recipe.axles {
            let delta = z - *axle;
            if delta.abs() < recipe.clearance_radius {
                bottom = bottom.max(
                    recipe.wheel_y
                        + (recipe.clearance_radius.powi(2) - delta.powi(2))
                            .max(0.0)
                            .sqrt(),
                );
            }
        }
        let left = recipe.center_x - recipe.thickness * 0.5;
        let right = recipe.center_x + recipe.thickness * 0.5;
        if ![left, right, z, top, bottom].iter().all(|v| v.is_finite())
            || left >= right
            || bottom >= top
        {
            return Err(invalid());
        }
        rings.push(vec![
            [left, bottom, z],
            [right, bottom, z],
            [right, top, z],
            [left, top, z],
        ]);
    }
    loft_convex_sections_into(&rings, positions, triangles)
}
