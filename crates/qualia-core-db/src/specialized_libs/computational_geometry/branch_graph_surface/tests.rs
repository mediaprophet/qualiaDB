use super::*;
use super::super::{Point3,ROOT_PARENT,HalfEdge,EdgeSlot,distance::Aabb,indexed_tetra_surface::TetraEdgeSlot,tapered_branch_field::TaperedFieldSegment};
fn spec()->BranchGraphSpec{BranchGraphSpec{origin:Point3::new(-1.43,-0.43,-0.43),spacing:0.1,dimensions:[30,10,40],
    minimum_radius_samples:2.,min_axis:1e-6,projection_tolerance:1e-8,min_area_sq:1e-24,max_incident_cosine:0.99}}
fn graph()->(Vec<Point3>,Vec<u32>,Vec<f64>){(vec![Point3::new(0.,0.,0.),Point3::new(0.,0.,1.),
    Point3::new(0.,0.,2.),Point3::new(0.,0.,3.),Point3::new(1.,0.,1.),Point3::new(-1.,0.,2.)],vec![ROOT_PARENT,0,1,2,1,2],vec![0.22;6])}
fn run(p:&[Point3],parents:&[u32],r:&[f64],s:BranchGraphSpec,cancelled:bool)->Result<(Vec<Point3>,Vec<[u32;3]>,BranchGraphReceipt),BranchGraphError>{
    let limits=BranchGraphLimits{vertices:10_000,triangles:20_000,..Default::default()};
    let b=branch_graph_surface_bounds(p.len(),s,limits)?;let q=b.topology;let zero=Point3::default();
    let mut samples=vec![zero;b.samples];let mut values=vec![0.;b.samples];
    let mut seg=vec![TaperedFieldSegment{a:zero,b:zero,radius_a:0.,radius_b:0.};b.segments];let mut boxes=vec![Aabb::new(zero,zero);b.segments];
    let mut v=vec![zero;b.vertices];let mut t=vec![[0;3];b.triangles];let mut normals=vec![zero;b.vertices];let mut faces=vec![zero;b.triangles];
    let mut sources=vec![[0;2];b.vertices];let mut slots=vec![TetraEdgeSlot::default();b.extraction_slots];
    let mut he=vec![HalfEdge::default();q.half_edges];let mut ts=vec![EdgeSlot::default();q.edge_slots];
    let mut starts=vec![0;q.vertex_starts];let mut visited=vec![false;q.visited_half_edges];let mut labels=vec![0;q.face_labels];let mut queue=vec![0;q.face_queue];
    let receipt=branch_graph_surface_into(BranchGraphInput{positions:p,parents,radii:r},s,limits,
        BranchGraphBuffers{samples:&mut samples,values:&mut values,segments:&mut seg,boxes:&mut boxes,
            vertices:&mut v,triangles:&mut t,normals:&mut normals,face_normals:&mut faces,sample_edges:&mut sources,
            extraction_slots:&mut slots,half_edges:&mut he,topology_slots:&mut ts,vertex_starts:&mut starts,
            visited:&mut visited,face_labels:&mut labels,face_queue:&mut queue},&||cancelled)?;
    assert_eq!(receipt.topology.component_count,1);
    assert!(normals[..receipt.vertices].iter().all(|n|(n.x*n.x+n.y*n.y+n.z*n.z-1.).abs()<1e-8));
    assert!(sources[..receipt.vertices].iter().all(|s|s[0]<s[1]&&s[1]<b.samples));
    v.truncate(receipt.vertices);t.truncate(receipt.triangles);Ok((v,t,receipt))
}
#[test]
fn two_junction_graph_is_one_repeatable_closed_surface(){
    let(p,parents,r)=graph();let(a,t,receipt)=run(&p,&parents,&r,spec(),false).unwrap();
    let(b,u,_)=run(&p,&parents,&r,spec(),false).unwrap();assert_eq!(a,b);assert_eq!(t,u);
    assert_eq!(receipt.smallest_radius_over_spacing,0.22/0.1);assert!(receipt.vertices>0);
}
#[test]
fn coarse_missing_parent_contact_and_cancel_fail_closed(){
    let(mut p,mut parents,r)=graph();let mut coarse=spec();coarse.spacing=0.2;
    assert!(matches!(run(&p,&parents,&r,coarse,false),Err(BranchGraphError::Undersampled{..})));
    parents[3]=3;assert!(matches!(run(&p,&parents,&r,spec(),false),Err(BranchGraphError::InvalidInput)));parents[3]=2;
    p[5]=Point3::new(0.1,0.,0.5);
    assert!(matches!(run(&p,&parents,&r,spec(),false),Err(BranchGraphError::NonincidentClearanceUnproven{..})));
    assert!(matches!(run(&p,&parents,&r,spec(),true),Err(BranchGraphError::Cancelled)));
    let tiny=BranchGraphLimits{bytes:16,..Default::default()};
    assert!(matches!(branch_graph_surface_bounds(6,spec(),tiny),Err(BranchGraphError::Budget)));
}
