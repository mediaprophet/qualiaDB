use super::*;
fn never_cancel() -> bool { false }
#[test]
fn exact_product_admission_preserves_subnormal_times_one_and_refuses_lost_half_bit() {
    use super::super::super::allocation_counter::assert_zero_alloc;
    let smallest = f64::from_bits(1);
    let p = ExactPoint3::from_point3(Point3::new(smallest, -0., 0.));
    let mut work = 1000000;
    let mut scratch = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let mut r = exact_registry::Registry::new(8);
    let id = r.constructed(p.clone(), &mut work, &mut scratch).unwrap();
    let mut unsupported = ExactPoint3::from_point3(Point3::new(0., 0., -0.));
    unsupported.den[0] = 0.5;
    assert_zero_alloc("exact subnormal product admission", || {
        assert_eq!(
            r.constructed(p.clone(), &mut work, &mut scratch).unwrap(),
            id
        );
        assert!(matches!(
            r.constructed(unsupported.clone(), &mut work, &mut scratch),
            Err(Error::Arithmetic)
        ));
    });
    assert_eq!(r.nodes.len(), 1);
    // This normal numerator divided by2^52 denotes the same min-subnormal.
    let mut normal = ExactPoint3::from_point3(Point3::new(f64::MIN_POSITIVE, 0., 0.));
    normal.den[0] = 2f64.powi(52);
    assert_eq!(r.constructed(normal, &mut work, &mut scratch).unwrap(), id);
}
#[test]
fn extra_interior_points_do_not_hide_missing_original_facet_corners() {
    let triangle = [
        Point3::new(0., 0., 0.),
        Point3::new(4., 0., 0.),
        Point3::new(0., 4., 0.),
    ];
    let mut work = 1000000;
    let mut r = exact_registry::Registry::new(8);
    for p in [
        Point3::new(1., 1., 0.),
        Point3::new(2., 1., 0.),
        Point3::new(1., 2., 0.),
    ] {
        r.plain(p, &mut work).unwrap();
    }
    let mut scratch = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    assert!(matches!(
        facet_constraints::triangulate(
            0,
            triangle,
            &r,
            &[],
            &[triangle],
            &mut work,
            &|| false,
            &mut scratch
        ),
        Err(Error::FacetIncidence {
            admitted: 3,
            corners: [false, false, false]
        })
    ));
}
#[test]
fn actual_third_arm_equal_expansions_share_one_node_without_allocating() {
    use super::super::super::allocation_counter::assert_zero_alloc;
    let old = [
        [3.344251798829065e-20, 0.00017451949501120633, 0.],
        [
            -2.033832372292349e-21,
            2.0328790734103208e-19,
            0.002250128288766181,
        ],
        [
            7.88786872104171e-23,
            1.2281977735187355e-20,
            -0.00016940267167579796,
        ],
    ];
    let next = [
        [6.337463676153038e-21, 0.00017451949501120636, 0.],
        [
            -3.397664777837484e-22,
            2.0159384144652348e-19,
            0.002250128288766181,
        ],
        [
            2.593912800702333e-23,
            1.2334917294390749e-20,
            -0.00016940267167579796,
        ],
    ];
    fn point(values: [[f64; 3]; 3]) -> ExactPoint3 {
        let mut p = ExactPoint3::from_point3(Point3::new(0., 0., 0.));
        p.x_num[..2].copy_from_slice(&values[0][..2]);
        p.x_num_len = 2;
        p.y_num[..3].copy_from_slice(&values[1]);
        p.y_num_len = 3;
        p.z_num[..3].copy_from_slice(&values[2]);
        p.z_num_len = 3;
        p.den[..3].copy_from_slice(&[
            -1.257314531080602e-22,
            -1.6898307297723292e-19,
            0.0030094159241648385,
        ]);
        p.den_len = 3;
        p
    }
    let mut work = 10_000_000;
    let mut scratch = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let mut registry = exact_registry::Registry::new(8);
    let id = registry
        .constructed(point(old), &mut work, &mut scratch)
        .unwrap();
    let next = point(next);
    assert_zero_alloc("actual third arm rational identity", || {
        assert_eq!(
            registry
                .constructed(next.clone(), &mut work, &mut scratch)
                .unwrap(),
            id
        );
    });
    assert_eq!(registry.nodes.len(), 1);
}
#[test]
fn original_corner_incidence_is_exact_but_constructed_low_tail_is_not_a_corner() {
    let e = 2f64.powi(-60);
    let triangle = [
        Point3::new(e, e, e),
        Point3::new(1., 0., 0.),
        Point3::new(0., 1., 0.),
    ];
    let mut work = 1000000;
    let mut scratch = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let mut r = exact_registry::Registry::new(16);
    for corner in triangle {
        let id = r.plain(corner, &mut work).unwrap();
        assert_eq!(
            exact_registry::plane(&r.nodes[id as usize], triangle, &mut work, &mut scratch).unwrap(),
            Sign::Zero
        );
        assert!(exact_registry::inside(&r.nodes[id as usize], triangle, &mut work, &mut scratch).unwrap());
    }
    // The rounded projection equals the first corner of this plane, but the
    // exact rational lies strictly off it. It must reach the exact predicate.
    let plane = [
        Point3::new(1., 0., 0.),
        Point3::new(1., 1., 0.),
        Point3::new(1., 0., 1.),
    ];
    let mut p = ExactPoint3::from_point3(plane[0]);
    p.x_num[0] = 2f64.powi(-54);
    p.x_num[1] = 1.;
    p.x_num_len = 2;
    let mut separate = exact_registry::Registry::new(8);
    let id = separate.constructed(p, &mut work, &mut scratch).unwrap();
    assert_eq!(separate.nodes[id as usize].p, plane[0]);
    assert_ne!(
        exact_registry::plane(&separate.nodes[id as usize], plane, &mut work, &mut scratch).unwrap(),
        Sign::Zero
    );
}
#[test]
fn actual_cap_side_shared_edge_retains_exact_input_endpoints() {
    let p = Point3::new(0.07999999821186066, 0.800000011920929, 0.);
    let q = Point3::new(0.05656854063272476, 0.800000011920929, -0.05656854063272476);
    let a = [
        Point3::new(
            -3.469446951953614e-18,
            0.800000011920929,
            8.673617379884035e-19,
        ),
        p,
        q,
    ];
    let b = [
        p,
        q,
        Point3::new(0.04797719419002533, 1.124000072479248, -0.04797719419002533),
    ];
    for face in [a, b] {
        for corner in face {
            assert_eq!(
                super::super::super::orient_3d_exact_3(
                    face[0],
                    face[1],
                    face[2],
                    &ExactPoint3::from_point3(corner)
                ),
                Sign::Zero,
                "own corner {corner:?} of {face:?}"
            );
        }
    }
    assert_ne!(
        super::super::super::orient_3d_exact_3(a[0], a[1], a[2], &ExactPoint3::from_point3(b[2])),
        Sign::Zero
    );
    let mut work = 1000000;
    let mut scratch = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let mut registry = exact_registry::Registry::new(16);
    let ids = [
        registry.plain(p, &mut work).unwrap(),
        registry.plain(q, &mut work).unwrap(),
    ];
    let mut collected = edge_collection::collect(a, b, &mut registry, &mut work, &mut scratch).unwrap().unwrap();
    collected.sort();
    assert_eq!(collected, ids);
    assert_eq!(registry.nodes.len(), 2);
}
#[test]
fn interior_intersection_endpoint_is_retained_as_a_constrained_shared_vertex() {
    let a = [
        Point3::new(0., 0., 0.),
        Point3::new(2., 0., 0.),
        Point3::new(0., 2., 0.),
    ];
    let b = [
        Point3::new(0.5, 0.5, -1.),
        Point3::new(0.5, 0.5, 1.),
        Point3::new(0.5, 2., 0.),
    ];
    let mut work = 20_000_000;
    let mut arithmetic = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let mut r = exact_registry::Registry::new(64);
    for p in a.into_iter().chain(b) {
        r.plain(p, &mut work).unwrap();
    }
    let [p,q] = edge_collection::collect(a,b,&mut r,&mut work,&mut arithmetic).unwrap().unwrap();
    assert_eq!(edge_collection::collect(a,b,&mut r,&mut work,&mut arithmetic).unwrap(), Some([p,q]));
    let cuts = [Cut {
        faces: [0, 1],
        nodes: [p, q],
    }];
    let triangles = facet_constraints::triangulate(
        0,
        a,
        &r,
        &cuts,
        &[a, b],
        &mut work,
        &|| false,
        &mut arithmetic,
    )
    .unwrap();
    assert!(triangles.iter().any(|t| t.contains(&p)));
    assert!(triangles.iter().any(|t| t.contains(&q)));
    assert!(triangles.iter().any(|t| (0..3)
        .any(|i| (t[i] == p && t[(i + 1) % 3] == q) || (t[i] == q && t[(i + 1) % 3] == p))));
    let mut area = 0.;
    for t in triangles {
        let v = t.map(|i| r.nodes[i as usize].p);
        let n = tri_normal(v[0], v[1], v[2]);
        assert!(n.z > 0.);
        area += n.z;
    }
    assert!((area - 4.).abs() < 1e-12);
}
#[test]
fn shared_registry_does_not_weld_distinct_close_input_points() {
    let mut r = exact_registry::Registry::new(8);
    let mut work = 100000;
    let mut arithmetic = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let a = r.plain(Point3::new(1., 0., 0.), &mut work).unwrap();
    let b = r
        .constructed(
            ExactPoint3::from_point3(Point3::new(f64::from_bits(1f64.to_bits() + 1), 0., 0.)),
            &mut work,
            &mut arithmetic,
        )
        .unwrap();
    assert_ne!(a, b);
}

