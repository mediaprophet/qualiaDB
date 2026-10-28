//! Explicit closed sampled-field extraction using a conforming tetrahedral grid.
//!
//! Unlike `marching_cubes`, this operation returns shared indexed vertices.
//! Each cube uses the same six-tetrahedron Freudenthal decomposition, so
//! adjacent cube faces have the same diagonal. Each zero-crossing vertex is
//! keyed by its exact pair of grid sample indices, not a spatial weld tolerance.
//! The surface is that of the piecewise-linear tetrahedral interpolant, not a
//! trilinear marching-cubes surface and not a botanical source representation.
//!
//! `grid` values must be finite, never equal to `iso`, and the complete grid
//! boundary must be strictly above `iso`. Positive finite spacing is required.
//! These restrictions deliberately refuse zero-valued sample vertices and
//! boundary-touching surfaces; there is no arbitrary epsilon-based snapping.
//! Preflight failures occur before output buffers are touched. Once extraction
//! starts, a geometry/overflow error may leave output scratch modified; publish
//! only a successful receipt. All scratch/output storage belongs to the caller.
//! Each output vertex retains its sorted grid-edge sample indices; these are
//! local sample provenance, not persistent author identifiers. This is not a guarantee of a single component, self-intersection-free smooth
//! geometry, genus, biological quality, UV/source mapping or a fork workflow.

use super::primitives::Point3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TetraSurfaceError {
    InvalidDimensions,
    SizeOverflow,
    InvalidGeometry,
    GridTooSmall,
    NonFiniteSample,
    SampleOnSurface,
    SurfaceTouchesBoundary,
    InsufficientBuffer,
    EdgeTableFull,
    DegenerateTriangle,
    EmptySurface,
    WorkLimitExceeded,
    OpenOrNonManifoldEdge,
}

