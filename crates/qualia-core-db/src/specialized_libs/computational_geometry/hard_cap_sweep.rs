//! Optional caller-buffered normal derivative for the existing capped sweep.
//!
//! This creates a new indexed derivative; it never edits an authored mesh or
//! its attributes. Cap rims are split from wall rims so `vertex_normals` does
//! not average across that hard boundary. `source_vertices` maps each output
//! vertex to the corresponding base-sweep vertex for explicit attribute
//! migration by the caller; it is not a persistent semantic identifier.
//! Output buffers are scratch until `Ok`; a kernel/geometry error can leave
//! partial writes. The preflight/buffer failures occur before any writes.

use super::ddg_operators::vertex_normals;
use super::math_geometry::AffineFrame3;
use super::primitives::{Point2, Point3};
use super::sweep_extrusion::{extrude_profile_along_path, ExtrusionStation, SweepError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardCapSweepError {
    InvalidInput,
    SizeOverflow,
    OutputTooSmall { vertices: usize, triangles: usize, frames: usize },
    Sweep(SweepError),
    DegenerateFace,
    DegenerateNormal,
    NormalKernel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardCapCounts { pub vertices: usize, pub triangles: usize, pub frames: usize }

/// Counts include two duplicated cap rims and both original centroid vertices.
pub fn hard_cap_sweep_counts(stations: usize, sides: usize) -> Result<HardCapCounts, HardCapSweepError> {
    if stations < 2 || !(3..=64).contains(&sides) { return Err(HardCapSweepError::InvalidInput); }
    let vertices = stations.checked_add(2).and_then(|n| n.checked_mul(sides)).and_then(|n| n.checked_add(2)).ok_or(HardCapSweepError::SizeOverflow)?;
    let triangles = stations.checked_mul(sides).and_then(|n| n.checked_mul(2)).ok_or(HardCapSweepError::SizeOverflow)?;
    if u32::try_from(vertices).is_err() { return Err(HardCapSweepError::SizeOverflow); }
    Ok(HardCapCounts { vertices, triangles, frames: stations })
}

fn finite3(p: Point3) -> bool { p.x.is_finite() && p.y.is_finite() && p.z.is_finite() }
fn sub3(a: Point3, b: Point3) -> Point3 { Point3::new(a.x-b.x,a.y-b.y,a.z-b.z) }
fn add3(a: Point3, b: Point3) -> Point3 { Point3::new(a.x+b.x,a.y+b.y,a.z+b.z) }
fn cross3(a: Point3, b: Point3) -> Point3 { Point3::new(a.y*b.z-a.z*b.y,a.z*b.x-a.x*b.z,a.x*b.y-a.y*b.x) }
fn orient(a: Point2, b: Point2, c: Point2) -> f64 { (b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x) }
// The existing centroid fan is valid for a strictly convex simple profile.
// Limit sections to 64 points so the intersection check is explicitly bounded.
fn strict_ccw_convex(section: &[Point2]) -> bool {
    let n = section.len();
    for i in 0..n {
        let turn = orient(section[i], section[(i+1)%n], section[(i+2)%n]);
        if !turn.is_finite() || turn <= 0.0 { return false; }
    }
    for i in 0..n {
        for j in i+1..n {
            if j == i+1 || (i == 0 && j == n-1) { continue; }
            let (a,b,c,d) = (section[i],section[(i+1)%n],section[j],section[(j+1)%n]);
            let (o1,o2,o3,o4)=(orient(a,b,c),orient(a,b,d),orient(c,d,a),orient(c,d,b));
            if [o1,o2,o3,o4].iter().any(|x| !x.is_finite() || *x == 0.0) { return false; }
            if (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0) { return false; }
        }
    }
    true
}
fn area2(a: Point3, b: Point3, c: Point3) -> f64 {
    let u = [b.x-a.x,b.y-a.y,b.z-a.z];
    let v = [c.x-a.x,c.y-a.y,c.z-a.z];
    let cross = [u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
    cross[0]*cross[0]+cross[1]*cross[1]+cross[2]*cross[2]
}

/// Generate the existing closed capped sweep then produce a hard-cap split and
/// area-weighted quad-smooth wall normals and triangle-weighted cap normals.
/// Each quad contributes its two-triangle area sum to all four wall vertices,
/// avoiding a dependency on its arbitrary triangulation diagonal. Geometry, normals, index and mapping
/// buffers belong to the caller; no heap allocation is performed here. This is
/// an opt-in derivative, not a change to `extrude_profile_along_path` output.
/// The duplicated boundary is a render-attribute seam: positions coincide,
/// but index topology has open boundaries at the two rims. Keep the original
/// sweep as the source/topological mesh when watertight incidence is needed.
/// Profiles are limited to 3–64 strictly convex, simple CCW cross-sections.
/// Centerline self-intersections,
/// collision, fork joining, UV quality and visual acceptance are not proved.
pub fn hard_cap_sweep_normals_into(
    axis: &[Point3], section: &[Point2], stations: Option<&[ExtrusionStation]>,
    frames: &mut [AffineFrame3], vertices: &mut [Point3], triangles: &mut [[u32; 3]],
    normals: &mut [Point3], source_vertices: &mut [u32],
) -> Result<HardCapCounts, HardCapSweepError> {
    let counts = hard_cap_sweep_counts(axis.len(), section.len())?;
    if frames.len() < counts.frames || vertices.len() < counts.vertices || triangles.len() < counts.triangles || normals.len() < counts.vertices || source_vertices.len() < counts.vertices {
        return Err(HardCapSweepError::OutputTooSmall { vertices: counts.vertices, triangles: counts.triangles, frames: counts.frames });
    }
    if axis.iter().copied().any(|p| !finite3(p)) || section.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err(HardCapSweepError::InvalidInput);
    }
    if !strict_ccw_convex(section) { return Err(HardCapSweepError::InvalidInput); }
    if let Some(stations) = stations {
        if stations.len() != axis.len() || stations.iter().any(|s| !s.scale_x.is_finite() || !s.scale_y.is_finite() || !s.twist_rad.is_finite() || s.scale_x <= 0.0 || s.scale_y <= 0.0) {
            return Err(HardCapSweepError::InvalidInput);
        }
    }
    let sides = section.len();
    let base_rings = axis.len().checked_mul(sides).ok_or(HardCapSweepError::SizeOverflow)?;
    let base_vertices = base_rings.checked_add(2).ok_or(HardCapSweepError::SizeOverflow)?;
    let (nv, nt) = extrude_profile_along_path(axis, section, stations, true, true, frames, &mut vertices[..base_vertices], &mut triangles[..counts.triangles]).map_err(HardCapSweepError::Sweep)?;
    if nv != base_vertices || nt != counts.triangles { return Err(HardCapSweepError::InvalidInput); }
    for (i, out) in source_vertices[..base_vertices].iter_mut().enumerate() { *out = i as u32; }
    let bottom = base_vertices;
    let top = bottom + sides;
    let last_ring = (axis.len() - 1) * sides;
    for i in 0..sides {
        vertices[bottom+i] = vertices[i];
        vertices[top+i] = vertices[last_ring+i];
        source_vertices[bottom+i] = i as u32;
        source_vertices[top+i] = (last_ring+i) as u32;
    }
    let side_triangles = (axis.len()-1) * sides * 2;
    for i in 0..sides {
        let next=(i+1)%sides;
        triangles[side_triangles+i] = [base_rings as u32, (bottom+next) as u32, (bottom+i) as u32];
        triangles[side_triangles+sides+i] = [(base_rings+1) as u32, (top+i) as u32, (top+next) as u32];
    }
    if vertices[..counts.vertices].iter().copied().any(|p| !finite3(p)) { return Err(HardCapSweepError::InvalidInput); }
    for tri in &triangles[..counts.triangles] {
        let area = area2(vertices[tri[0] as usize], vertices[tri[1] as usize], vertices[tri[2] as usize]);
        if !area.is_finite() || area <= 0.0 { return Err(HardCapSweepError::DegenerateFace); }
    }
    vertex_normals(&vertices[..counts.vertices], &triangles[..counts.triangles], &mut normals[..counts.vertices]).map_err(|_| HardCapSweepError::NormalKernel)?;
    // Triangle-wise accumulation biases opposite end rings of a planar sweep
    // because the diagonal contributes twice to some corners. Attribute one
    // oriented area vector per wall quad to each of its four corners instead.
    normals[..base_rings].fill(Point3::new(0.0,0.0,0.0));
    for station in 0..axis.len()-1 {
        for j in 0..sides {
            let next=(j+1)%sides;
            let a=station*sides+j; let b=station*sides+next;
            let c=(station+1)*sides+next; let d=(station+1)*sides+j;
            let area=add3(cross3(sub3(vertices[b],vertices[a]),sub3(vertices[c],vertices[a])),
                          cross3(sub3(vertices[c],vertices[a]),sub3(vertices[d],vertices[a])));
            let mag2=area.x*area.x+area.y*area.y+area.z*area.z;
            if !mag2.is_finite() || mag2<=0.0 { return Err(HardCapSweepError::DegenerateNormal); }
            for index in [a,b,c,d] { normals[index]=add3(normals[index],area); }
        }
    }
    for normal in &mut normals[..base_rings] {
        let mag2=normal.x*normal.x+normal.y*normal.y+normal.z*normal.z;
        if !mag2.is_finite() || mag2<=0.0 { return Err(HardCapSweepError::DegenerateNormal); }
        let mag=mag2.sqrt();
        *normal=Point3::new(normal.x/mag,normal.y/mag,normal.z/mag);
    }
    if normals[..counts.vertices].iter().any(|p| !finite3(*p) || p.x*p.x+p.y*p.y+p.z*p.z <= 0.0) { return Err(HardCapSweepError::DegenerateNormal); }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x:f64,y:f64,z:f64)->Point3 { Point3::new(x,y,z) }
    fn frame()->AffineFrame3 { AffineFrame3 { origin:p(0.,0.,0.), e0:p(0.,0.,0.), e1:p(0.,0.,0.), e2:p(0.,0.,0.) } }
    fn section()->[Point2;4] { [Point2::new(1.,0.), Point2::new(0.,1.), Point2::new(-1.,0.), Point2::new(0.,-1.)] }
    #[test]
    fn wall_normals_are_continuous_and_cap_normals_are_hard_split() {
        let axis=[p(0.,0.,0.),p(0.,0.,1.),p(0.,0.,2.)]; let section=section(); let c=hard_cap_sweep_counts(3,4).unwrap();
        let mut frames=vec![frame();c.frames]; let mut v=vec![p(0.,0.,0.);c.vertices]; let mut t=vec![[0;3];c.triangles]; let mut n=v.clone(); let mut map=vec![0;c.vertices];
        assert_eq!(hard_cap_sweep_normals_into(&axis,&section,None,&mut frames,&mut v,&mut t,&mut n,&mut map).unwrap(),c);
        assert_eq!(c.vertices,22); assert_eq!(c.triangles,24);
        for i in 0..12 {
            assert!(n[i].z.abs()<1e-12); assert!((n[i].x*n[i].x+n[i].y*n[i].y-1.).abs()<1e-12);
            let radial=sub3(v[i],frames[i/4].origin);
            let dot=n[i].x*radial.x+n[i].y*radial.y+n[i].z*radial.z;
            assert!(dot>0.999999,"wall normal is not outward radial");
            let reference=n[i%4];
            assert!((n[i].x-reference.x).abs()<1e-12 && (n[i].y-reference.y).abs()<1e-12 && (n[i].z-reference.z).abs()<1e-12, "wall normals differ at the same angular index across stations");
        }
        for i in 0..4 { assert_eq!(v[14+i],v[i]); assert_eq!(map[14+i],i as u32); assert!(n[14+i].z < -0.999999); assert_eq!(v[18+i],v[8+i]); assert_eq!(map[18+i],(8+i) as u32); assert!(n[18+i].z > 0.999999); }
        for tri in &t[..16] { assert!(tri.iter().all(|i| *i < 12)); }
        for tri in &t[16..20] { assert_eq!(tri[0],12); assert!(tri[1]>=14 && tri[1]<18 && tri[2]>=14 && tri[2]<18); }
        for tri in &t[20..24] { assert_eq!(tri[0],13); assert!(tri[1]>=18 && tri[1]<22 && tri[2]>=18 && tri[2]<22); }
    }
    #[test]
    fn taper_and_gentle_bend_keep_outward_finite_continuous_wall_normals() {
        let axis=[p(0.,0.,0.),p(0.,0.,1.),p(0.3,0.,2.)]; let section=section();
        let stations=[ExtrusionStation { scale_x:1.0,scale_y:1.0,twist_rad:0.0 },ExtrusionStation { scale_x:0.8,scale_y:0.8,twist_rad:0.0 },ExtrusionStation { scale_x:0.6,scale_y:0.6,twist_rad:0.0 }];
        let c=hard_cap_sweep_counts(3,4).unwrap(); let mut frames=vec![frame();c.frames]; let mut v=vec![p(0.,0.,0.);c.vertices]; let mut t=vec![[0;3];c.triangles]; let mut n=v.clone(); let mut map=vec![0;c.vertices];
        hard_cap_sweep_normals_into(&axis,&section,Some(&stations),&mut frames,&mut v,&mut t,&mut n,&mut map).unwrap();
        for i in 0..12 {
            let radial=sub3(v[i],frames[i/4].origin); let dot=n[i].x*radial.x+n[i].y*radial.y+n[i].z*radial.z;
            assert!(dot>0.0,"wall normal must point outward");
            assert!((n[i].x*n[i].x+n[i].y*n[i].y+n[i].z*n[i].z-1.0).abs()<1e-12);
            if i>=4 { let prev=n[i-4]; assert!(n[i].x*prev.x+n[i].y*prev.y+n[i].z*prev.z > 0.7, "normal continuity violated by gentle bend"); }
        }
    }
    #[test]
    fn preflight_and_invalid_inputs_refuse_without_writing() {
        let axis=[p(0.,0.,0.),p(0.,0.,1.)]; let section=section(); let sentinel=p(19.,19.,19.);
        let mut frames=vec![frame();2]; let mut v=vec![sentinel;2]; let mut t=vec![[77;3];1]; let mut n=v.clone(); let mut map=vec![99;2];
        assert!(matches!(hard_cap_sweep_normals_into(&axis,&section,None,&mut frames,&mut v,&mut t,&mut n,&mut map),Err(HardCapSweepError::OutputTooSmall { .. })));
        assert_eq!(v,vec![sentinel;2]); assert_eq!(n,vec![sentinel;2]); assert_eq!(t,vec![[77;3];1]); assert_eq!(map,vec![99;2]);
        assert_eq!(hard_cap_sweep_counts(usize::MAX,4),Err(HardCapSweepError::SizeOverflow));
        let c=hard_cap_sweep_counts(2,4).unwrap(); let mut v=vec![sentinel;c.vertices]; let mut n=v.clone(); let mut t=vec![[77;3];c.triangles]; let mut map=vec![99;c.vertices];
        let invalid=[p(f64::NAN,0.,0.),p(0.,0.,1.)];
        assert_eq!(hard_cap_sweep_normals_into(&invalid,&section,None,&mut frames,&mut v,&mut t,&mut n,&mut map),Err(HardCapSweepError::InvalidInput));
        assert!(v.iter().all(|x| *x==sentinel)); assert!(t.iter().all(|x| *x==[77;3]));
        let clockwise=[section[0],section[3],section[2],section[1]];
        assert_eq!(hard_cap_sweep_normals_into(&axis,&clockwise,None,&mut frames,&mut v,&mut t,&mut n,&mut map),Err(HardCapSweepError::InvalidInput));
        assert!(v.iter().all(|x| *x==sentinel)); assert!(t.iter().all(|x| *x==[77;3]));
        let stations=[ExtrusionStation { scale_x:-1.,scale_y:1.,twist_rad:0. },ExtrusionStation::default()];
        assert_eq!(hard_cap_sweep_normals_into(&axis,&section,Some(&stations),&mut frames,&mut v,&mut t,&mut n,&mut map),Err(HardCapSweepError::InvalidInput));
        assert!(v.iter().all(|x| *x==sentinel)); assert!(t.iter().all(|x| *x==[77;3]));
    }
    #[test]
    fn reversed_or_degenerate_geometry_is_refused() {
        let axis=[p(0.,0.,0.),p(0.,0.,1.),p(0.,0.,0.)]; let section=section(); let c=hard_cap_sweep_counts(3,4).unwrap();
        let mut frames=vec![frame();c.frames]; let mut v=vec![p(0.,0.,0.);c.vertices]; let mut t=vec![[0;3];c.triangles]; let mut n=v.clone(); let mut map=vec![0;c.vertices];
        assert!(hard_cap_sweep_normals_into(&axis,&section,None,&mut frames,&mut v,&mut t,&mut n,&mut map).is_err());
        let degenerate=[Point2::new(1.,0.),Point2::new(1.,0.),Point2::new(0.,1.)]; let c=hard_cap_sweep_counts(2,3).unwrap();
        let mut frames=vec![frame();c.frames]; let mut v=vec![p(0.,0.,0.);c.vertices]; let mut t=vec![[0;3];c.triangles]; let mut n=v.clone(); let mut map=vec![0;c.vertices];
        assert!(hard_cap_sweep_normals_into(&axis[..2],&degenerate,None,&mut frames,&mut v,&mut t,&mut n,&mut map).is_err());
    }
}
