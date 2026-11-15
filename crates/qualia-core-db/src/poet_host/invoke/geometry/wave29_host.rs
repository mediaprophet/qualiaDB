//! Remaining PlantGL review catalog seams for existing geometry kernels.

use super::super::args;
use crate::specialized_libs::computational_geometry as cg;
use vibe::{Diagnostic, Span, Value};

const MAX_HOST_POINTS: usize = 100_000;

fn flat_points3(value: &Value, span: Span) -> Result<Vec<cg::Point3>, Diagnostic> {
    let raw = args::f64s(value).ok_or_else(|| args::bad(span, "points must be a flat f64 list"))?;
    if raw.is_empty()
        || raw.len() % 3 != 0
        || raw.len() / 3 > MAX_HOST_POINTS
        || raw.iter().any(|x| !x.is_finite())
    {
        return Err(args::bad(span, "invalid or oversized point list"));
    }
    Ok(raw
        .chunks_exact(3)
        .map(|p| cg::Point3::new(p[0], p[1], p[2]))
        .collect())
}

/// `{profiles:[[radius,z,…],…], angles:[radians,…], slices}` → mesh.
pub fn swung_surface_host(input: &Value, span: Span) -> Result<Value, Diagnostic> {
    let raw_profiles = args::rec(input, "profiles")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "missing 'profiles' list"))?;
    let angles = args::rec_f64_list(input, "angles")
        .ok_or_else(|| args::bad(span, "missing 'angles' list"))?;
    let slices =
        args::rec_u64(input, "slices").ok_or_else(|| args::bad(span, "missing 'slices'"))? as usize;
    if raw_profiles.is_empty()
        || raw_profiles.len() != angles.len()
        || slices < 3
        || angles.iter().any(|x| !x.is_finite())
    {
        return Err(args::bad(span, "invalid profiles, angles or slices"));
    }
    let mut profiles = Vec::with_capacity(raw_profiles.len());
    for row in raw_profiles {
        let raw =
            args::f64s(row).ok_or_else(|| args::bad(span, "profile must be flat [radius,z,…]"))?;
        if raw.len() < 4 || raw.len() % 2 != 0 || raw.iter().any(|x| !x.is_finite()) {
            return Err(args::bad(span, "invalid profile"));
        }
        profiles.push(
            raw.chunks_exact(2)
                .map(|p| cg::Point2::new(p[0], p[1]))
                .collect::<Vec<_>>(),
        );
    }
    let profile_refs = profiles.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let m = profiles[0].len();
    let vertices_len = slices
        .checked_mul(m)
        .filter(|&n| n <= MAX_HOST_POINTS)
        .ok_or_else(|| args::bad(span, "swung surface exceeds Host budget"))?;
    let triangles_len = slices
        .checked_mul(m - 1)
        .and_then(|n| n.checked_mul(2))
        .filter(|&n| n <= 2 * MAX_HOST_POINTS)
        .ok_or_else(|| args::bad(span, "swung surface exceeds Host budget"))?;
    let mut vertices = vec![cg::Point3::default(); vertices_len];
    let mut triangles = vec![[0u32; 3]; triangles_len];
    let (nv, nt) = cg::swung_surface(
        &profile_refs,
        &angles,
        slices,
        &mut vertices,
        &mut triangles,
    )
    .map_err(|e| args::bad(span, format!("swung_surface: {e:?}")))?;
    Ok(args::record([
        (
            "vertices",
            args::f64_list_value(vertices[..nv].iter().flat_map(|p| [p.x, p.y, p.z])),
        ),
        (
            "triangles",
            Value::List(
                triangles[..nt]
                    .iter()
                    .flat_map(|t| t.iter().map(|&i| Value::U64(i as u64)))
                    .collect(),
            ),
        ),
    ]))
}

