use super::*;
fn cube(x: f64) -> (Vec<Point3>, Vec<[u32; 3]>) {
    (
        vec![
            Point3::new(x, 0., 0.),
            Point3::new(x + 1., 0., 0.),
            Point3::new(x + 1., 1., 0.),
            Point3::new(x, 1., 0.),
            Point3::new(x, 0., 1.),
            Point3::new(x + 1., 0., 1.),
            Point3::new(x + 1., 1., 1.),
            Point3::new(x, 1., 1.),
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    )
}
fn run(
    x: f64,
    work: usize,
    bytes: usize,
    fragments: usize,
    output: usize,
) -> Result<(UnionReceipt, Vec<Point3>, Vec<[u32; 3]>, Vec<SourceFace>), BoundedUnionError> {
    let (a, ta) = cube(0.);
    let (b, tb) = cube(x);
    let z = Point3::new(0., 0., 0.);
    let (mut ca, mut cb) = (vec![[z; 3]; 12], vec![[z; 3]; 12]);
    let (mut current, mut next) = (vec![[z; 3]; fragments], vec![[z; 3]; fragments]);
    let (mut v, mut t, mut s) = (
        vec![z; output],
        vec![[0; 3]; output],
        vec![SourceFace::default(); output],
    );
    let r = union_exact_bounded_into(
        &a,
        &ta,
        &b,
        &tb,
        UnionBudget { work, bytes },
        UnionBuffers {
            corners_a: &mut ca,
            corners_b: &mut cb,
            current: &mut current,
            next: &mut next,
            vertices: &mut v,
            triangles: &mut t,
            source_faces: &mut s,
        },
        &|| false,
    )?;
    v.truncate(r.vertices);
    t.truncate(r.triangles);
    s.truncate(r.triangles);
    Ok((r, v, t, s))
}
#[test]
fn adequate_budget_matches_legacy_disjoint_geometry_and_source_faces() {
    let (r, v, t, s) = run(2., 1_000_000, 4 * 1024 * 1024, 64, 512).unwrap();
    let (a, ta) = cube(0.);
    let (b, tb) = cube(2.);
    let mut lv = vec![Point3::new(0., 0., 0.); 512];
    let mut lt = vec![[0; 3]; 512];
    let (nv, nt) = boolean_3_exact(&a, &ta, &b, &tb, Boolean3Op::Union, &mut lv, &mut lt).unwrap();
    assert_eq!((r.vertices, r.triangles), (nv, nt));
    assert_eq!(&v, &lv[..nv]);
    let mut sorted = t.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, lt[..nt]);
    for (index, ((t, s), original)) in t.iter().zip(&s).zip(ta.iter().chain(&tb)).enumerate() {
        assert!(s.mesh < 2);
        assert_eq!(s.face as usize, index % 12);
        let input = if s.mesh == 0 { &a } else { &b };
        assert_eq!(
            t.map(|i| v[i as usize]),
            original.map(|i| input[i as usize])
        );
    }
}
#[test]
fn work_byte_fragment_output_and_overflow_fail_explicitly() {
    assert!(matches!(
        run(2., 0, 4 * 1024 * 1024, 64, 512),
        Err(BoundedUnionError::Work)
    ));
    assert!(matches!(
        run(2., 1_000_000, 1, 64, 512),
        Err(BoundedUnionError::Bytes)
    ));
    assert!(matches!(
        run(2., 1_000_000, 4 * 1024 * 1024, 64, 1),
        Err(BoundedUnionError::Output)
    ));
    assert!(matches!(
        run(0.5, 1_000_000, 4 * 1024 * 1024, 1, 512),
        Err(BoundedUnionError::Fragments)
    ));
    assert_eq!(
        union_buffer_bytes(usize::MAX, 1, 1, 1, 1),
        Err(BoundedUnionError::Overflow)
    );
}
#[test]
fn invalid_input_leaves_public_output_unwritten() {
    let (mut a, ta) = cube(0.);
    let (b, tb) = cube(2.);
    a[0].x = f64::NAN;
    let z = Point3::new(7., 7., 7.);
    let (mut ca, mut cb) = ([[z; 3]; 12], [[z; 3]; 12]);
    let (mut c, mut n) = ([[z; 3]; 8], [[z; 3]; 8]);
    let (mut v, mut t, mut s) = ([z; 32], [[9; 3]; 32], [SourceFace { mesh: 9, face: 9 }; 32]);
    let result = union_exact_bounded_into(
        &a,
        &ta,
        &b,
        &tb,
        UnionBudget {
            bytes: 4 * 1024 * 1024,
            work: 100_000,
        },
        UnionBuffers {
            corners_a: &mut ca,
            corners_b: &mut cb,
            current: &mut c,
            next: &mut n,
            vertices: &mut v,
            triangles: &mut t,
            source_faces: &mut s,
        },
        &|| false,
    );
    assert!(matches!(result, Err(BoundedUnionError::InvalidInput)));
    assert_eq!(v, [z; 32]);
    assert_eq!(t, [[9; 3]; 32]);
}

#[test]
fn cancellation_is_observed_inside_first_face_pair_loop() {
    let (a, ta) = cube(0.);
    let (b, tb) = cube(0.5);
    let z = Point3::new(0., 0., 0.);
    let (mut ca, mut cb) = ([[z; 3]; 12], [[z; 3]; 12]);
    let (mut c, mut n) = ([[z; 3]; 64], [[z; 3]; 64]);
    let (mut v, mut t, mut s) = ([z; 512], [[0; 3]; 512], [SourceFace::default(); 512]);
    let calls = std::cell::Cell::new(0usize);
    let result = union_exact_bounded_into(
        &a,
        &ta,
        &b,
        &tb,
        UnionBudget {
            bytes: 4 * 1024 * 1024,
            work: 1_000_000,
        },
        UnionBuffers {
            corners_a: &mut ca,
            corners_b: &mut cb,
            current: &mut c,
            next: &mut n,
            vertices: &mut v,
            triangles: &mut t,
            source_faces: &mut s,
        },
        &|| {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        },
    );
    assert!(matches!(result, Err(BoundedUnionError::Cancelled)));
    assert_eq!(calls.get(), 3);
}