#[test]
fn rational_identity_uses_compressed_cross_products_not_representation_or_zero_tail() {
    let mut r = exact_registry::Registry::new(8);
    let mut work = 100000;
    let mut arithmetic = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let p = Point3::new(1., -2., 0.5);
    let first = r.plain(p, &mut work).unwrap();
    let mut scaled = ExactPoint3::from_point3(p);
    scaled.den[0] = 2.;
    scaled.x_num[0] *= 2.;
    scaled.y_num[0] *= 2.;
    scaled.z_num[0] *= 2.;
    assert_eq!(
        r.constructed(scaled, &mut work, &mut arithmetic).unwrap(),
        first
    );
    for x in [
        f64::from_bits(1f64.to_bits() - 1),
        f64::from_bits(1f64.to_bits() + 1),
    ] {
        let id = r
            .constructed(
                ExactPoint3::from_point3(Point3::new(x, -2., 0.5)),
                &mut work,
                &mut arithmetic,
            )
            .unwrap();
        assert_ne!(id, first);
    }
    assert_eq!(r.nodes.len(), 3);
}

#[test]
fn comparison_scratch_is_admitted_once_and_predicates_make_no_allocation_calls() {
    use super::super::super::allocation_counter::assert_zero_alloc;
    let mut insufficient = 3 * 4096 - 1;
    assert!(matches!(
        exact_registry::Scratch::new(&mut insufficient, &never_cancel),
        Err(Error::Work)
    ));
    let mut work = 1_000_000;
    let mut arithmetic = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let addresses = arithmetic.addresses();
    let mut r = exact_registry::Registry::new(8);
    let a = r.plain(Point3::new(1., 0., 0.), &mut work).unwrap();
    let b = r
        .plain(
            Point3::new(f64::from_bits(1f64.to_bits() + 1), 0., 0.),
            &mut work,
        )
        .unwrap();
    let mut scaled = ExactPoint3::from_point3(Point3::new(1., 0., 0.));
    scaled.x_num[0] = 2.;
    scaled.den[0] = 2.;
    assert_eq!(
        r.constructed(scaled.clone(), &mut work, &mut arithmetic)
            .unwrap(),
        a
    );
    assert_zero_alloc("shared exact comparison scratch", || {
        for _ in 0..32 {
            assert_eq!(
                exact_registry::cmp_axis(
                    &r.nodes[a as usize],
                    &r.nodes[b as usize],
                    0,
                    &mut work,
                    &mut arithmetic
                )
                .unwrap(),
                Sign::Negative
            );
            assert_eq!(
                exact_registry::cmp_axis(
                    &r.nodes[b as usize],
                    &r.nodes[a as usize],
                    0,
                    &mut work,
                    &mut arithmetic
                )
                .unwrap(),
                Sign::Positive
            );
            assert_eq!(
                r.constructed(scaled.clone(), &mut work, &mut arithmetic)
                    .unwrap(),
                a
            );
        }
    });
    assert_eq!(arithmetic.addresses(), addresses);
    let mut exhausted = 0;
    assert!(matches!(
        exact_registry::cmp_axis(
            &r.nodes[a as usize],
            &r.nodes[b as usize],
            0,
            &mut exhausted,
            &mut arithmetic
        ),
        Err(Error::Work)
    ));
}

