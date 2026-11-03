use super::*;
use crate::specialized_libs::computational_geometry::{
    allocation_counter::assert_zero_alloc, exact_test_helper::{expansion_to_exact, Exact},
};

fn exact_equal(a: &ExactPoint3, b: &ExactPoint3) -> bool {
    let da = expansion_to_exact(&a.den[..a.den_len]);
    let db = expansion_to_exact(&b.den[..b.den_len]);
    [
        (&a.x_num[..a.x_num_len], &b.x_num[..b.x_num_len]),
        (&a.y_num[..a.y_num_len], &b.y_num[..b.y_num_len]),
        (&a.z_num[..a.z_num_len], &b.z_num[..b.z_num_len]),
    ]
    .iter()
    .all(|(x, y)| {
        expansion_to_exact(x)
            .mul(db.clone())
            .equals(&expansion_to_exact(y).mul(da.clone()))
    })
}

// Independent existing BigInt owner, outside the measured zero-allocation path.
fn plane_oracle(t: [Point3; 3], p: &ExactPoint3) -> Sign {
    let a = coords(t[0]).map(Exact::from_f64);
    let b = coords(t[1]).map(Exact::from_f64);
    let c = coords(t[2]).map(Exact::from_f64);
    let den = expansion_to_exact(&p.den[..p.den_len]);
    let nums = [expansion_to_exact(&p.x_num[..p.x_num_len]),
        expansion_to_exact(&p.y_num[..p.y_num_len]), expansion_to_exact(&p.z_num[..p.z_num_len])];
    let mut sum = Exact::from_f64(0.);
    for i in 0..3 {
        let j = (i+1)%3;
        let k = (i+2)%3;
        let n = b[j].clone().sub(a[j].clone()).mul(c[k].clone().sub(a[k].clone()))
            .sub(b[k].clone().sub(a[k].clone()).mul(c[j].clone().sub(a[j].clone())));
        sum = sum.add(n.mul(nums[i].clone().sub(a[i].clone().mul(den.clone()))));
    }
    sum.sign()
}

#[test]
fn actual_tiny_z_pair_reversed_edges_have_identical_exact_intersections() {
    let a = [
        Point3::new(0.08810000121593475, 0.47600001096725464, 0.),
        Point3::new(0.05656854063272476, 0.800000011920929, -0.05656854063272476),
        Point3::new(0.07999999821186066, 0.800000011920929, 0.),
    ];
    let b = [
        Point3::new(0., 0.7202500104904175, 9.766558715541163e-18),
        Point3::new(0., 0.7436082363128662, -0.05639176815748215),
        Point3::new(0.3240000009536743, 0.750696063041687, -0.04930390417575836),
    ];
    let mut acc = [0.; CAPACITY];
    let mut next = [0.; CAPACITY];
    let mut work = 100_000_000;
    let cancelled = || false;
    let mut scratch = Scratch::new(&mut acc, &mut next, &cancelled).unwrap();
    for (source, target) in [(a, b), (b, a)] {
        let plane = Plane3::new(target, &mut scratch, &mut work).unwrap();
        let source_plane = Plane3::new(source, &mut scratch, &mut work).unwrap();
        for i in 0..3 {
            let mut forward = None;
            let mut reverse = None;
            assert_zero_alloc("checked original-coordinate line plane", || {
                forward =
                    Some(plane.intersect(source[i], source[(i + 1) % 3], &mut scratch, &mut work));
                reverse =
                    Some(plane.intersect(source[(i + 1) % 3], source[i], &mut scratch, &mut work));
            });
            match (forward.unwrap(), reverse.unwrap()) {
                (Ok((p, _)), Ok((q, _))) => {
                    assert!(exact_equal(&p, &q));
                    for point in [&p, &q] {
                        assert_eq!(plane_oracle(source, point), Sign::Zero);
                        assert_eq!(plane_oracle(target, point), Sign::Zero);
                        assert_eq!(
                            plane.orient(point, &mut scratch, &mut work).unwrap(),
                            Sign::Zero
                        );
                        assert_eq!(
                            source_plane.orient(point, &mut scratch, &mut work).unwrap(),
                            Sign::Zero
                        );
                    }
                }
                (Err(Error::Parallel), Err(Error::Parallel)) => {}
                (x, y) => panic!("inconsistent edge constructions: {x:?} / {y:?}"),
            }
        }
    }
}