/// Scratch slot for mapping a sorted pair of sample indices to one output vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TetraEdgeSlot { pub a: usize, pub b: usize, pub vertex: u32 }
impl Default for TetraEdgeSlot {
    fn default() -> Self { Self { a: usize::MAX, b: usize::MAX, vertex: 0 } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TetraSurfaceBounds {
    pub samples: usize,
    pub cells: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub edge_slots: usize,
}

/// Conservative worst-case sizes; `max_cells` is the host's explicit work cap.
/// Each cell has six tetrahedra; each tetrahedron has at most six crossing
/// edges and two triangles. Table load never exceeds one half.
pub fn tetra_surface_bounds(nx: usize, ny: usize, nz: usize, max_cells: usize) -> Result<TetraSurfaceBounds, TetraSurfaceError> {
    if nx < 2 || ny < 2 || nz < 2 { return Err(TetraSurfaceError::InvalidDimensions); }
    let samples = nx.checked_mul(ny).and_then(|x| x.checked_mul(nz)).ok_or(TetraSurfaceError::SizeOverflow)?;
    if samples == usize::MAX { return Err(TetraSurfaceError::SizeOverflow); }
    let cells = (nx-1).checked_mul(ny-1).and_then(|x| x.checked_mul(nz-1)).ok_or(TetraSurfaceError::SizeOverflow)?;
    if cells > max_cells { return Err(TetraSurfaceError::InvalidDimensions); }
    let vertices = cells.checked_mul(36).ok_or(TetraSurfaceError::SizeOverflow)?;
    let triangles = cells.checked_mul(12).ok_or(TetraSurfaceError::SizeOverflow)?;
    if vertices > u32::MAX as usize { return Err(TetraSurfaceError::SizeOverflow); }
    let edge_slots = vertices.checked_mul(2).and_then(|n| n.checked_next_power_of_two()).ok_or(TetraSurfaceError::SizeOverflow)?;
    Ok(TetraSurfaceBounds { samples, cells, vertices, triangles, edge_slots })
}

fn point(x: usize, y: usize, z: usize, origin: Point3, spacing: Point3) -> Point3 {
    Point3::new(origin.x + x as f64 * spacing.x, origin.y + y as f64 * spacing.y, origin.z + z as f64 * spacing.z)
}
fn sub(a: Point3, b: Point3) -> Point3 { Point3::new(a.x-b.x,a.y-b.y,a.z-b.z) }
fn cross(a: Point3, b: Point3) -> Point3 { Point3::new(a.y*b.z-a.z*b.y,a.z*b.x-a.x*b.z,a.x*b.y-a.y*b.x) }
fn dot(a: Point3,b: Point3)->f64 { a.x*b.x+a.y*b.y+a.z*b.z }
fn finite(p: Point3) -> bool { p.x.is_finite() && p.y.is_finite() && p.z.is_finite() }

fn edge_vertex(a: usize, b: usize, pa: Point3, pb: Point3, va: f64, vb: f64,
    slots: &mut [TetraEdgeSlot], vertices: &mut [Point3], edge_sources: &mut [[usize;2]], count: &mut usize, remaining_probes: &mut usize) -> Result<u32,TetraSurfaceError> {
    let (lo,hi)=if a<b {(a,b)} else {(b,a)};
    // Integer hash only selects a bucket; exact sample-pair equality determines identity.
    let hash=(lo as u64).wrapping_mul(0x9e3779b185ebca87) ^ (hi as u64).wrapping_mul(0xc2b2ae3d27d4eb4f);
    let mask=slots.len()-1; let mut bucket=(hash as usize)&mask;
    for _ in 0..slots.len() {
        if *remaining_probes == 0 { return Err(TetraSurfaceError::WorkLimitExceeded); }
        *remaining_probes -= 1;
        let slot=slots[bucket];
        if slot.a==lo && slot.b==hi { return Ok(slot.vertex); }
        if slot.a==usize::MAX {
            let aa=va.abs(); let bb=vb.abs(); let scale=aa.max(bb);
            let t=(aa/scale)/((aa/scale)+(bb/scale));
            if !t.is_finite() || t<=0.0 || t>=1.0 { return Err(TetraSurfaceError::InvalidGeometry); }
            let v=Point3::new(pa.x+(pb.x-pa.x)*t,pa.y+(pb.y-pa.y)*t,pa.z+(pb.z-pa.z)*t);
            if !finite(v) || v==pa || v==pb || *count>=vertices.len() { return Err(TetraSurfaceError::InvalidGeometry); }
            let index=u32::try_from(*count).map_err(|_|TetraSurfaceError::SizeOverflow)?;
            vertices[*count]=v; edge_sources[*count]=[lo,hi]; *count+=1; slots[bucket]=TetraEdgeSlot{a:lo,b:hi,vertex:index};
            return Ok(index);
        }
        bucket=(bucket+1)&mask;
    }
    Err(TetraSurfaceError::EdgeTableFull)
}

fn emit(mut tri:[u32;3], outward:Point3, vertices:&[Point3], triangles:&mut [[u32;3]], count:&mut usize, min_area_sq:f64) -> Result<(),TetraSurfaceError> {
    let a=vertices[tri[0] as usize]; let b=vertices[tri[1] as usize]; let c=vertices[tri[2] as usize];
    let normal=cross(sub(b,a),sub(c,a)); let area_sq=dot(normal,normal); let sign=dot(normal,outward);
    if !area_sq.is_finite() || area_sq <= min_area_sq || !sign.is_finite() || sign==0.0 { return Err(TetraSurfaceError::DegenerateTriangle); }
    if sign<0.0 { tri.swap(1,2); }
    if *count>=triangles.len() { return Err(TetraSurfaceError::InsufficientBuffer); }
    triangles[*count]=tri; *count+=1; Ok(())
}

// Reuse the sample-edge table as a triangle-edge validator after extraction.
// A value of 1/2 records the first directed use; 3 means its unique reverse
// was found. This checks edge incidence and winding, not vertex-link manifoldness.
fn validate_closed_oriented_edges(triangles:&[[u32;3]], slots:&mut [TetraEdgeSlot], remaining_probes:&mut usize) -> Result<(),TetraSurfaceError> {
    slots.fill(TetraEdgeSlot::default());
    let mask=slots.len()-1;
    for &[a,b,c] in triangles {
        if a==b || b==c || c==a { return Err(TetraSurfaceError::DegenerateTriangle); }
        for (u,v) in [(a,b),(b,c),(c,a)] {
            let lo=u.min(v) as usize; let hi=u.max(v) as usize;
            let direction=if u<v {1} else {2};
            let hash=(lo as u64).wrapping_mul(0x9e3779b185ebca87) ^ (hi as u64).wrapping_mul(0xc2b2ae3d27d4eb4f);
            let mut bucket=(hash as usize)&mask; let mut found=false;
            for _ in 0..slots.len() {
                if *remaining_probes == 0 { return Err(TetraSurfaceError::WorkLimitExceeded); }
                *remaining_probes -= 1;
                let slot=&mut slots[bucket];
                if slot.a==usize::MAX { *slot=TetraEdgeSlot{a:lo,b:hi,vertex:direction}; found=true;break; }
                if slot.a==lo && slot.b==hi {
                    if slot.vertex==3 || slot.vertex==direction { return Err(TetraSurfaceError::OpenOrNonManifoldEdge); }
                    slot.vertex=3;found=true;break;
                }
                bucket=(bucket+1)&mask;
            }
            if !found { return Err(TetraSurfaceError::EdgeTableFull); }
        }
    }
    if slots.iter().any(|slot|slot.a!=usize::MAX && slot.vertex!=3) { return Err(TetraSurfaceError::OpenOrNonManifoldEdge); }
    Ok(())
}

/// Extract the closed piecewise-linear level set into shared-index triangles.
/// Caller must treat all buffers as scratch until this function returns `Ok`.
/// Numeric budgets, maximum cells and aggregate hash probes, and an explicit minimum squared doubled-face
/// area are selected by the caller. `slots` is cleared on successful preflight.
pub fn indexed_tetra_surface_into(
    grid:&[f64], dims:[usize;3], origin:Point3, spacing:Point3, iso:f64,
    max_cells:usize, max_probes:usize, min_area_sq:f64, vertices:&mut [Point3], triangles:&mut [[u32;3]], edge_sources:&mut [[usize;2]], slots:&mut [TetraEdgeSlot],
) -> Result<(usize,usize),TetraSurfaceError> {
    let [nx,ny,nz]=dims; let bounds=tetra_surface_bounds(nx,ny,nz,max_cells)?;
    if max_probes==0 { return Err(TetraSurfaceError::WorkLimitExceeded); }
    let mut remaining_probes=max_probes;
    if grid.len()<bounds.samples { return Err(TetraSurfaceError::GridTooSmall); }
    if !finite(origin) || !finite(spacing) || spacing.x<=0.0 || spacing.y<=0.0 || spacing.z<=0.0 || !iso.is_finite() || !min_area_sq.is_finite() || min_area_sq<0.0 || !finite(point(nx-1,ny-1,nz-1,origin,spacing)) { return Err(TetraSurfaceError::InvalidGeometry); }
    if vertices.len()<bounds.vertices || triangles.len()<bounds.triangles || edge_sources.len()<bounds.vertices || slots.len()<bounds.edge_slots { return Err(TetraSurfaceError::InsufficientBuffer); }
    for z in 0..nz { for y in 0..ny { for x in 0..nx {
        let v=grid[x+y*nx+z*nx*ny]; if !v.is_finite(){ return Err(TetraSurfaceError::NonFiniteSample); }
        if v==iso { return Err(TetraSurfaceError::SampleOnSurface); }
        if (x==0 || y==0 || z==0 || x+1==nx || y+1==ny || z+1==nz) && v<iso { return Err(TetraSurfaceError::SurfaceTouchesBoundary); }
        if !(v-iso).is_finite() { return Err(TetraSurfaceError::InvalidGeometry); }
    } } }
    let slots=&mut slots[..bounds.edge_slots]; slots.fill(TetraEdgeSlot::default());
    let mut vc=0; let mut tc=0;
    const CORNERS:[[usize;3];8]=[[0,0,0],[1,0,0],[1,1,0],[0,1,0],[0,0,1],[1,0,1],[1,1,1],[0,1,1]];
    const TETS:[[usize;4];6]=[[0,1,2,6],[0,2,3,6],[0,3,7,6],[0,7,4,6],[0,4,5,6],[0,5,1,6]];
    for z in 0..nz-1 { for y in 0..ny-1 { for x in 0..nx-1 {
        let mut id=[0usize;8]; let mut pos=[Point3::default();8]; let mut v=[0.0;8];
        for k in 0..8 { let c=CORNERS[k]; let (cx,cy,cz)=(x+c[0],y+c[1],z+c[2]); id[k]=cx+cy*nx+cz*nx*ny; pos[k]=point(cx,cy,cz,origin,spacing); v[k]=grid[id[k]]-iso; }
        for tet in TETS {
            let mut inside=[0usize;4]; let mut outside=[0usize;4]; let mut ni=0; let mut no=0;
            for k in tet { if v[k]<0.0 {inside[ni]=k;ni+=1;} else {outside[no]=k;no+=1;} }
            if ni==0 || no==0 {continue;}
            let mut iavg=Point3::default(); for &k in &inside[..ni] {iavg.x+=pos[k].x/(ni as f64);iavg.y+=pos[k].y/(ni as f64);iavg.z+=pos[k].z/(ni as f64);}
            let mut oavg=Point3::default(); for &k in &outside[..no] {oavg.x+=pos[k].x/(no as f64);oavg.y+=pos[k].y/(no as f64);oavg.z+=pos[k].z/(no as f64);}
            let outward=sub(oavg,iavg);
            let mut edge=|a:usize,b:usize| edge_vertex(id[a],id[b],pos[a],pos[b],v[a],v[b],slots,vertices,edge_sources,&mut vc,&mut remaining_probes);
            if ni==1 || no==1 {
                let (center,others)=if ni==1 {(inside[0],&outside[..no])} else {(outside[0],&inside[..ni])};
                let tri=[edge(center,others[0])?,edge(center,others[1])?,edge(center,others[2])?];
                emit(tri,outward,vertices,triangles,&mut tc,min_area_sq)?;
            } else {
                let (a,b,c,d)=(inside[0],inside[1],outside[0],outside[1]);
                let e0=edge(a,c)?; let e1=edge(a,d)?; let e2=edge(b,c)?; let e3=edge(b,d)?;
                emit([e0,e1,e2],outward,vertices,triangles,&mut tc,min_area_sq)?;
                emit([e1,e3,e2],outward,vertices,triangles,&mut tc,min_area_sq)?;
            }
        }
    } } }
    if tc==0 { return Err(TetraSurfaceError::EmptySurface); }
    validate_closed_oriented_edges(&triangles[..tc],slots,&mut remaining_probes)?;
    Ok((vc,tc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::sdf_ops::{sdf_capsule, sdf_smooth_union};
    use std::collections::{HashMap, VecDeque};

    fn output(grid: &[f64], dims: [usize;3], origin: Point3, spacing: Point3)
      -> Result<(Vec<Point3>,Vec<[u32;3]>),TetraSurfaceError> {
        let b=tetra_surface_bounds(dims[0],dims[1],dims[2],100_000)?;
        let mut vertices=vec![Point3::default();b.vertices];
        let mut triangles=vec![[0;3];b.triangles];
        let mut slots=vec![TetraEdgeSlot::default();b.edge_slots];
        let mut edge_sources=vec![[0usize;2];b.vertices];
        let (v,t)=indexed_tetra_surface_into(grid,dims,origin,spacing,0.0,100_000,10_000_000,1e-28,&mut vertices,&mut triangles,&mut edge_sources,&mut slots)?;
        vertices.truncate(v);triangles.truncate(t); Ok((vertices,triangles))
    }
    fn check_closed_connected(vertices:&[Point3], triangles:&[[u32;3]]) {
        assert!(!vertices.is_empty() && !triangles.is_empty());
        let mut edges:HashMap<(u32,u32),(usize,i32)>=HashMap::new();
        let mut adj=vec![Vec::new();vertices.len()];
        for &[a,b,c] in triangles {
            assert!(a!=b && b!=c && c!=a);
            for (u,v) in [(a,b),(b,c),(c,a)] {
                let key=(u.min(v),u.max(v)); let e=edges.entry(key).or_insert((0,0));
                e.0+=1; e.1+=if u<v {1} else {-1};
                adj[u as usize].push(v as usize);adj[v as usize].push(u as usize);
            }
        }
        assert!(edges.values().all(|&(count,orientation)| count==2 && orientation==0),"closed oriented edges required");
        let mut visited=vec![false;vertices.len()]; let mut queue=VecDeque::from([0usize]); visited[0]=true;
        while let Some(i)=queue.pop_front(){for &j in &adj[i]{if !visited[j]{visited[j]=true;queue.push_back(j);}}}
        assert!(visited.into_iter().all(|v|v),"one connected indexed surface required for fixture");
    }
    #[test]
    fn shared_edge_sphere_is_closed_connected_and_deterministic() {
        let dims=[13,13,13]; let origin=Point3::new(-3.07,-3.09,-3.03); let step=Point3::new(0.5,0.5,0.5);
        let mut grid=Vec::new();
        for z in 0..dims[2] { for y in 0..dims[1] { for x in 0..dims[0] {
            let p=point(x,y,z,origin,step);grid.push((p.x*p.x+p.y*p.y+p.z*p.z).sqrt()-1.1);
        } } }
        let a=output(&grid,dims,origin,step).unwrap(); let b=output(&grid,dims,origin,step).unwrap();
        assert_eq!(a,b);check_closed_connected(&a.0,&a.1);
    }
    #[test]
    fn blended_three_arm_constant_radius_field_is_indexed_and_closed_for_fixture() {
        let dims=[15,15,15]; let origin=Point3::new(-3.57,-3.59,-3.53);let step=Point3::new(0.5,0.5,0.5);
        let center=Point3::new(0.0,0.0,0.0); let arms=[Point3::new(-1.4,0.3,1.2),Point3::new(1.3,0.1,1.3),Point3::new(0.0,-0.2,-1.7)];
        let mut grid=Vec::new();
        for z in 0..dims[2] { for y in 0..dims[1] { for x in 0..dims[0] {
            let p=point(x,y,z,origin,step);let a=sdf_capsule(p,center,arms[0],0.52);let b=sdf_capsule(p,center,arms[1],0.43);let c=sdf_capsule(p,center,arms[2],0.62);
            grid.push(sdf_smooth_union(sdf_smooth_union(a,b,0.2),c,0.2));
        } } }
        let (vertices,triangles)=output(&grid,dims,origin,step).unwrap();check_closed_connected(&vertices,&triangles);
    }
    #[test]
    fn edge_validator_rejects_open_and_duplicate_directed_faces() {
        let mut slots=vec![TetraEdgeSlot::default();64];
        assert_eq!(validate_closed_oriented_edges(&[[0,1,2]],&mut slots,&mut 1000),Err(TetraSurfaceError::OpenOrNonManifoldEdge));
        assert_eq!(validate_closed_oriented_edges(&[[0,1,2],[0,1,2]],&mut slots,&mut 1000),Err(TetraSurfaceError::OpenOrNonManifoldEdge));
    }
    #[test]
    fn interpolation_refuses_rounded_endpoints_and_probe_limit() {
        let mut slots=vec![TetraEdgeSlot::default();8];let sentinel=Point3::new(9.0,9.0,9.0);let mut v=[sentinel;2];let mut sources=[[7,8];2];let mut count=0;
        for (a,b) in [(-f64::from_bits(1),f64::MAX),(-f64::MAX,f64::from_bits(1))] {
            let mut remaining=8;
            assert_eq!(edge_vertex(0,1,Point3::default(),Point3::new(1.0,0.0,0.0),a,b,&mut slots,&mut v,&mut sources,&mut count,&mut remaining),Err(TetraSurfaceError::InvalidGeometry));
            assert_eq!(count,0);assert_eq!(v,[sentinel;2]);assert_eq!(sources,[[7,8];2]);
        }
        for (pa,pb,va,vb) in [(Point3::new(1.0,0.0,0.0),Point3::new(2.0,0.0,0.0),-1e-30,1.0),(Point3::new(2.0,0.0,0.0),Point3::new(1.0,0.0,0.0),1.0,-1e-30)] {
            let mut remaining=8;assert_eq!(edge_vertex(0,1,pa,pb,va,vb,&mut slots,&mut v,&mut sources,&mut count,&mut remaining),Err(TetraSurfaceError::InvalidGeometry));assert_eq!(count,0);
        }
        let mut remaining=0;
        assert_eq!(edge_vertex(0,1,Point3::default(),Point3::new(1.0,0.0,0.0),-1.0,1.0,&mut slots,&mut v,&mut sources,&mut count,&mut remaining),Err(TetraSurfaceError::WorkLimitExceeded));
        assert_eq!(count,0);
        let mut remaining=0;assert_eq!(validate_closed_oriented_edges(&[[0,1,2]],&mut slots,&mut remaining),Err(TetraSurfaceError::WorkLimitExceeded));
    }
    #[test]
    fn checked_dimensions_and_short_buffers_refuse_without_writes() {
        assert_eq!(tetra_surface_bounds(usize::MAX,2,2,usize::MAX),Err(TetraSurfaceError::SizeOverflow));
        assert!(tetra_surface_bounds(3,3,3,7).is_err());
        let bounds=tetra_surface_bounds(3,3,3,8).unwrap(); let mut grid=vec![1.0;bounds.samples];grid[13]=-1.0;
        let sentinel=Point3::new(7.0,8.0,9.0);let mut v=vec![sentinel;bounds.vertices-1];let mut t=vec![[1,2,3];bounds.triangles];let mut e=vec![[4usize,5];bounds.vertices];let mut slots=vec![TetraEdgeSlot{a:1,b:2,vertex:3};bounds.edge_slots];
        assert_eq!(indexed_tetra_surface_into(&grid,[3,3,3],Point3::default(),Point3::new(1.0,1.0,1.0),0.0,8,10_000,0.0,&mut v,&mut t,&mut e,&mut slots),Err(TetraSurfaceError::InsufficientBuffer));
        assert!(v.iter().all(|x|*x==sentinel));assert!(t.iter().all(|x|*x==[1,2,3]));assert!(e.iter().all(|x|*x==[4,5]));assert!(slots.iter().all(|x|x.a==1 && x.b==2 && x.vertex==3));
    }
    #[test]
    fn exact_zero_and_inside_boundary_refuse_before_touching_outputs() {
        let dims=[3,3,3]; let b=tetra_surface_bounds(3,3,3,8).unwrap(); let mut grid=vec![1.0;b.samples];
        let sentinel=Point3::new(7.0,8.0,9.0);let mut vertices=vec![sentinel;b.vertices];let mut tris=vec![[7,8,9];b.triangles];let mut slots=vec![TetraEdgeSlot{a:1,b:2,vertex:3};b.edge_slots]; let mut edge_sources=vec![[4usize,5];b.vertices];
        grid[13]=0.0;
        assert_eq!(indexed_tetra_surface_into(&grid,dims,Point3::default(),Point3::new(1.0,1.0,1.0),0.0,8,10_000,0.0,&mut vertices,&mut tris,&mut edge_sources,&mut slots),Err(TetraSurfaceError::SampleOnSurface));
        assert!(vertices.iter().all(|v|*v==sentinel));assert!(tris.iter().all(|t|*t==[7,8,9]));assert!(slots.iter().all(|s|s.a==1 && s.b==2 && s.vertex==3)); assert!(edge_sources.iter().all(|v|*v==[4,5]));
        grid[13]=-1.0;grid[0]=-1.0;
        assert_eq!(indexed_tetra_surface_into(&grid,dims,Point3::default(),Point3::new(1.0,1.0,1.0),0.0,8,10_000,0.0,&mut vertices,&mut tris,&mut edge_sources,&mut slots),Err(TetraSurfaceError::SurfaceTouchesBoundary));
        assert!(vertices.iter().all(|v|*v==sentinel));
    }
}
