use super::*;
use super::super::super::allocation_counter::assert_zero_alloc;
fn running() -> bool { false }

fn tiny_z_pair() -> [[Point3; 3]; 2] {
    [[Point3::new(0.08810000121593475,0.47600001096725464,0.),
      Point3::new(0.05656854063272476,0.800000011920929,-0.05656854063272476),
      Point3::new(0.07999999821186066,0.800000011920929,0.)],
     [Point3::new(0.,0.7202500104904175,9.766558715541163e-18),
      Point3::new(0.,0.7436082363128662,-0.05639176815748215),
      Point3::new(0.3240000009536743,0.750696063041687,-0.04930390417575836)]]
}

#[test]
fn logged_tiny_z_pair_all_windings_share_registry_endpoints_without_hot_allocation() {
    let [a,b]=tiny_z_pair();
    let mut work=100_000_000;
    let mut scratch=exact_registry::Scratch::new(&mut work,&running).unwrap();
    let mut registry=exact_registry::Registry::new(32);
    for p in a.into_iter().chain(b) {registry.plain(p,&mut work).unwrap();}
    let mut expected=edge_collection::collect(a,b,&mut registry,&mut work,&mut scratch).unwrap().unwrap();
    expected.sort();
    let count=registry.nodes.len();
    // Cold registry/node admission is outside the measured window. Repeated
    // exact construction, predicates and identity lookup must allocate nothing.
    assert_zero_alloc("checked finite collector repeat", || {
        for (one,two) in [(a,b),(b,a),([a[2],a[1],a[0]],b),(a,[b[1],b[2],b[0]])] {
            let mut got=edge_collection::collect(one,two,&mut registry,&mut work,&mut scratch).unwrap().unwrap();
            got.sort();
            assert_eq!(got,expected);
        }
    });
    assert_eq!(registry.nodes.len(),count);
    for face in [a,b] {
        let facet=exact_registry::triangle(face,&mut work,&mut scratch).unwrap();
        for id in expected {
            assert!(exact_registry::inside_on(&registry.nodes[id as usize],&facet,&mut work,&mut scratch).unwrap());
        }
    }
}

#[test]
fn in_plane_edge_is_clipped_by_opposite_edges_and_infinite_extensions_are_rejected() {
    let a=[Point3::new(0.,0.,0.),Point3::new(2.,0.,0.),Point3::new(0.,2.,0.)];
    let b=[Point3::new(0.5,-1.,0.),Point3::new(0.5,3.,0.),Point3::new(0.5,1.,1.)];
    let mut work=10_000_000;
    let mut scratch=exact_registry::Scratch::new(&mut work,&running).unwrap();
    let mut registry=exact_registry::Registry::new(32);
    for p in a.into_iter().chain(b) {registry.plain(p,&mut work).unwrap();}
    let ids=edge_collection::collect(a,b,&mut registry,&mut work,&mut scratch).unwrap().unwrap();
    let endpoints=ids.map(|id| registry.nodes[id as usize].p);
    assert!(endpoints.contains(&Point3::new(0.5,0.,0.)));
    assert!(endpoints.contains(&Point3::new(0.5,1.5,0.)));
    let outside=[Point3::new(0.5,4.,0.),Point3::new(0.5,6.,0.),Point3::new(0.5,5.,1.)];
    assert!(edge_collection::collect(a,outside,&mut registry,&mut work,&mut scratch).unwrap().is_none());
}

#[test]
fn coplanar_partial_area_remains_explicitly_unsupported_but_shared_boundary_is_not_area() {
    let a=[Point3::new(0.,0.,0.),Point3::new(2.,0.,0.),Point3::new(0.,2.,0.)];
    let area=[Point3::new(0.5,0.,0.),Point3::new(2.5,0.,0.),Point3::new(0.5,2.,0.)];
    let touch=[a[1],a[0],Point3::new(1.,-1.,0.)];
    let mut work=10_000_000;
    let mut scratch=exact_registry::Scratch::new(&mut work,&running).unwrap();
    let mut registry=exact_registry::Registry::new(32);
    assert!(matches!(edge_collection::collect(a,area,&mut registry,&mut work,&mut scratch),Err(Error::CoplanarOverlap)));
    assert!(edge_collection::collect(a,touch,&mut registry,&mut work,&mut scratch).unwrap().is_none());
    assert!(edge_collection::collect(a,[a[2],a[1],a[0]],&mut registry,&mut work,&mut scratch).unwrap().is_none());
    assert!(registry.nodes.is_empty());
}

#[test]
fn checked_collector_propagates_work_and_mid_operation_cancellation() {
    use std::cell::Cell;
    let [a,b]=tiny_z_pair();
    let checks=Cell::new(0usize);
    let cancel=|| {checks.set(checks.get()+1);checks.get()>=20};
    let mut work=1_000_000;
    let mut scratch=exact_registry::Scratch::new(&mut work,&cancel).unwrap();
    let mut registry=exact_registry::Registry::new(32);
    assert!(matches!(edge_collection::collect(a,b,&mut registry,&mut work,&mut scratch),Err(Error::Cancelled)));
    assert!(registry.nodes.is_empty());
    let mut work=1_000_000;
    let mut scratch=exact_registry::Scratch::new(&mut work,&running).unwrap();
    let mut exhausted=0;
    assert!(matches!(edge_collection::collect(a,b,&mut registry,&mut exhausted,&mut scratch),Err(Error::Work)));
    assert!(registry.nodes.is_empty());
}
