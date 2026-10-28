//! UV-aligned tangent frame generation without allocation. This is a bounded
//! kernel shared by native and WASM mesh uploads, not a normal-map shader patch.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TangentFrameError {
    InvalidInput,
    OutputTooSmall,
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: [f32; 3]) -> [f32; 3] {
    let scale = dot(v, v).max(1e-20).sqrt().recip();
    v.map(|x| x * scale)
}

/// Strict derivative admission. The legacy tangent generator deliberately uses
/// a fallback for missing/degenerate UVs; this opt-in path refuses those cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictTangentFrameError {
    InvalidInput,
    Budget,
    OutputTooSmall,
    DegenerateGeometry,
    DegenerateUv,
    MixedOrientation,
    UndefinedFrame,
    Kernel,
}

/// Generate UV-aligned tangents using the existing deterministic owner, after
/// strict geometric/UV admission. UV orientation changes require vertex splits.
/// `temporary` and `scratch` may change on failure; `out` is unchanged on all
/// returned errors. No allocation occurs. The checked work charge is a bound on
/// structural visits, not elapsed time or a bound on caller memory.
pub fn tangents_from_uvs_strict_into(
    positions: &[[f32; 3]],
    uv: &[[f32; 2]],
    normals: &[[f32; 3]],
    indices: &[u32],
    max_work: u64,
    out: &mut [[f32; 4]],
    temporary: &mut [[f32; 4]],
    scratch: &mut [[f32; 6]],
) -> Result<usize, StrictTangentFrameError> {
    use StrictTangentFrameError::*;
    let n = positions.len();
    if n == 0 || indices.is_empty() || indices.len() % 3 != 0 || uv.len() != n || normals.len() != n
    {
        return Err(InvalidInput);
    }
    let visits = u64::try_from(n)
        .ok()
        .and_then(|v| v.checked_mul(100))
        .and_then(|v| {
            u64::try_from(indices.len() / 3)
                .ok()
                .and_then(|f| f.checked_mul(100)?.checked_add(v))
        })
        .ok_or(Budget)?;
    if visits > max_work {
        return Err(Budget);
    }
    if out.len() < n || temporary.len() < n || scratch.len() < n {
        return Err(OutputTooSmall);
    }
    if positions
        .iter()
        .flatten()
        .chain(uv.iter().flatten())
        .chain(normals.iter().flatten())
        .any(|v| !v.is_finite())
        || normals
            .iter()
            .any(|v| !dot(*v, *v).is_finite() || dot(*v, *v) < 1e-12)
        || indices.iter().any(|&v| v as usize >= n)
    {
        return Err(InvalidInput);
    }
    temporary[..n].fill([0.0; 4]);
    for face in indices.chunks_exact(3) {
        let [a, b, c] = [face[0] as usize, face[1] as usize, face[2] as usize];
        if a == b || b == c || c == a {
            return Err(DegenerateGeometry);
        }
        let e1 =
            std::array::from_fn::<_, 3, _>(|k| f64::from(positions[b][k]) - f64::from(positions[a][k]));
        let e2 =
            std::array::from_fn::<_, 3, _>(|k| f64::from(positions[c][k]) - f64::from(positions[a][k]));
        let area = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let area2 = area.iter().map(|v| v * v).sum::<f64>();
        if !area2.is_finite() || area2 <= 0.0 {
            return Err(DegenerateGeometry);
        }
        // Use the same f32 expression and cutoff as the existing tangent owner.
        let determinant = (uv[b][0] - uv[a][0]) * (uv[c][1] - uv[a][1])
            - (uv[c][0] - uv[a][0]) * (uv[b][1] - uv[a][1]);
        if !determinant.is_finite() || determinant.abs() < 1e-12 {
            return Err(DegenerateUv);
        }
        let sign = determinant.signum();
        for vertex in [a, b, c] {
            if temporary[vertex][3] != 0.0 && temporary[vertex][3] != sign {
                return Err(MixedOrientation);
            }
            temporary[vertex][3] = sign;
        }
    }
    // Unreferenced vertices do not have an authored tangent correspondence.
    if temporary[..n].iter().any(|v| v[3] == 0.0) {
        return Err(UndefinedFrame);
    }
    tangents_from_uvs_into(positions, uv, normals, indices, temporary, scratch)
        .map_err(|_| Kernel)?;
    for i in 0..n {
        if scratch[i].iter().chain(temporary[i].iter()).any(|v| !v.is_finite()) {
            return Err(UndefinedFrame);
        }
        let normal = unit(normals[i]);
        let sum = [scratch[i][0], scratch[i][1], scratch[i][2]];
        let along = dot(normal, sum);
        let tangent = std::array::from_fn::<_, 3, _>(|k| sum[k] - normal[k] * along);
        if !dot(tangent, tangent).is_finite() || dot(tangent, tangent) < 1e-12 {
            return Err(UndefinedFrame);
        }
        let bitangent = [scratch[i][3], scratch[i][4], scratch[i][5]];
        let orientation = dot(
            cross(
                normal,
                [temporary[i][0], temporary[i][1], temporary[i][2]],
            ),
            bitangent,
        );
        if !orientation.is_finite() || orientation == 0.0 {
            return Err(UndefinedFrame);
        }
    }
    out[..n].copy_from_slice(&temporary[..n]);
    Ok(n)
}

