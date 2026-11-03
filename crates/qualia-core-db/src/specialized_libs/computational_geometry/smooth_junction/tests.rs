use super::*;
fn controls() -> Controls {
    Controls {
        blend: 0.1,
        tolerance: 1e-8,
        gradient_step: 1e-5,
        max_step: 0.1,
        max_displacement: 0.5,
        iterations: 32,
        refinements: 1,
        max_dihedral: 1.5,
        min_normal_alignment: 0.5,
        min_area_sq: 1e-20,
    }
}
fn fixture() -> (
    Vec<Point3>,
    Vec<[u32; 3]>,
    Vec<FaceSource>,
    [TaperedFieldSegment; 2],
) {
    let p = vec![
        Point3::new(0.8, 0., 0.),
        Point3::new(-0.8, 0., 0.),
        Point3::new(0., 0.8, 0.),
        Point3::new(0., -0.8, 0.),
        Point3::new(0., 0., 0.8),
        Point3::new(0., 0., -0.8),
    ];
    let t = vec![
        [0, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [2, 0, 5],
        [1, 2, 5],
        [3, 1, 5],
        [0, 3, 5],
    ];
    let s = (0..8)
        .map(|i| FaceSource {
            piece: (i / 4) as u32,
            face: i as u32,
        })
        .collect();
    let segment = TaperedFieldSegment {
        a: Point3::new(0., -0.001, 0.),
        b: Point3::new(0., 0.001, 0.),
        radius_a: 1.,
        radius_b: 1.,
    };
    (p, t, s, [segment; 2])
}
#[test]
fn positive_field_projection_moves_geometry_and_retains_source_faces() {
    let (p, t, s, segments) = fixture();
    // Refined octahedron face centers start at radius 0.8/sqrt(3), while
    // the duplicate positive-blend capsule surface is near radius 1.025.
    // Its approximately 0.563 displacement exceeds the former 0.5 fixture
    // allowance. This changes this positive fixture only, not production.
    let mut positive_controls = controls();
    positive_controls.max_displacement = 0.75;
    let zero = Point3::new(0., 0., 0.);
    let mut outp = vec![zero; 256];
    let mut refs = outp.clone();
    let mut normals = outp.clone();
    let mut weights = vec![0.; 256];
    let mut outt = vec![[0; 3]; 512];
    let mut outs = vec![FaceSource::default(); 512];
    let limits = Limits {
        vertices: 256,
        faces: 512,
        bytes: 8 * 1024 * 1024,
        work: 1_000_000,
    };
    let r = smooth_into(
        Input {
            positions: &p,
            triangles: &t,
            sources: &s,
            segments: &segments,
            fade_coordinates: &[1.; 6],
        },
        positive_controls,
        limits,
        Output {
            positions: &mut outp,
            references: &mut refs,
            triangles: &mut outt,
            sources: &mut outs,
            normals: &mut normals,
            weights: &mut weights,
        },
        &|| false,
    )
    .unwrap();
    assert!(r.moved_vertices > 0);
    assert!(r.faces > t.len());
    assert!(r.max_field_residual <= controls().tolerance);
    assert!(r.max_dihedral <= controls().max_dihedral);
    assert!(r.min_normal_alignment >= controls().min_normal_alignment);
    assert!(outs[..r.faces].iter().all(|f| s.contains(f)));
    assert_ne!(&outp[..p.len()], p.as_slice());
}
#[test]
fn interface_in_fixed_band_and_invalid_budget_never_publish() {
    let (p, t, s, segments) = fixture();
    let sentinel = Point3::new(9., 9., 9.);
    let mut outp = vec![sentinel; 256];
    let mut refs = outp.clone();
    let mut normals = outp.clone();
    let mut weights = vec![7.; 256];
    let mut outt = vec![[9; 3]; 512];
    let mut outs = vec![FaceSource::default(); 512];
    for bytes in [1, 8 * 1024 * 1024] {
        let result = smooth_into(
            Input {
                positions: &p,
                triangles: &t,
                sources: &s,
                segments: &segments,
                fade_coordinates: &[0.; 6],
            },
            controls(),
            Limits {
                vertices: 256,
                faces: 512,
                bytes,
                work: 1_000_000,
            },
            Output {
                positions: &mut outp,
                references: &mut refs,
                triangles: &mut outt,
                sources: &mut outs,
                normals: &mut normals,
                weights: &mut weights,
            },
            &|| false,
        );
        assert_eq!(
            result.unwrap_err(),
            if bytes == 1 {
                Error::Bytes
            } else {
                Error::BoundaryContact
            }
        );
        assert!(outp.iter().all(|&p| p == sentinel));
        assert!(outt.iter().all(|&t| t == [9; 3]));
    }
    let mut c = controls();
    c.blend = 0.;
    assert_eq!(validate_controls(c), Err(Error::Invalid));
    assert_eq!(
        storage_bytes(Limits {
            vertices: usize::MAX,
            faces: 512,
            bytes: usize::MAX,
            work: usize::MAX
        }),
        Err(Error::Overflow)
    );
}
