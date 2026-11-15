//! Caller-buffered orchestration. Every buffer is unpublished scratch until Ok.
use super::super::{Point3, HalfEdge, EdgeSlot, ROOT_PARENT, distance::Aabb,
    indexed_tetra_surface::{self as tetra, TetraEdgeSlot},
    surface_topology_qualification as topology,
    tapered_branch_field::{self as field, TaperedFieldSegment}, ddg_operators};

pub struct BranchGraphInput<'a> {
    /// Local source indices, not biological or globally authoritative identifiers.
    pub positions: &'a [Point3], pub parents: &'a [u32], pub radii: &'a [f64],
}
#[derive(Clone, Copy, Debug)]
pub struct BranchGraphSpec {
    pub origin: Point3, pub spacing: f64, pub dimensions: [usize;3],
    pub minimum_radius_samples: f64, pub min_axis: f64,
    pub projection_tolerance: f64, pub min_area_sq: f64,
    pub max_incident_cosine: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct BranchGraphLimits {
    pub nodes: usize, pub samples: usize, pub cells: usize,
    pub vertices: usize, pub triangles: usize, pub bytes: usize,
    pub work: usize, pub extraction_probes: usize, pub topology_work: usize,
}
impl Default for BranchGraphLimits {
    fn default() -> Self { Self {nodes:64,samples:512_000,cells:512_000,
        vertices:65_536,triangles:131_072,bytes:256*1024*1024,
        work:250_000_000,extraction_probes:20_000_000,topology_work:20_000_000} }
}
#[derive(Clone, Copy, Debug)]
pub struct BranchGraphBounds {
    pub samples:usize, pub cells:usize, pub vertices:usize, pub triangles:usize,
    pub extraction_slots:usize, pub topology:topology::SurfaceQualificationStorage,
    pub segments:usize, pub bytes:usize, pub reserved_work:usize,
}
pub struct BranchGraphBuffers<'a> {
    pub samples:&'a mut [Point3], pub values:&'a mut [f64],
    pub segments:&'a mut [TaperedFieldSegment], pub boxes:&'a mut [Aabb],
    pub vertices:&'a mut [Point3], pub triangles:&'a mut [[u32;3]],
    pub normals:&'a mut [Point3], pub face_normals:&'a mut [Point3],
    pub sample_edges:&'a mut [[usize;2]], pub extraction_slots:&'a mut [TetraEdgeSlot],
    pub half_edges:&'a mut [HalfEdge], pub topology_slots:&'a mut [EdgeSlot],
    pub vertex_starts:&'a mut [u32], pub visited:&'a mut [bool],
    pub face_labels:&'a mut [u32], pub face_queue:&'a mut [u32],
}
#[derive(Clone, Copy, Debug)]
pub struct BranchGraphReceipt {
    pub vertices:usize, pub triangles:usize, pub samples:usize,
    pub smallest_radius_over_spacing:f64,
    pub topology:topology::SurfaceQualificationReceipt,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BranchGraphError {
    InvalidInput, Overflow, Budget, ShortBuffer, Cancelled,
    Undersampled {node:usize}, NonincidentClearanceUnproven {a:usize,b:usize},
    IncidentAxesTooClose {a:usize,b:usize}, Field(field::TaperedFieldError),
    Extraction(tetra::TetraSurfaceError), Topology(topology::BudgetedSurfaceQualificationError), Normals,
}
fn add(a:usize,b:usize)->Result<usize,BranchGraphError>{a.checked_add(b).ok_or(BranchGraphError::Overflow)}
fn mul(a:usize,b:usize)->Result<usize,BranchGraphError>{a.checked_mul(b).ok_or(BranchGraphError::Overflow)}

/// Count preflight before the caller allocates. Budgets are host-owned, never
/// selected by the authored recipe. This includes every used buffer prefix.
pub fn branch_graph_surface_bounds(nodes:usize,spec:BranchGraphSpec,limits:BranchGraphLimits)
    ->Result<BranchGraphBounds,BranchGraphError> {
    if nodes<2 || nodes>limits.nodes || limits.vertices<3 || limits.triangles<4
        || limits.extraction_probes==0 || limits.topology_work==0 {return Err(BranchGraphError::Budget);}
    if !finite(spec.origin)||!spec.spacing.is_finite()||spec.spacing<=0.0||spec.spacing>1e6
        ||!spec.minimum_radius_samples.is_finite()||!(2.0..=32.0).contains(&spec.minimum_radius_samples)
        ||!spec.min_axis.is_finite()||spec.min_axis<=0.0||!spec.projection_tolerance.is_finite()||spec.projection_tolerance<=0.0
        ||!spec.min_area_sq.is_finite()||spec.min_area_sq<0.0||!spec.max_incident_cosine.is_finite()
        ||!(0.0..1.0).contains(&spec.max_incident_cosine){return Err(BranchGraphError::InvalidInput);}
    let b=tetra::tetra_surface_bounds(spec.dimensions[0],spec.dimensions[1],spec.dimensions[2],limits.cells)
        .map_err(BranchGraphError::Extraction)?;
    if b.samples>limits.samples {return Err(BranchGraphError::Budget);}
    for axis in 0..3 {
        let start=[spec.origin.x,spec.origin.y,spec.origin.z][axis];let mut previous=start;
        for i in 1..spec.dimensions[axis]{let next=start+i as f64*spec.spacing;
            if !next.is_finite()||next.abs()>1e6||next<=previous{return Err(BranchGraphError::InvalidInput);}previous=next;}
    }
    let v=b.vertices.min(limits.vertices); let t=b.triangles.min(limits.triangles);
    let slots=mul(v,2)?.max(mul(t,3)?).checked_next_power_of_two().ok_or(BranchGraphError::Overflow)?;
    let q=topology::surface_qualification_storage_bounds(u32::try_from(v).map_err(|_|BranchGraphError::Overflow)?,t)
        .map_err(|_|BranchGraphError::Overflow)?;
    let mut bytes=0;
    use std::mem::size_of;
    for (count,size) in [(b.samples,size_of::<Point3>()+size_of::<f64>()),
        (nodes-1,size_of::<TaperedFieldSegment>()+size_of::<Aabb>()),
        (v,2*size_of::<Point3>()+size_of::<[usize;2]>()),
        (t,size_of::<[u32;3]>()+size_of::<Point3>()), (slots,size_of::<TetraEdgeSlot>()),
        (q.half_edges,size_of::<HalfEdge>()),(q.edge_slots,size_of::<EdgeSlot>()),
        (q.vertex_starts,size_of::<u32>()),(q.visited_half_edges,size_of::<bool>()),
        (q.face_labels,size_of::<u32>()),(q.face_queue,size_of::<u32>())] {bytes=add(bytes,mul(count,size)?)?;}
    let work=add(add(mul(b.samples,nodes-1)?,mul(nodes,mul(nodes,64)?)?)?,
        add(add(mul(b.cells,100)?,mul(add(v,t)?,128)?)?,add(limits.extraction_probes,limits.topology_work)?)?)?;
    let work=add(work,bytes)?; // includes complete scratch initialization/clears
    if bytes>limits.bytes || work>limits.work {return Err(BranchGraphError::Budget);}
    Ok(BranchGraphBounds{samples:b.samples,cells:b.cells,vertices:v,triangles:t,extraction_slots:slots,
        topology:q,segments:nodes-1,bytes,reserved_work:work})
}
fn finite(p:Point3)->bool {[p.x,p.y,p.z].iter().all(|v|v.is_finite()&&v.abs()<=1e6)}
fn difference(a:Point3,b:Point3)->Point3{Point3::new(a.x-b.x,a.y-b.y,a.z-b.z)}
fn unit(a:Point3)->Point3{let n=a.x.hypot(a.y).hypot(a.z);Point3::new(a.x/n,a.y/n,a.z/n)}

/// Uses hard minimum of authored tapered segments. This can have C0 collar
/// creases; smooth normals do not make the underlying surface C1. A successful
/// result is a closed selected study, not a boundary-compatible master graft.
pub fn branch_graph_surface_into(input:BranchGraphInput<'_>,spec:BranchGraphSpec,limits:BranchGraphLimits,
    out:BranchGraphBuffers<'_>,cancelled:&dyn Fn()->bool)->Result<BranchGraphReceipt,BranchGraphError> {
    if cancelled(){return Err(BranchGraphError::Cancelled);}
    let n=input.positions.len();let b=branch_graph_surface_bounds(n,spec,limits)?;
    if input.parents.len()!=n||input.radii.len()!=n||input.parents[0]!=ROOT_PARENT
        ||!finite(spec.origin)||!spec.spacing.is_finite()||spec.spacing<=0.0
        ||!spec.minimum_radius_samples.is_finite()||!(2.0..=32.0).contains(&spec.minimum_radius_samples)
        ||!spec.min_axis.is_finite()||spec.min_axis<=0.0||!spec.projection_tolerance.is_finite()||spec.projection_tolerance<=0.0
        ||!spec.min_area_sq.is_finite()||spec.min_area_sq<0.0||!spec.max_incident_cosine.is_finite()
        ||!(0.0..1.0).contains(&spec.max_incident_cosine) {return Err(BranchGraphError::InvalidInput);}
    for (i,&p) in input.positions.iter().enumerate(){
        if !finite(p)||!input.radii[i].is_finite()||input.radii[i]<=0.0||input.radii[i]>1e6
            ||(i>0&&input.parents[i] as usize>=i){return Err(BranchGraphError::InvalidInput);}
        if input.radii[i]/spec.spacing<spec.minimum_radius_samples {return Err(BranchGraphError::Undersampled{node:i});}
    }
    let q=b.topology;
    if out.samples.len()<b.samples||out.values.len()<b.samples||out.segments.len()<b.segments||out.boxes.len()<b.segments
        ||out.vertices.len()<b.vertices||out.triangles.len()<b.triangles||out.normals.len()<b.vertices||out.face_normals.len()<b.triangles
        ||out.sample_edges.len()<b.vertices||out.extraction_slots.len()<b.extraction_slots||out.half_edges.len()<q.half_edges
        ||out.topology_slots.len()<q.edge_slots||out.vertex_starts.len()<q.vertex_starts||out.visited.len()<q.visited_half_edges
        ||out.face_labels.len()<q.face_labels||out.face_queue.len()<q.face_queue{return Err(BranchGraphError::ShortBuffer);}
    for axis in 0..3 {
        let origin=[spec.origin.x,spec.origin.y,spec.origin.z][axis];let mut prior=origin;
        for i in 1..spec.dimensions[axis]{let next=origin+i as f64*spec.spacing;
            if !next.is_finite()||next.abs()>1e6||next<=prior{return Err(BranchGraphError::InvalidInput);}prior=next;}
    }
    for i in 1..n {
        let parent=input.parents[i] as usize;let a=input.positions[parent];let z=input.positions[i];
        let d=difference(a,z);if d.x.hypot(d.y).hypot(d.z)<spec.min_axis{return Err(BranchGraphError::InvalidInput);}
        out.segments[i-1]=TaperedFieldSegment{a,b:z,radius_a:input.radii[parent],radius_b:input.radii[i]};
        // Capsule maximum-radius envelope plus a sample-cell diagonal margin.
        let radius=input.radii[parent].max(input.radii[i])+3.0f64.sqrt()*spec.spacing;
        out.boxes[i-1]=Aabb::new(Point3::new(a.x.min(z.x)-radius,a.y.min(z.y)-radius,a.z.min(z.z)-radius),
            Point3::new(a.x.max(z.x)+radius,a.y.max(z.y)+radius,a.z.max(z.z)+radius));
    }
    for i in 1..n {for j in i+1..n {
        let a=input.parents[i] as usize;let c=input.parents[j] as usize;
        let shared=if a==c{Some((a,i,j))}else if i==c{Some((i,a,j))}else if j==a{Some((j,i,c))}else{None};
        if let Some((center,x,y))=shared {
            let dx=unit(difference(input.positions[x],input.positions[center]));let dy=unit(difference(input.positions[y],input.positions[center]));
            if dx.x*dy.x+dx.y*dy.y+dx.z*dy.z>=spec.max_incident_cosine{return Err(BranchGraphError::IncidentAxesTooClose{a:i,b:j});}
        }else if out.boxes[i-1].overlaps(&out.boxes[j-1]){return Err(BranchGraphError::NonincidentClearanceUnproven{a:i,b:j});}
    }}
    let [nx,ny,nz]=spec.dimensions;
    for z in 0..nz {if cancelled(){return Err(BranchGraphError::Cancelled);}for y in 0..ny {for x in 0..nx {
        out.samples[x+nx*y+nx*ny*z]=Point3::new(spec.origin.x+x as f64*spec.spacing,spec.origin.y+y as f64*spec.spacing,spec.origin.z+z as f64*spec.spacing);
    }}}
    field::tapered_branch_field_into(&out.samples[..b.samples],&out.segments[..b.segments],0.0,spec.min_axis,
        spec.projection_tolerance,limits.samples,limits.nodes-1,limits.work,&mut out.values[..b.samples]).map_err(BranchGraphError::Field)?;
    if cancelled(){return Err(BranchGraphError::Cancelled);}
    let (vc,tc)=tetra::indexed_tetra_surface_capped_into(&out.values[..b.samples],spec.dimensions,spec.origin,
        Point3::new(spec.spacing,spec.spacing,spec.spacing),0.0,limits.cells,limits.extraction_probes,spec.min_area_sq,
        &mut out.vertices[..b.vertices],&mut out.triangles[..b.triangles],&mut out.sample_edges[..b.vertices],
        &mut out.extraction_slots[..b.extraction_slots]).map_err(BranchGraphError::Extraction)?;
    let mut work=limits.topology_work;
    let receipt=topology::qualify_surface_topology_budgeted_into(vc as u32,&out.triangles[..tc],&mut work,
        out.half_edges,out.topology_slots,out.vertex_starts,out.visited,out.face_labels,out.face_queue).map_err(BranchGraphError::Topology)?;
    ddg_operators::face_vector_areas(&out.vertices[..vc],&out.triangles[..tc],&mut out.face_normals[..tc]).map_err(|_|BranchGraphError::Normals)?;
    ddg_operators::vertex_normals(&out.vertices[..vc],&out.triangles[..tc],&mut out.normals[..vc]).map_err(|_|BranchGraphError::Normals)?;
    if out.face_normals[..tc].iter().chain(&out.normals[..vc]).any(|n|!finite(*n)||!(n.x*n.x+n.y*n.y+n.z*n.z>0.0)) {return Err(BranchGraphError::Normals);}
    if cancelled(){return Err(BranchGraphError::Cancelled);}
    Ok(BranchGraphReceipt{vertices:vc,triangles:tc,samples:b.samples,
        smallest_radius_over_spacing:input.radii.iter().copied().fold(f64::INFINITY,f64::min)/spec.spacing,topology:receipt})
}