/// Outputs xyz tangent and glTF handedness w. Six floats of scratch per vertex
/// accumulate tangent/bitangent directions in deterministic triangle order.
/// Invalid indices, streams and buffers fail before either output is touched.
pub fn tangents_from_uvs_into(
    positions: &[[f32; 3]],
    uv: &[[f32; 2]],
    normals: &[[f32; 3]],
    indices: &[u32],
    out: &mut [[f32; 4]],
    scratch: &mut [[f32; 6]],
) -> Result<usize, TangentFrameError> {
    let n = positions.len();
    if n == 0
        || uv.len() != n
        || normals.len() != n
        || indices.len() % 3 != 0
        || positions
            .iter()
            .flatten()
            .chain(uv.iter().flatten())
            .chain(normals.iter().flatten())
            .any(|v| !v.is_finite())
        || normals
            .iter()
            .any(|v| !dot(*v, *v).is_finite() || dot(*v, *v) < 1e-12)
        || indices.iter().any(|i| *i as usize >= n)
    {
        return Err(TangentFrameError::InvalidInput);
    }
    if out.len() < n || scratch.len() < n {
        return Err(TangentFrameError::OutputTooSmall);
    }
    scratch[..n].fill([0.0; 6]);
    for face in indices.chunks_exact(3) {
        let [a, b, c] = [face[0] as usize, face[1] as usize, face[2] as usize];
        let e1 = std::array::from_fn::<_, 3, _>(|k| positions[b][k] - positions[a][k]);
        let e2 = std::array::from_fn::<_, 3, _>(|k| positions[c][k] - positions[a][k]);
        let du1 = uv[b][0] - uv[a][0];
        let dv1 = uv[b][1] - uv[a][1];
        let du2 = uv[c][0] - uv[a][0];
        let dv2 = uv[c][1] - uv[a][1];
        let det = du1 * dv2 - du2 * dv1;
        if !det.is_finite() || det.abs() < 1e-12 {
            continue;
        }
        let t = std::array::from_fn::<_, 3, _>(|k| (e1[k] * dv2 - e2[k] * dv1) / det);
        let bdir = std::array::from_fn::<_, 3, _>(|k| (e2[k] * du1 - e1[k] * du2) / det);
        if t.iter().chain(bdir.iter()).any(|v| !v.is_finite()) {
            continue;
        }
        for vertex in [a, b, c] {
            for k in 0..3 {
                scratch[vertex][k] += t[k];
                scratch[vertex][k + 3] += bdir[k];
            }
        }
    }
    for i in 0..n {
        let normal = unit(normals[i]);
        let sum = [scratch[i][0], scratch[i][1], scratch[i][2]];
        let along = dot(normal, sum);
        let mut tangent = std::array::from_fn::<_, 3, _>(|k| sum[k] - normal[k] * along);
        if dot(tangent, tangent) < 1e-12 || tangent.iter().any(|v| !v.is_finite()) {
            let axis = if normal[0].abs() < 0.8 {
                [1.0, 0.0, 0.0]
            } else {
                [0.0, 1.0, 0.0]
            };
            tangent = cross(axis, normal);
        }
        tangent = unit(tangent);
        let bitangent = [scratch[i][3], scratch[i][4], scratch[i][5]];
        out[i] = [
            tangent[0],
            tangent[1],
            tangent[2],
            if dot(cross(normal, tangent), bitangent) < 0.0 {
                -1.0
            } else {
                1.0
            },
        ];
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swapped_uv_axes_rotate_tangent_and_preserve_mirrored_handedness() {
        let p = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let mut out = [[0.0; 4]; 3];
        let mut scratch = [[0.0; 6]; 3];
        tangents_from_uvs_into(
            &p,
            &[[0.0, 0.0], [0.0, 1.0], [1.0, 0.0]],
            &[[0.0, 0.0, 1.0]; 3],
            &[0, 1, 2],
            &mut out,
            &mut scratch,
        )
        .unwrap();
        assert_eq!(out, [[0.0, 1.0, 0.0, -1.0]; 3]);
    }
    #[test]
    fn invalid_index_does_not_touch_caller_buffers() {
        let mut out = [[9.0; 4]; 3];
        let mut scratch = [[8.0; 6]; 3];
        assert_eq!(
            tangents_from_uvs_into(
                &[[0.0; 3]; 3],
                &[[0.0; 2]; 3],
                &[[0.0, 0.0, 1.0]; 3],
                &[0, 1, 9],
                &mut out,
                &mut scratch
            ),
            Err(TangentFrameError::InvalidInput)
        );
        assert_eq!(out, [[9.0; 4]; 3]);
        assert_eq!(scratch, [[8.0; 6]; 3]);
    }

    #[test]
    fn strict_mirrored_chart_has_negative_handedness() {
        let mut out = [[9.; 4]; 3];
        let mut temporary = [[0.; 4]; 3];
        let mut scratch = [[0.; 6]; 3];
        let n = tangents_from_uvs_strict_into(
            &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            &[[0., 0.], [0., 1.], [1., 0.]],
            &[[0., 0., 1.]; 3],
            &[0, 1, 2],
            400,
            &mut out,
            &mut temporary,
            &mut scratch,
        )
        .unwrap();
        assert_eq!(n, 3);
        assert_eq!(out, [[0., 1., 0., -1.]; 3]);
    }

    #[test]
    fn strict_mixed_orientation_requires_split_without_publication() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
        let mut out = [[9.; 4]; 4];
        let mut temporary = [[0.; 4]; 4];
        let mut scratch = [[0.; 6]; 4];
        assert_eq!(
            tangents_from_uvs_strict_into(
                &positions,
                &[[0., 0.], [1., 0.], [1., 1.], [1., 0.]],
                &[[0., 0., 1.]; 4],
                &[0, 1, 2, 0, 2, 3],
                600,
                &mut out,
                &mut temporary,
                &mut scratch
            ),
            Err(StrictTangentFrameError::MixedOrientation)
        );
        assert_eq!(out, [[9.; 4]; 4]);
    }

    #[test]
    fn strict_degenerate_uv_and_budget_do_not_publish() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let mut out = [[9.; 4]; 3];
        let mut temporary = [[0.; 4]; 3];
        let mut scratch = [[0.; 6]; 3];
        assert_eq!(
            tangents_from_uvs_strict_into(
                &positions,
                &[[0., 0.]; 3],
                &[[0., 0., 1.]; 3],
                &[0, 1, 2],
                400,
                &mut out,
                &mut temporary,
                &mut scratch
            ),
            Err(StrictTangentFrameError::DegenerateUv)
        );
        assert_eq!(
            tangents_from_uvs_strict_into(
                &positions,
                &[[0., 0.], [1., 0.], [0., 1.]],
                &[[0., 0., 1.]; 3],
                &[0, 1, 2],
                399,
                &mut out,
                &mut temporary,
                &mut scratch
            ),
            Err(StrictTangentFrameError::Budget)
        );
        assert_eq!(out, [[9.; 4]; 3]);
    }
}
