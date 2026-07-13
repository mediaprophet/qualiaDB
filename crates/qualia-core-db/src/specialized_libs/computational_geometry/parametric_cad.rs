//! P18 - Parametric CAD, procedural lattices, and shape optimisation.

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NurbsControlPoint {
    pub point: Point3,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeDistance {
    pub chamfer: f64,
    pub hausdorff: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSample {
    pub position: Point3,
    pub du: Point3,
    pub dv: Point3,
    pub normal: Point3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuityReport {
    pub c0: bool,
    pub c1: bool,
    pub g1: bool,
    pub tangent_angle: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SdfPrimitive {
    Sphere {
        center: Point3,
        radius: f64,
    },
    Box {
        center: Point3,
        half_extents: Point3,
    },
    Plane {
        normal: Point3,
        offset: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdfOp {
    Union,
    Intersection,
    Difference,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterConstraint {
    pub min: f64,
    pub max: f64,
    pub integer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurrogatePrediction {
    pub value: f64,
    pub uncertainty: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimizationReport {
    pub best_index: u32,
    pub best_value: f64,
    pub evaluations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainAdapterNotice {
    pub generated_structural_geometry: bool,
    pub requires_human_attestation: bool,
    pub clinically_validated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadError {
    InvalidInput,
    OutputTooSmall { required: usize },
}

pub fn bezier_eval(control: &[Point3], t: f64) -> Result<Point3, CadError> {
    if control.is_empty() || !t.is_finite() {
        return Err(CadError::InvalidInput);
    }
    let mut tmp = control.to_vec();
    let t = t.clamp(0.0, 1.0);
    for r in 1..control.len() {
        for i in 0..control.len() - r {
            tmp[i] = lerp(tmp[i], tmp[i + 1], t);
        }
    }
    Ok(tmp[0])
}

pub fn bezier_derivative_eval(control: &[Point3], t: f64) -> Result<Point3, CadError> {
    if control.len() < 2 {
        return Err(CadError::InvalidInput);
    }
    let degree = control.len() - 1;
    let mut derived = Vec::with_capacity(degree);
    for i in 0..degree {
        derived.push(scale(sub(control[i + 1], control[i]), degree as f64));
    }
    bezier_eval(&derived, t)
}

pub fn bspline_eval(control: &[Point3], degree: usize, t: f64) -> Result<Point3, CadError> {
    if control.is_empty() || degree == 0 || degree >= control.len() || !t.is_finite() {
        return Err(CadError::InvalidInput);
    }
    let n = control.len();
    let spans = n - degree;
    let u = t.clamp(0.0, 1.0) * spans as f64;
    let span = (u.floor() as usize).min(spans - 1);
    let local_t = u - span as f64;
    bezier_eval(&control[span..=span + degree], local_t)
}

pub fn bspline_insert_uniform_knot(
    control: &[Point3],
    degree: usize,
    t: f64,
    out: &mut [Point3],
) -> Result<usize, CadError> {
    if control.is_empty() || degree == 0 || degree >= control.len() || out.len() < control.len() + 1
    {
        return Err(CadError::InvalidInput);
    }
    let spans = control.len() - degree;
    let u = t.clamp(0.0, 1.0) * spans as f64;
    let span = (u.floor() as usize).min(spans - 1);
    let alpha = u - span as f64;
    for i in 0..=span {
        out[i] = control[i];
    }
    out[span + 1] = lerp(
        control[span],
        control[(span + 1).min(control.len() - 1)],
        alpha,
    );
    for i in span + 1..control.len() {
        out[i + 1] = control[i];
    }
    Ok(control.len() + 1)
}

pub fn nurbs_eval(
    control: &[NurbsControlPoint],
    degree: usize,
    t: f64,
) -> Result<Point3, CadError> {
    if control.is_empty() {
        return Err(CadError::InvalidInput);
    }
    let weighted: Vec<Point3> = control.iter().map(|c| scale(c.point, c.weight)).collect();
    let weights: Vec<Point3> = control
        .iter()
        .map(|c| Point3::new(c.weight, 0.0, 0.0))
        .collect();
    let p = bspline_eval(&weighted, degree, t)?;
    let w = bspline_eval(&weights, degree, t)?.x;
    if w == 0.0 {
        return Err(CadError::InvalidInput);
    }
    Ok(scale(p, 1.0 / w))
}

pub fn tensor_surface_eval(
    grid: &[Point3],
    u_count: usize,
    v_count: usize,
    u: f64,
    v: f64,
) -> Result<Point3, CadError> {
    if grid.len() != u_count * v_count || u_count < 2 || v_count < 2 {
        return Err(CadError::InvalidInput);
    }
    let uu = u.clamp(0.0, 1.0) * (u_count - 1) as f64;
    let vv = v.clamp(0.0, 1.0) * (v_count - 1) as f64;
    let i = (uu.floor() as usize).min(u_count - 2);
    let j = (vv.floor() as usize).min(v_count - 2);
    let fu = uu - i as f64;
    let fv = vv - j as f64;
    let p00 = grid[j * u_count + i];
    let p10 = grid[j * u_count + i + 1];
    let p01 = grid[(j + 1) * u_count + i];
    let p11 = grid[(j + 1) * u_count + i + 1];
    Ok(lerp(lerp(p00, p10, fu), lerp(p01, p11, fu), fv))
}

pub fn tensor_surface_sample(
    grid: &[Point3],
    u_count: usize,
    v_count: usize,
    u: f64,
    v: f64,
) -> Result<SurfaceSample, CadError> {
    let position = tensor_surface_eval(grid, u_count, v_count, u, v)?;
    let eps = 1e-5;
    let pu0 = tensor_surface_eval(grid, u_count, v_count, (u - eps).clamp(0.0, 1.0), v)?;
    let pu1 = tensor_surface_eval(grid, u_count, v_count, (u + eps).clamp(0.0, 1.0), v)?;
    let pv0 = tensor_surface_eval(grid, u_count, v_count, u, (v - eps).clamp(0.0, 1.0))?;
    let pv1 = tensor_surface_eval(grid, u_count, v_count, u, (v + eps).clamp(0.0, 1.0))?;
    let du = scale(sub(pu1, pu0), 0.5 / eps);
    let dv = scale(sub(pv1, pv0), 0.5 / eps);
    Ok(SurfaceSample {
        position,
        du,
        dv,
        normal: normalize(cross(du, dv)),
    })
}

pub fn classify_trim_uv(point: [f64; 2], loop_uv: &[[f64; 2]]) -> Result<bool, CadError> {
    if loop_uv.len() < 3 {
        return Err(CadError::InvalidInput);
    }
    let mut inside = false;
    let mut j = loop_uv.len() - 1;
    for i in 0..loop_uv.len() {
        let pi = loop_uv[i];
        let pj = loop_uv[j];
        if ((pi[1] > point[1]) != (pj[1] > point[1]))
            && point[0]
                < (pj[0] - pi[0]) * (point[1] - pi[1]) / ((pj[1] - pi[1]).max(1e-12)) + pi[0]
        {
            inside = !inside;
        }
        j = i;
    }
    Ok(inside)
}

pub fn continuity_between_curves(
    a: &[Point3],
    b: &[Point3],
    tolerance: f64,
) -> Result<ContinuityReport, CadError> {
    if a.len() < 2 || b.len() < 2 || !(tolerance.is_finite() && tolerance >= 0.0) {
        return Err(CadError::InvalidInput);
    }
    let pa = *a.last().unwrap();
    let pb = b[0];
    let ta = normalize(sub(a[a.len() - 1], a[a.len() - 2]));
    let tb = normalize(sub(b[1], b[0]));
    let dot_t = dot(ta, tb).clamp(-1.0, 1.0);
    Ok(ContinuityReport {
        c0: distance_sq(pa, pb).sqrt() <= tolerance,
        c1: distance_sq(ta, tb).sqrt() <= tolerance,
        g1: dot_t >= 1.0 - tolerance,
        tangent_angle: dot_t.acos(),
    })
}

pub fn tube_along_polyline(
    path: &[Point3],
    radius: f64,
    sides: usize,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), CadError> {
    if path.len() < 2 || sides < 3 || !(radius.is_finite() && radius > 0.0) {
        return Err(CadError::InvalidInput);
    }
    let required_v = path.len() * sides;
    let required_t = (path.len() - 1) * sides * 2;
    if out_vertices.len() < required_v || out_triangles.len() < required_t {
        return Err(CadError::OutputTooSmall {
            required: required_v.max(required_t),
        });
    }
    for (i, p) in path.iter().enumerate() {
        for s in 0..sides {
            let a = core::f64::consts::TAU * s as f64 / sides as f64;
            out_vertices[i * sides + s] =
                Point3::new(p.x + radius * a.cos(), p.y + radius * a.sin(), p.z);
        }
    }
    let mut ti = 0usize;
    for i in 0..path.len() - 1 {
        for s in 0..sides {
            let a = (i * sides + s) as u32;
            let b = (i * sides + (s + 1) % sides) as u32;
            let c = ((i + 1) * sides + (s + 1) % sides) as u32;
            let d = ((i + 1) * sides + s) as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }
    Ok((required_v, required_t))
}

pub fn revolve_profile(
    profile: &[Point3],
    segments: usize,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), CadError> {
    if profile.len() < 2 || segments < 3 {
        return Err(CadError::InvalidInput);
    }
    let required_v = profile.len() * segments;
    let required_t = (profile.len() - 1) * segments * 2;
    if out_vertices.len() < required_v || out_triangles.len() < required_t {
        return Err(CadError::OutputTooSmall {
            required: required_v.max(required_t),
        });
    }
    for s in 0..segments {
        let a = core::f64::consts::TAU * s as f64 / segments as f64;
        let (ca, sa) = (a.cos(), a.sin());
        for (i, p) in profile.iter().enumerate() {
            out_vertices[s * profile.len() + i] = Point3::new(p.x * ca, p.y, p.x * sa);
        }
    }
    let mut ti = 0usize;
    for s in 0..segments {
        let ns = (s + 1) % segments;
        for i in 0..profile.len() - 1 {
            let a = (s * profile.len() + i) as u32;
            let b = (ns * profile.len() + i) as u32;
            let c = (ns * profile.len() + i + 1) as u32;
            let d = (s * profile.len() + i + 1) as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }
    Ok((required_v, required_t))
}

pub fn loft_profiles(
    a: &[Point3],
    b: &[Point3],
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), CadError> {
    if a.len() != b.len() || a.len() < 2 {
        return Err(CadError::InvalidInput);
    }
    let required_v = a.len() * 2;
    let required_t = (a.len() - 1) * 2;
    if out_vertices.len() < required_v || out_triangles.len() < required_t {
        return Err(CadError::OutputTooSmall {
            required: required_v.max(required_t),
        });
    }
    out_vertices[..a.len()].copy_from_slice(a);
    out_vertices[a.len()..required_v].copy_from_slice(b);
    let mut ti = 0usize;
    for i in 0..a.len() - 1 {
        out_triangles[ti] = [i as u32, (i + 1) as u32, (a.len() + i + 1) as u32];
        out_triangles[ti + 1] = [i as u32, (a.len() + i + 1) as u32, (a.len() + i) as u32];
        ti += 2;
    }
    Ok((required_v, required_t))
}

pub fn offset_polyline(
    path: &[Point3],
    distance: f64,
    out: &mut [Point3],
) -> Result<usize, CadError> {
    if path.len() < 2 || !distance.is_finite() || out.len() < path.len() {
        return Err(CadError::InvalidInput);
    }
    for i in 0..path.len() {
        let dir = if i + 1 < path.len() {
            sub(path[i + 1], path[i])
        } else {
            sub(path[i], path[i - 1])
        };
        let n = normalize(Point3::new(-dir.y, dir.x, 0.0));
        out[i] = add(path[i], scale(n, distance));
    }
    Ok(path.len())
}

pub fn sdf_eval(p: Point3, primitive: SdfPrimitive) -> f64 {
    match primitive {
        SdfPrimitive::Sphere { center, radius } => distance_sq(p, center).sqrt() - radius,
        SdfPrimitive::Box {
            center,
            half_extents,
        } => {
            let q = Point3::new(
                (p.x - center.x).abs() - half_extents.x,
                (p.y - center.y).abs() - half_extents.y,
                (p.z - center.z).abs() - half_extents.z,
            );
            let outside = Point3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0));
            distance_sq(outside, Point3::new(0.0, 0.0, 0.0)).sqrt() + q.x.max(q.y.max(q.z)).min(0.0)
        }
        SdfPrimitive::Plane { normal, offset } => dot(normalize(normal), p) + offset,
    }
}

pub fn sdf_compose(a: f64, b: f64, op: SdfOp) -> f64 {
    match op {
        SdfOp::Union => a.min(b),
        SdfOp::Intersection => a.max(b),
        SdfOp::Difference => a.max(-b),
    }
}

pub fn helical_lattice(
    radius: f64,
    pitch: f64,
    turns: usize,
    samples_per_turn: usize,
    out: &mut [Point3],
) -> Result<usize, CadError> {
    if turns == 0 || samples_per_turn < 3 || !(radius.is_finite() && pitch.is_finite()) {
        return Err(CadError::InvalidInput);
    }
    let count = turns * samples_per_turn + 1;
    if out.len() < count {
        return Err(CadError::OutputTooSmall { required: count });
    }
    for (i, slot) in out.iter_mut().take(count).enumerate() {
        let t = i as f64 / samples_per_turn as f64;
        let a = core::f64::consts::TAU * t;
        *slot = Point3::new(radius * a.cos(), radius * a.sin(), pitch * t);
    }
    Ok(count)
}

pub fn validate_parameters(
    values: &[f64],
    constraints: &[ParameterConstraint],
) -> Result<(), CadError> {
    if values.len() != constraints.len() {
        return Err(CadError::InvalidInput);
    }
    for (&v, c) in values.iter().zip(constraints.iter()) {
        if !v.is_finite() || v < c.min || v > c.max || (c.integer && (v.round() - v).abs() > 1e-9) {
            return Err(CadError::InvalidInput);
        }
    }
    Ok(())
}

pub fn latin_hypercube_samples(
    dimensions: usize,
    samples: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, CadError> {
    let required = dimensions.saturating_mul(samples);
    if dimensions == 0 || samples == 0 || out.len() < required {
        return Err(CadError::InvalidInput);
    }
    let mut state = seed;
    for d in 0..dimensions {
        let mut strata: Vec<usize> = (0..samples).collect();
        for i in (1..samples).rev() {
            let j = (splitmix64(&mut state) as usize) % (i + 1);
            strata.swap(i, j);
        }
        for s in 0..samples {
            out[s * dimensions + d] = (strata[s] as f64 + splitmix01(&mut state)) / samples as f64;
        }
    }
    Ok(required)
}

pub fn rbf_surrogate_predict(
    samples: &[Point3],
    values: &[f64],
    query: Point3,
    radius: f64,
) -> Result<SurrogatePrediction, CadError> {
    if samples.is_empty() || samples.len() != values.len() || !(radius.is_finite() && radius > 0.0)
    {
        return Err(CadError::InvalidInput);
    }
    let mut wsum = 0.0;
    let mut value = 0.0;
    for (&p, &v) in samples.iter().zip(values.iter()) {
        let w = (-distance_sq(p, query) / (radius * radius)).exp();
        wsum += w;
        value += w * v;
    }
    if wsum == 0.0 {
        return Err(CadError::InvalidInput);
    }
    let value = value / wsum;
    let mut variance = 0.0;
    for (&p, &v) in samples.iter().zip(values.iter()) {
        let w = (-distance_sq(p, query) / (radius * radius)).exp();
        variance += w * (v - value) * (v - value);
    }
    Ok(SurrogatePrediction {
        value,
        uncertainty: (variance / wsum).sqrt(),
    })
}

pub fn budgeted_evolution_optimize<F>(
    candidates: &[Point3],
    budget: usize,
    objective: F,
) -> Result<OptimizationReport, CadError>
where
    F: Fn(Point3) -> f64,
{
    if candidates.is_empty() || budget == 0 {
        return Err(CadError::InvalidInput);
    }
    let evals = budget.min(candidates.len());
    let mut best_index = 0u32;
    let mut best_value = objective(candidates[0]);
    for (i, &candidate) in candidates.iter().take(evals).enumerate().skip(1) {
        let value = objective(candidate);
        if value < best_value {
            best_value = value;
            best_index = i as u32;
        }
    }
    Ok(OptimizationReport {
        best_index,
        best_value,
        evaluations: evals as u32,
    })
}

pub fn vascular_lattice_adapter_notice() -> DomainAdapterNotice {
    DomainAdapterNotice {
        generated_structural_geometry: true,
        requires_human_attestation: true,
        clinically_validated: false,
    }
}

pub fn shape_distance(a: &[Point3], b: &[Point3]) -> Result<ShapeDistance, CadError> {
    if a.is_empty() || b.is_empty() {
        return Err(CadError::InvalidInput);
    }
    let ab: Vec<f64> = a.iter().map(|&p| nearest_distance(p, b)).collect();
    let ba: Vec<f64> = b.iter().map(|&p| nearest_distance(p, a)).collect();
    let chamfer =
        (ab.iter().sum::<f64>() / ab.len() as f64 + ba.iter().sum::<f64>() / ba.len() as f64) * 0.5;
    let hausdorff = ab.iter().chain(ba.iter()).fold(0.0f64, |m, &x| m.max(x));
    Ok(ShapeDistance { chamfer, hausdorff })
}

fn nearest_distance(p: Point3, qs: &[Point3]) -> f64 {
    qs.iter()
        .map(|&q| distance_sq(p, q))
        .fold(f64::INFINITY, f64::min)
        .sqrt()
}

fn lerp(a: Point3, b: Point3, t: f64) -> Point3 {
    add(scale(a, 1.0 - t), scale(b, t))
}

fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}

fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn normalize(a: Point3) -> Point3 {
    let n = dot(a, a).sqrt();
    if n > 0.0 {
        scale(a, 1.0 / n)
    } else {
        Point3::new(0.0, 0.0, 0.0)
    }
}

fn distance_sq(a: Point3, b: Point3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn splitmix01(state: &mut u64) -> f64 {
    ((splitmix64(state) >> 11) as f64) / ((1u64 << 53) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_endpoints_match_controls() {
        let c = [Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        assert_eq!(bezier_eval(&c, 0.0).unwrap(), c[0]);
        assert_eq!(bezier_eval(&c, 1.0).unwrap(), c[1]);
    }

    #[test]
    fn tube_generates_expected_counts() {
        let path = [Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)];
        let mut v = vec![Point3::new(0.0, 0.0, 0.0); 16];
        let mut t = vec![[0u32; 3]; 16];
        assert_eq!(
            tube_along_polyline(&path, 0.1, 4, &mut v, &mut t).unwrap(),
            (8, 8)
        );
    }

    #[test]
    fn identical_shapes_have_zero_distance() {
        let p = [Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        let d = shape_distance(&p, &p).unwrap();
        assert_eq!(d.chamfer, 0.0);
        assert_eq!(d.hausdorff, 0.0);
    }
}
