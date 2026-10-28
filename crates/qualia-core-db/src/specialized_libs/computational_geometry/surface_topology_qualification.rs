//! Caller-buffered topology qualification for a closed indexed triangle surface.
//!
//! Reuses QualiaDB's half-edge builder and face-component owner. A successful
//! receipt establishes one face-connected component, oppositely oriented paired
//! edges, no boundary, no unused vertices, and one closed fan at each vertex.
//! It does not establish finite coordinates, geometric nondegeneracy, no
//! self-intersections, genus, UV seams, source lineage or biological quality.
//!
//! Work is bounded conservatively before invoking the existing hash and BFS
//! owners: worst-case slot probes plus checked linear triangle-ordered BFS. Caller chooses an explicit `max_work`. No heap allocation occurs.
//! Preflight failures leave scratch untouched; later topology errors may alter
//! scratch and must not be treated as a published successful receipt.

use super::connectivity::{label_components_triangle_ordered, triangle_ordered_component_work, ConnectivityError, OrderedConnectivityError};
use super::topology::{build_triangle_half_edges, build_triangle_half_edges_budgeted, BudgetedTopologyError, required_edge_slots, EdgeSlot, HalfEdge, TopologyError, INVALID_INDEX};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceQualificationError {
    EmptyMesh,
    SizeOverflow,
    InsufficientScratch,
    WorkLimitExceeded { required: usize },
    Topology(TopologyError),
    Connectivity(ConnectivityError),
    OrderedConnectivity(OrderedConnectivityError),
    BoundaryEdges(u32),
    Disconnected(u32),
    UnusedVertex(u32),
    InvalidVertexLink(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceQualificationReceipt {
    pub vertex_count: u32,
    pub face_count: u32,
    pub half_edge_count: u32,
    pub component_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceQualificationBounds {
    pub half_edges: usize,
    pub edge_slots: usize,
    pub vertex_starts: usize,
    pub visited_half_edges: usize,
    pub face_labels: usize,
    pub face_queue: usize,
    pub work: usize,
}

/// Conservative upper bound for the existing topology/face-connectivity
/// owners and the vertex-fan validation. `work` counts elementary visits/probes,
/// not measured wall-clock time or hardware resource consumption.
pub fn surface_qualification_bounds(vertex_count:u32,face_count:usize) -> Result<SurfaceQualificationBounds,SurfaceQualificationError> {
    if vertex_count==0 || face_count==0 { return Err(SurfaceQualificationError::EmptyMesh); }
    if face_count>(u32::MAX as usize)/3 { return Err(SurfaceQualificationError::SizeOverflow); }
    let half_edges=face_count.checked_mul(3).ok_or(SurfaceQualificationError::SizeOverflow)?;
    let slots=required_edge_slots(face_count);
    if slots==usize::MAX || !slots.is_power_of_two() { return Err(SurfaceQualificationError::SizeOverflow); }
    // Two scans of up to slot length per directed edge: existing find/insert.
    let hash_work=half_edges.checked_mul(2).and_then(|x|x.checked_mul(slots)).ok_or(SurfaceQualificationError::SizeOverflow)?;
    // Checked triangle-ordered connectivity traversal is linear in faces.
    let component_work=triangle_ordered_component_work(face_count as u32).map_err(|_|SurfaceQualificationError::SizeOverflow)?;
    let housekeeping=half_edges.checked_mul(32)
        .and_then(|x| x.checked_add((vertex_count as usize).checked_mul(8)?))
        .and_then(|x| x.checked_add(slots.checked_mul(4)?))
        .and_then(|x| x.checked_add(64)).ok_or(SurfaceQualificationError::SizeOverflow)?;
    let work=hash_work.checked_add(component_work).and_then(|x|x.checked_add(housekeeping)).ok_or(SurfaceQualificationError::SizeOverflow)?;
    Ok(SurfaceQualificationBounds { half_edges,edge_slots:slots,vertex_starts:vertex_count as usize,
        visited_half_edges:half_edges,face_labels:face_count,face_queue:face_count,work })
}

/// Qualify an already-produced indexed triangle surface without changing it.
/// Buffers must satisfy `surface_qualification_bounds`; only their required
/// prefixes are changed. Assumes no geometry or attribute interpretation.
pub fn qualify_surface_topology_into(
    vertex_count:u32,triangles:&[[u32;3]],max_work:usize,
    half_edges:&mut [HalfEdge],edge_slots:&mut [EdgeSlot],vertex_starts:&mut [u32],
    visited_half_edges:&mut [bool],face_labels:&mut [u32],face_queue:&mut [u32],
) -> Result<SurfaceQualificationReceipt,SurfaceQualificationError> {
    let bounds=surface_qualification_bounds(vertex_count,triangles.len())?;
    if max_work<bounds.work { return Err(SurfaceQualificationError::WorkLimitExceeded{required:bounds.work}); }
    if half_edges.len()<bounds.half_edges || edge_slots.len()<bounds.edge_slots ||
       vertex_starts.len()<bounds.vertex_starts || visited_half_edges.len()<bounds.visited_half_edges ||
       face_labels.len()<bounds.face_labels || face_queue.len()<bounds.face_queue {
        return Err(SurfaceQualificationError::InsufficientScratch);
    }
    let edges=&mut half_edges[..bounds.half_edges];
    let summary=build_triangle_half_edges(vertex_count,triangles,edges,&mut edge_slots[..bounds.edge_slots]).map_err(SurfaceQualificationError::Topology)?;
    if summary.boundary_half_edges!=0 { return Err(SurfaceQualificationError::BoundaryEdges(summary.boundary_half_edges)); }
    let traversal_work=triangle_ordered_component_work(triangles.len() as u32).map_err(|_|SurfaceQualificationError::SizeOverflow)?;
    let components=label_components_triangle_ordered(triangles.len() as u32,edges,traversal_work,&mut face_labels[..bounds.face_labels],&mut face_queue[..bounds.face_queue]).map_err(SurfaceQualificationError::OrderedConnectivity)?;
    if components!=1 { return Err(SurfaceQualificationError::Disconnected(components)); }
    let starts=&mut vertex_starts[..bounds.vertex_starts];starts.fill(INVALID_INDEX);
    let visited=&mut visited_half_edges[..bounds.visited_half_edges];visited.fill(false);
    for (i,edge) in edges.iter().enumerate() {
        let start=&mut starts[edge.origin as usize];
        if *start==INVALID_INDEX { *start=i as u32; }
    }
    for vertex in 0..vertex_count {
        let start=starts[vertex as usize];
        if start==INVALID_INDEX { return Err(SurfaceQualificationError::UnusedVertex(vertex)); }
        let mut cur=start; let mut steps=0usize;
        loop {
            let i=cur as usize;
            if i>=edges.len() || edges[i].origin!=vertex { return Err(SurfaceQualificationError::InvalidVertexLink(vertex)); }
            if visited[i] {
                if cur==start && steps>0 { break; }
                return Err(SurfaceQualificationError::InvalidVertexLink(vertex));
            }
            visited[i]=true;steps+=1;
            // For v→a, the preceding edge b→v is next(next(v→a)); its
            // twin v→b advances to the next triangle incident on v.
            let next=edges[i].next as usize;
            if next>=edges.len() { return Err(SurfaceQualificationError::InvalidVertexLink(vertex)); }
            let prev=edges[next].next as usize;
            if prev>=edges.len() || edges[prev].twin==INVALID_INDEX { return Err(SurfaceQualificationError::InvalidVertexLink(vertex)); }
            cur=edges[prev].twin;
            if steps>edges.len() { return Err(SurfaceQualificationError::InvalidVertexLink(vertex)); }
        }
    }
    // More than one fan at a vertex leaves additional outgoing half-edges
    // unvisited even if every edge is paired and all faces are connected.
    if let Some(i)=visited.iter().position(|x|!*x) { return Err(SurfaceQualificationError::InvalidVertexLink(edges[i].origin)); }
    Ok(SurfaceQualificationReceipt { vertex_count,face_count:triangles.len() as u32,
        half_edge_count:summary.half_edge_count,component_count:components })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualify(vertex_count:u32,triangles:&[[u32;3]]) -> Result<SurfaceQualificationReceipt,SurfaceQualificationError> {
        let b=surface_qualification_bounds(vertex_count,triangles.len()).unwrap();
        let mut edges=vec![HalfEdge::default();b.half_edges];
        let mut slots=vec![EdgeSlot::default();b.edge_slots];
        let mut starts=vec![0;b.vertex_starts]; let mut visited=vec![false;b.visited_half_edges];
        let mut labels=vec![0;b.face_labels]; let mut queue=vec![0;b.face_queue];
        qualify_surface_topology_into(vertex_count,triangles,b.work,&mut edges,&mut slots,&mut starts,&mut visited,&mut labels,&mut queue)
    }
    const TET:[[u32;3];4]=[[0,2,1],[0,1,3],[1,2,3],[2,0,3]];
    const ICOS:[[u32;3];20]=[
        [0,11,5],[0,5,1],[0,1,7],[0,7,10],[0,10,11],
        [1,5,9],[5,11,4],[11,10,2],[10,7,6],[7,1,8],
        [3,9,4],[3,4,2],[3,2,6],[3,6,8],[3,8,9],
        [4,9,5],[2,4,11],[6,2,10],[8,6,7],[9,8,1],
    ];
    #[test]
    fn qualifies_closed_tetrahedron_and_icosahedron() {
        let a=qualify(4,&TET).unwrap();assert_eq!(a.component_count,1);assert_eq!(a.half_edge_count,12);
        let b=qualify(12,&ICOS).unwrap();assert_eq!(b.face_count,20);assert_eq!(b.component_count,1);
    }
    #[test]
    fn refuses_open_and_disconnected_closed_surfaces() {
        assert!(matches!(qualify(4,&TET[..3]),Err(SurfaceQualificationError::BoundaryEdges(_))));
        let mut pair=TET.to_vec(); pair.extend(TET.iter().map(|t|t.map(|v|v+4)));
        assert_eq!(qualify(8,&pair),Err(SurfaceQualificationError::Disconnected(2)));
        assert_eq!(qualify(5,&TET),Err(SurfaceQualificationError::UnusedVertex(4)));
    }
    #[test]
    fn refuses_closed_connected_edges_with_two_disjoint_vertex_fans() {
        // Icosahedron opposite vertices 0 and 3 share no edge/neighbour.
        // Identify them and compact the remaining indices: edges still pair
        // and faces remain connected, but vertex 0 has two distinct fans.
        let pinched:Vec<[u32;3]>=ICOS.iter().map(|t|t.map(|v|if v==3 {0} else if v>3 {v-1} else {v})).collect();
        assert_eq!(qualify(11,&pinched),Err(SurfaceQualificationError::InvalidVertexLink(0)));
    }
    #[test]
    fn exhausted_work_and_short_buffers_refuse_before_scratch_writes() {
        let b=surface_qualification_bounds(4,TET.len()).unwrap();
        let sentinel=HalfEdge{origin:9,twin:9,next:9,face:9};
        let mut edges=vec![sentinel;b.half_edges];let mut slots=vec![EdgeSlot{key:9,half_edge:9,_reserved:9};b.edge_slots];
        let mut starts=vec![9;b.vertex_starts];let mut visited=vec![true;b.visited_half_edges];let mut labels=vec![9;b.face_labels];let mut queue=vec![9;b.face_queue];
        assert_eq!(qualify_surface_topology_into(4,&TET,b.work-1,&mut edges,&mut slots,&mut starts,&mut visited,&mut labels,&mut queue),Err(SurfaceQualificationError::WorkLimitExceeded{required:b.work}));
        assert!(edges.iter().all(|v|*v==sentinel));assert!(slots.iter().all(|v|v.key==9 && v.half_edge==9 && v._reserved==9));
        assert!(starts.iter().all(|v|*v==9));assert!(visited.iter().all(|v|*v));assert!(labels.iter().all(|v|*v==9));assert!(queue.iter().all(|v|*v==9));
        assert_eq!(qualify_surface_topology_into(4,&TET,b.work,&mut edges[..b.half_edges-1],&mut slots,&mut starts,&mut visited,&mut labels,&mut queue),Err(SurfaceQualificationError::InsufficientScratch));
        assert!(edges.iter().all(|v|*v==sentinel));
    }
    #[test]
    fn checked_bounds_refuse_empty_and_oversized_inputs() {
        assert_eq!(surface_qualification_bounds(0,4),Err(SurfaceQualificationError::EmptyMesh));
        assert_eq!(surface_qualification_bounds(4,0),Err(SurfaceQualificationError::EmptyMesh));
        assert_eq!(surface_qualification_bounds(4,usize::MAX),Err(SurfaceQualificationError::SizeOverflow));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetedSurfaceQualificationError { Qualification(SurfaceQualificationError), TopologyBuild(BudgetedTopologyError) }
impl From<SurfaceQualificationError> for BudgetedSurfaceQualificationError {
    fn from(value:SurfaceQualificationError) -> Self { Self::Qualification(value) }
}
/// Reserve fixed work, charge actual probes; late errors dirty scratch and consume budget.
pub fn qualify_surface_topology_budgeted_into(
    vertex_count:u32,triangles:&[[u32;3]],remaining_work:&mut usize,
    half_edges:&mut [HalfEdge],edge_slots:&mut [EdgeSlot],vertex_starts:&mut [u32],
    visited_half_edges:&mut [bool],face_labels:&mut [u32],face_queue:&mut [u32],
) -> Result<SurfaceQualificationReceipt,BudgetedSurfaceQualificationError> {
    let bounds=surface_qualification_bounds(vertex_count,triangles.len())?;
    let hash_bound=bounds.half_edges.checked_mul(2).and_then(|n|n.checked_mul(bounds.edge_slots)).ok_or(SurfaceQualificationError::SizeOverflow)?;
    let fixed=bounds.work.checked_sub(hash_bound).ok_or(SurfaceQualificationError::SizeOverflow)?;
    let minimum=fixed.checked_add(1).ok_or(SurfaceQualificationError::SizeOverflow)?;
    if *remaining_work<minimum { return Err(SurfaceQualificationError::WorkLimitExceeded{required:minimum}.into()); }
    if half_edges.len()<bounds.half_edges || edge_slots.len()<bounds.edge_slots ||
       vertex_starts.len()<bounds.vertex_starts || visited_half_edges.len()<bounds.visited_half_edges ||
       face_labels.len()<bounds.face_labels || face_queue.len()<bounds.face_queue {
        return Err(SurfaceQualificationError::InsufficientScratch.into());
    }
    *remaining_work -= fixed;
    let edges=&mut half_edges[..bounds.half_edges];
    let summary=build_triangle_half_edges_budgeted(vertex_count,triangles,edges,&mut edge_slots[..bounds.edge_slots],remaining_work).map_err(BudgetedSurfaceQualificationError::TopologyBuild)?;
    if summary.boundary_half_edges!=0 { return Err(SurfaceQualificationError::BoundaryEdges(summary.boundary_half_edges).into()); }
    let traversal_work=triangle_ordered_component_work(triangles.len() as u32).map_err(|_|SurfaceQualificationError::SizeOverflow)?;
    let components=label_components_triangle_ordered(triangles.len() as u32,edges,traversal_work,&mut face_labels[..bounds.face_labels],&mut face_queue[..bounds.face_queue]).map_err(SurfaceQualificationError::OrderedConnectivity)?;
    if components!=1 { return Err(SurfaceQualificationError::Disconnected(components).into()); }
    let starts=&mut vertex_starts[..bounds.vertex_starts];starts.fill(INVALID_INDEX);
    let visited=&mut visited_half_edges[..bounds.visited_half_edges];visited.fill(false);
    for (i,edge) in edges.iter().enumerate() {
        let start=&mut starts[edge.origin as usize];
        if *start==INVALID_INDEX { *start=i as u32; }
    }
    for vertex in 0..vertex_count {
        let start=starts[vertex as usize];
        if start==INVALID_INDEX { return Err(SurfaceQualificationError::UnusedVertex(vertex).into()); }
        let mut cur=start; let mut steps=0usize;
        loop {
            let i=cur as usize;
            if i>=edges.len() || edges[i].origin!=vertex { return Err(SurfaceQualificationError::InvalidVertexLink(vertex).into()); }
            if visited[i] {
                if cur==start && steps>0 { break; }
                return Err(SurfaceQualificationError::InvalidVertexLink(vertex).into());
            }
            visited[i]=true;steps+=1;
            // For v→a, the preceding edge b→v is next(next(v→a)); its
            // twin v→b advances to the next triangle incident on v.
            let next=edges[i].next as usize;
            if next>=edges.len() { return Err(SurfaceQualificationError::InvalidVertexLink(vertex).into()); }
            let prev=edges[next].next as usize;
            if prev>=edges.len() || edges[prev].twin==INVALID_INDEX { return Err(SurfaceQualificationError::InvalidVertexLink(vertex).into()); }
            cur=edges[prev].twin;
            if steps>edges.len() { return Err(SurfaceQualificationError::InvalidVertexLink(vertex).into()); }
        }
    }
    // More than one fan at a vertex leaves additional outgoing half-edges
    // unvisited even if every edge is paired and all faces are connected.
    if let Some(i)=visited.iter().position(|x|!*x) { return Err(SurfaceQualificationError::InvalidVertexLink(edges[i].origin).into()); }
    Ok(SurfaceQualificationReceipt { vertex_count,face_count:triangles.len() as u32,
        half_edge_count:summary.half_edge_count,component_count:components })
}

