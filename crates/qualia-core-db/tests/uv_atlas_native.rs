use qualia_core_db::specialized_libs::computational_geometry::{uv_atlas::*, Point3};
type Result<T> = core::result::Result<T, AtlasError>;
#[test]
fn non_power_of_two_atlas_delivery_stays_inside_reserved_gutter_with_subpixel_tolerance() {
    let (p, t) = disk();
    let mut o = options();
    o.atlas_size = 257;
    o.gutter_pixels = 3;
    let mut a = buffers(t.len());
    let n = run(
        &p,
        &t,
        &[[0, 4], [1, 4], [2, 4], [3, 4]],
        o,
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap();
    assert_eq!(n.charts, 4);
    for (face, t) in a.triangles.iter().enumerate() {
        let rect = a.charts[a.ids[face] as usize].rect_pixels;
        for &v in t {
            for axis in 0..2 {
                let pixel = a.uv[v as usize][axis] as f64 * o.atlas_size as f64;
                let lo = (rect[axis] + o.gutter_pixels) as f64;
                let hi = (rect[axis] + rect[axis + 2] - o.gutter_pixels) as f64;
                assert!(
                    pixel >= lo - 0.001 && pixel <= hi + 0.001,
                    "{pixel} outside [{lo},{hi}]"
                );
            }
        }
    }
}
#[test]
fn cancellation_at_last_probe_keeps_complete_caller_buffers_unchanged() {
    let (p, t) = disk();
    let count = std::cell::Cell::new(0usize);
    let mut accepted = buffers(t.len());
    run(
        &p,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| {
            count.set(count.get() + 1);
            false
        },
        &mut accepted,
    )
    .unwrap();
    let last = count.get();
    assert!(last > 10);
    let current = std::cell::Cell::new(0usize);
    let mut rejected = buffers(t.len());
    let original = buffers(t.len());
    let cancel = || {
        current.set(current.get() + 1);
        current.get() >= last
    };
    assert_eq!(
        run(
            &p,
            &t,
            &[],
            options(),
            AtlasLimits::default(),
            &cancel,
            &mut rejected
        ),
        Err(AtlasError::Cancelled)
    );
    assert_eq!(rejected, original);
}
fn options() -> AtlasOptions {
    AtlasOptions {
        atlas_size: 256,
        gutter_pixels: 4,
        texels_per_metre: 16.0,
        iterations: 64,
    }
}
#[derive(Debug, PartialEq)]
struct Owned {
    sources: Vec<u32>,
    triangles: Vec<[u32; 3]>,
    ids: Vec<u32>,
    uv: Vec<[f32; 2]>,
    charts: Vec<AtlasChart>,
}
fn buffers(f: usize) -> Owned {
    Owned {
        sources: vec![99; f * 3],
        triangles: vec![[99; 3]; f],
        ids: vec![99; f],
        uv: vec![[99.0; 2]; f * 3],
        charts: vec![AtlasChart::default(); f],
    }
}
fn run(
    p: &[Point3],
    t: &[[u32; 3]],
    s: &[[u32; 2]],
    o: AtlasOptions,
    l: AtlasLimits,
    c: &dyn Fn() -> bool,
    b: &mut Owned,
) -> Result<AtlasCounts> {
    unwrap_atlas_into(
        p,
        t,
        s,
        o,
        l,
        c,
        AtlasOutput {
            source_vertices: &mut b.sources,
            triangles: &mut b.triangles,
            triangle_charts: &mut b.ids,
            uv0: &mut b.uv,
            charts: &mut b.charts,
        },
    )
}
fn disk() -> (Vec<Point3>, Vec<[u32; 3]>) {
    (
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(0.5, 0.5, 0.4),
        ],
        vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
    )
}
#[test]
fn curved_disk_has_positive_coverage_density_and_repeatable_uvs() {
    let (p, t) = disk();
    let mut a = buffers(t.len());
    let mut b = buffers(t.len());
    let n = run(
        &p,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap();
    assert_eq!(
        n,
        run(
            &p,
            &t,
            &[],
            options(),
            AtlasLimits::default(),
            &|| false,
            &mut b
        )
        .unwrap()
    );
    assert_eq!(a, b);
    assert_eq!(n.charts, 1);
    assert_eq!(n.vertices, 5);
    assert!(a.charts[0].uv_area_pixels > 0.0);
    assert!((a.charts[0].uv_area_pixels / a.charts[0].surface_area - 256.0).abs() < 0.001);
    for u in &a.uv[..n.vertices] {
        assert!(u
            .iter()
            .all(|&v| v >= 4.0 / 256.0 && v <= 1.0 - 4.0 / 256.0));
    }
    assert_eq!(
        a.triangles
            .iter()
            .map(|t| t.map(|v| a.sources[v as usize]))
            .collect::<Vec<_>>(),
        t
    );
}
#[test]
fn explicit_diagonal_seam_duplicates_corners_and_gutters_do_not_overlap() {
    let (p, t) = disk();
    let seams = [[0, 4], [1, 4], [2, 4], [3, 4]];
    let mut a = buffers(t.len());
    let n = run(
        &p,
        &t,
        &seams,
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap();
    assert_eq!(n.charts, 4);
    assert_eq!(n.vertices, 12);
    for i in 0..n.charts {
        for j in i + 1..n.charts {
            let r = a.charts[i].rect_pixels;
            let s = a.charts[j].rect_pixels;
            assert!(
                r[0] + r[2] <= s[0]
                    || s[0] + s[2] <= r[0]
                    || r[1] + r[3] <= s[1]
                    || s[1] + s[3] <= r[1]
            );
        }
    }
}
fn cylinder() -> (Vec<Point3>, Vec<[u32; 3]>) {
    let mut p = Vec::new();
    for z in [0.0, 1.0] {
        for [x, y] in [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]] {
            p.push(Point3::new(x, y, z));
        }
    }
    let mut t = Vec::new();
    for i in 0..4u32 {
        let j = (i + 1) % 4;
        t.push([i, j, j + 4]);
        t.push([i, j + 4, i + 4]);
    }
    (p, t)
}
#[test]
fn nonseparating_cylinder_cut_splits_vertices_within_one_chart() {
    let (p, t) = cylinder();
    let mut a = buffers(t.len());
    assert!(run(
        &p,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    let n = run(
        &p,
        &t,
        &[[0, 4]],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap();
    assert_eq!(n.charts, 1);
    assert_eq!(n.vertices, 10);
    assert_eq!(
        a.sources[..n.vertices].iter().filter(|&&v| v == 0).count(),
        2
    );
    assert_eq!(
        a.sources[..n.vertices].iter().filter(|&&v| v == 4).count(),
        2
    );
}
#[test]
fn bad_topology_seams_budget_density_and_cancel_preserve_all_outputs() {
    let (p, t) = disk();
    let mut a = buffers(t.len());
    let original = buffers(t.len());
    for seams in [vec![[4, 0]], vec![[0, 4], [0, 4]], vec![[0, 2]]] {
        assert!(run(
            &p,
            &t,
            &seams,
            options(),
            AtlasLimits::default(),
            &|| false,
            &mut a
        )
        .is_err());
        assert_eq!(a, original);
    }
    let mut bad = t.clone();
    bad[1] = [1, 4, 2];
    assert!(run(
        &p,
        &bad,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    assert_eq!(a, original);
    let mut l = AtlasLimits::default();
    l.scratch_bytes = 1;
    assert_eq!(
        run(&p, &t, &[], options(), l, &|| false, &mut a),
        Err(AtlasError::Budget)
    );
    assert_eq!(a, original);
    let mut o = options();
    o.texels_per_metre = 1e6;
    assert!(run(&p, &t, &[], o, AtlasLimits::default(), &|| false, &mut a).is_err());
    assert_eq!(a, original);
    let probes = std::cell::Cell::new(0);
    let cancelled = || {
        probes.set(probes.get() + 1);
        probes.get() > 8
    };
    assert_eq!(
        run(
            &p,
            &t,
            &[],
            options(),
            AtlasLimits::default(),
            &cancelled,
            &mut a
        ),
        Err(AtlasError::Cancelled)
    );
    assert_eq!(a, original);
}
#[test]
fn closed_tetrahedron_requires_explicit_cuts_and_keeps_face_order() {
    let p = [
        Point3::new(0., 0., 0.),
        Point3::new(1., 0., 0.),
        Point3::new(0., 1., 0.),
        Point3::new(0., 0., 1.),
    ];
    let t = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let mut a = buffers(t.len());
    let initial = buffers(t.len());
    assert!(run(
        &p,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    assert_eq!(a, initial);
    let n = run(
        &p,
        &t,
        &[[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap();
    assert_eq!(n.charts, 4);
    assert_eq!(n.vertices, 12);
    assert_eq!(
        a.triangles
            .iter()
            .map(|t| t.map(|v| a.sources[v as usize]))
            .collect::<Vec<_>>(),
        t
    );
}
#[test]
fn disconnected_islands_pack_as_distinct_charts_and_validate_output_capacity() {
    let p = [
        Point3::new(0., 0., 0.),
        Point3::new(1., 0., 0.),
        Point3::new(0., 1., 0.),
        Point3::new(2., 0., 0.),
        Point3::new(3., 0., 0.),
        Point3::new(2., 1., 0.),
    ];
    let t = [[0, 1, 2], [3, 4, 5]];
    let mut a = buffers(t.len());
    let n = run(
        &p,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap();
    assert_eq!(n.charts, 2);
    assert_ne!(a.ids[0], a.ids[1]);
    let mut small = buffers(1);
    let original = buffers(1);
    assert_eq!(
        run(
            &p,
            &t,
            &[],
            options(),
            AtlasLimits::default(),
            &|| false,
            &mut small
        ),
        Err(AtlasError::OutputTooSmall)
    );
    assert_eq!(small, original);
}
#[test]
fn f32_delivery_collapse_refuses_instead_of_promoting_invalid_uvs() {
    let p = [
        Point3::new(0., 0., 0.),
        Point3::new(1., 0., 0.),
        Point3::new(1., 1e-10, 0.),
    ];
    let t = [[0, 1, 2]];
    let mut a = buffers(1);
    let original = buffers(1);
    let err = run(
        &p,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a,
    )
    .unwrap_err();
    assert!(
        matches!(err,AtlasError::Invalid(message) if message.contains("f32")),
        "{err:?}"
    );
    assert_eq!(a, original);
}
#[test]
fn malformed_indices_nonfinite_degenerate_and_nonmanifold_edges_refuse_atomically() {
    let (p, t) = disk();
    let mut a = buffers(5);
    let original = buffers(5);
    let mut bad = t.clone();
    bad[0][0] = 99;
    assert!(run(
        &p,
        &bad,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    assert_eq!(a, original);
    let mut bad = t.clone();
    bad[0] = [0, 0, 4];
    assert!(run(
        &p,
        &bad,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    assert_eq!(a, original);
    let mut bad = t.clone();
    bad.push(t[0]);
    assert!(run(
        &p,
        &bad,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    assert_eq!(a, original);
    let mut bad = p.clone();
    bad[0].x = f64::NAN;
    assert!(run(
        &bad,
        &t,
        &[],
        options(),
        AtlasLimits::default(),
        &|| false,
        &mut a
    )
    .is_err());
    assert_eq!(a, original);
}