#[test]
fn finite_parameter_spans_and_projected_containment_preserve_exact_edges() {
    let t = [Point3::new(0.,0.,0.), Point3::new(2.,0.,0.), Point3::new(0.,2.,0.)];
    let mut acc = [0.; CAPACITY];
    let mut next = [0.; CAPACITY];
    let running = || false;
    let mut scratch = Scratch::new(&mut acc, &mut next, &running).unwrap();
    let mut work = 10_000_000;
    let triangle = Triangle3::new(t, &mut scratch, &mut work).unwrap();
    for (z0,z1,want) in [(1.,2.,ParameterSpan::BeforeStart), (2.,1.,ParameterSpan::BeyondEnd),
        (0.,1.,ParameterSpan::AtStart), (1.,0.,ParameterSpan::AtEnd), (-1.,1.,ParameterSpan::Interior)] {
        let (p, span) = triangle.plane.intersect(Point3::new(0.5,0.5,z0), Point3::new(0.5,0.5,z1), &mut scratch, &mut work).unwrap();
        assert_eq!(span, want);
        assert_eq!(plane_oracle(t,&p), Sign::Zero);
        assert!(triangle.contains(&p, &mut scratch, &mut work).unwrap());
    }
    let edge = ExactPoint3::from_point3(Point3::new(1.,1.,0.));
    assert!(triangle.contains(&edge,&mut scratch,&mut work).unwrap());
    let mut outside = edge.clone();
    outside.x_num[..2].copy_from_slice(&[2f64.powi(-54),1.]);
    outside.x_num_len=2;
    assert_eq!(outside.to_point3(),edge.to_point3());
    assert!(!triangle.contains(&outside,&mut scratch,&mut work).unwrap());
    assert_eq!(triangle.side(&outside,1,&mut scratch,&mut work).unwrap(),Sign::Negative);
}

#[test]
fn cancellation_during_checked_arithmetic_refuses_before_success() {
    use std::cell::Cell;
    let checks = Cell::new(0usize);
    let cancel = || { checks.set(checks.get()+1); checks.get() >= 15 };
    let mut a=[0.;CAPACITY];
    let mut b=[0.;CAPACITY];
    let mut scratch=Scratch::new(&mut a,&mut b,&cancel).unwrap();
    let mut work=1_000_000;
    let result=Plane3::new([Point3::new(0.,0.,0.),Point3::new(1.,0.,0.),Point3::new(0.,1.,0.)],&mut scratch,&mut work);
    assert!(matches!(result,Err(Error::Cancelled)));
    assert_eq!(checks.get(),15);
    assert!(work<1_000_000);
}

#[test]
fn exact_difference_plane_contains_all_original_tiny_offset_corners() {
    let e = 2f64.powi(-60);
    let t = [
        Point3::new(e, e, e),
        Point3::new(1., 0., 0.),
        Point3::new(0., 1., 0.),
    ];
    let mut acc = [0.; CAPACITY];
    let mut next = [0.; CAPACITY];
    let mut work = 1000000;
    let cancelled = || false;
    let mut scratch = Scratch::new(&mut acc, &mut next, &cancelled).unwrap();
    let plane = Plane3::new(t, &mut scratch, &mut work).unwrap();
    for p in t {
        assert_eq!(
            plane
                .orient(&ExactPoint3::from_point3(p), &mut scratch, &mut work)
                .unwrap(),
            Sign::Zero
        );
    }
}

#[test]
fn checked_owner_refuses_capacity_work_cancellation_and_unrepresentable_products() {
    let mut a = [0.; CAPACITY];
    let mut b = [0.; CAPACITY];
    let running = || false;
    let cancelled = || true;
    assert!(matches!(
        Scratch::new(&mut a[..CAPACITY - 1], &mut b, &running),
        Err(Error::Capacity)
    ));
    let t = [
        Point3::new(0., 0., 0.),
        Point3::new(1., 0., 0.),
        Point3::new(0., 1., 0.),
    ];
    let mut work = 0;
    let mut scratch = Scratch::new(&mut a, &mut b, &running).unwrap();
    assert!(matches!(
        Plane3::new(t, &mut scratch, &mut work),
        Err(Error::Work)
    ));
    let mut work = 1000000;
    let mut scratch = Scratch::new(&mut a, &mut b, &cancelled).unwrap();
    assert!(matches!(
        Plane3::new(t, &mut scratch, &mut work),
        Err(Error::Cancelled)
    ));
    assert!(exact_product_is_supported(f64::from_bits(1), 1.));
    assert!(!exact_product_is_supported(f64::from_bits(1), 0.5));
}