#[test]
fn nonzero_exact_low_tail_that_rounds_away_is_refused_not_welded() {
    let mut work = 100000;
    let mut arithmetic = exact_registry::Scratch::new(&mut work, &never_cancel).unwrap();
    let mut r = exact_registry::Registry::new(8);
    r.plain(Point3::new(1., 0., 0.), &mut work).unwrap();
    let mut distinct = ExactPoint3::from_point3(Point3::new(1., 0., 0.));
    distinct.x_num[0] = 2f64.powi(-54);
    distinct.x_num[1] = 1.;
    distinct.x_num_len = 2;
    assert_eq!(distinct.to_point3().x, 1.);
    assert!(matches!(
        r.constructed(distinct, &mut work, &mut arithmetic),
        Err(Error::RegistryCollision { .. })
    ));
    assert_eq!(r.nodes.len(), 1);
}

#[test]
fn constructed_retained_copy_is_charged_before_allocation_or_insertion() {
    use super::super::super::allocation_counter::assert_zero_alloc;
    let mut cold_work = 100000;
    let mut arithmetic = exact_registry::Scratch::new(&mut cold_work, &never_cancel).unwrap();
    let mut registry = exact_registry::Registry::new(2);
    let point = ExactPoint3::from_point3(Point3::new(1., 2., 3.));
    let rounding = point.x_num_len + point.y_num_len + point.z_num_len + point.den_len;
    let copy = std::mem::size_of::<ExactPoint3>() / std::mem::size_of::<f64>();
    let mut insufficient = rounding + copy - 1;
    assert_zero_alloc("constructed copy budget refuses before Box", || {
        assert!(matches!(registry.constructed(point.clone(), &mut insufficient, &mut arithmetic), Err(Error::Work)));
    });
    assert!(registry.nodes.is_empty());
    let mut exact_budget = rounding + copy;
    assert_eq!(registry.constructed(point, &mut exact_budget, &mut arithmetic).unwrap(), 0);
    assert_eq!(exact_budget, 0);
    assert_eq!(registry.nodes.len(), 1);
}