/// `{parents:[u32,…], tip_radius, exponent}` → `{radii:[f64,…]}`.
pub fn pipe_model_radii_host(input: &Value, span: Span) -> Result<Value, Diagnostic> {
    let raw = args::rec_f64_list(input, "parents")
        .ok_or_else(|| args::bad(span, "missing 'parents' list"))?;
    if raw.is_empty() || raw.len() > MAX_HOST_POINTS {
        return Err(args::bad(span, "invalid or oversized parents list"));
    }
    let count = raw.len();
    let mut parents = Vec::with_capacity(count);
    for (i, x) in raw.into_iter().enumerate() {
        if !x.is_finite() || x < 0.0 || x > u32::MAX as f64 || x.fract() != 0.0 {
            return Err(args::bad(span, "invalid parent index"));
        }
        let p = x as u32;
        if p != cg::ROOT_PARENT && (p as usize >= count || p as usize == i) {
            return Err(args::bad(span, "parent index out of range"));
        }
        parents.push(p);
    }
    let tip = args::rec_f64(input, "tip_radius")
        .ok_or_else(|| args::bad(span, "missing 'tip_radius'"))?;
    let exponent =
        args::rec_f64(input, "exponent").ok_or_else(|| args::bad(span, "missing 'exponent'"))?;
    let mut scratch = vec![0u32; parents.len()];
    let mut radii = vec![0.0; parents.len()];
    cg::pipe_model_radii(&parents, tip, exponent, &mut scratch, &mut radii)
        .map_err(|e| args::bad(span, format!("pipe_model_radii: {e:?}")))?;
    Ok(args::record([("radii", args::f64_list_value(radii))]))
}

/// `{points:[x,y,z,…]}` → center, radius, and support indices.
pub fn miniball_3d_host(input: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = flat_points3(
        args::rec(input, "points").ok_or_else(|| args::bad(span, "missing 'points'"))?,
        span,
    )?;
    let sphere = cg::smallest_enclosing_sphere_3d(&points)
        .map_err(|e| args::bad(span, format!("miniball_3d: {e:?}")))?;
    Ok(args::record([
        (
            "center",
            args::f64_list_value(vec![sphere.center.x, sphere.center.y, sphere.center.z]),
        ),
        ("radius", Value::F64(sphere.radius)),
        (
            "support",
            Value::List(
                sphere.support[..sphere.support_count as usize]
                    .iter()
                    .map(|&i| Value::U64(i as u64))
                    .collect(),
            ),
        ),
    ]))
}

/// `{vertices:[x,y,z,…], triangles:[i,j,k,…], origin:[x,y,z], normal:[x,y,z]}`
/// → `{segments:[x0,y0,z0,x1,y1,z1,…], count}`.
pub fn mesh_plane_slice_host(input: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = flat_points3(
        args::rec(input, "vertices").ok_or_else(|| args::bad(span, "missing 'vertices'"))?,
        span,
    )?;
    let raw = args::rec_f64_list(input, "triangles")
        .ok_or_else(|| args::bad(span, "missing 'triangles' list"))?;
    if raw.len() % 3 != 0 || raw.len() / 3 > 2 * MAX_HOST_POINTS {
        return Err(args::bad(span, "invalid or oversized triangle list"));
    }
    let mut triangles = Vec::with_capacity(raw.len() / 3);
    for tri in raw.chunks_exact(3) {
        let mut indices = [0u32; 3];
        for (slot, &index) in indices.iter_mut().zip(tri) {
            if !index.is_finite()
                || index < 0.0
                || index.fract() != 0.0
                || index > u32::MAX as f64
                || index as usize >= vertices.len()
            {
                return Err(args::bad(span, "invalid triangle vertex index"));
            }
            *slot = index as u32;
        }
        triangles.push(indices);
    }
    let point = |key| -> Result<cg::Point3, Diagnostic> {
        let raw = args::rec_f64_list(input, key)
            .ok_or_else(|| args::bad(span, format!("missing '{key}' point")))?;
        if raw.len() != 3 || raw.iter().any(|x| !x.is_finite()) {
            return Err(args::bad(span, format!("invalid '{key}' point")));
        }
        Ok(cg::Point3::new(raw[0], raw[1], raw[2]))
    };
    let (origin, normal) = (point("origin")?, point("normal")?);
    let mut segments = vec![[cg::Point3::default(); 2]; triangles.len()];
    let count = cg::slice_mesh_with_plane(&vertices, &triangles, origin, normal, &mut segments)
        .map_err(|e| args::bad(span, format!("mesh_plane_slice: {e:?}")))?;
    Ok(args::record([
        (
            "segments",
            args::f64_list_value(
                segments[..count]
                    .iter()
                    .flat_map(|pair| pair.iter().flat_map(|p| [p.x, p.y, p.z])),
            ),
        ),
        ("count", Value::U64(count as u64)),
    ]))
}

#[cfg(test)]
#[path = "wave29_host_tests.rs"]
mod tests;
