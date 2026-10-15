use super::super::allocation_counter::assert_zero_alloc;
use super::super::botanical_hulls::{asymmetric_hull_into, AsymmetricHullParams};
use super::super::canopy_radiation::{
    integrate_canopy_radiation, organ_surface_areas, periodize_id_depth_buffer,
    rasterize_orthographic_id_buffer, soc_sky_dome_16, solar_track_sources, DirectionalSource,
};
use super::super::curve_skeleton::{contract_to_skeleton, extract_skeleton_graph};
use super::super::differential_growth::{differential_growth_step, edge_rest_lengths};
use super::super::dual_quaternion_skinning::{dqs_skin_vertex, DualQuaternion};
use super::super::incircle::incircle;
use super::super::insphere::insphere;
use super::super::math_geometry::{quaternion_normalize, AffineFrame3, Quaternion};
use super::super::orient3d::orient_3d;
use super::super::parametric_cad::SdfOp;
use super::super::pbd_softbody::{pbd_step, PbdConstraint};
use super::super::poisson_disk::{poisson_disk_2d, poisson_disk_3d};
use super::super::primitives::{orientation_2, Point2, Point3};
use super::super::sdf_ops::{
    sdf_capsule, sdf_compose_smooth, sdf_metaballs, sdf_rounded_box, sdf_sphere,
};
use super::super::sweep_extrusion::{
    compute_bishop_frames, extrude_profile_along_path, frustum_into,
};
use super::super::turtle_3d::{execute_turtle_3d, TurtleCmd, TurtleState};

#[test]
fn orientation_2_hot_path_is_zero_heap() {
    // Warm up (in case of lazy init).
    let _ = orientation_2(
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(0.5, 0.5),
    );
    assert_zero_alloc("orientation_2", || {
        let _ = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 0.5),
        );
        let _ = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
        );
        let _ = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, -1.0),
        );
    });
}

#[test]
fn orient_3d_hot_path_is_zero_heap() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(1.0, 0.0, 0.0);
    let c = Point3::new(0.0, 1.0, 0.0);
    let d = Point3::new(0.0, 0.0, 1.0);
    let _ = orient_3d(a, b, c, d); // warm up
    assert_zero_alloc("orient_3d", || {
        let _ = orient_3d(a, b, c, d);
        let _ = orient_3d(a, b, c, Point3::new(0.0, 0.0, -1.0));
    });
}

#[test]
fn incircle_hot_path_is_zero_heap() {
    let a = Point2::new(0.0, 0.0);
    let b = Point2::new(1.0, 0.0);
    let c = Point2::new(0.0, 1.0);
    let d = Point2::new(0.25, 0.25);
    let _ = incircle(a, b, c, d); // warm up
    assert_zero_alloc("incircle", || {
        let _ = incircle(a, b, c, d);
        let _ = incircle(a, b, c, Point2::new(2.0, 2.0));
    });
}

#[test]
fn insphere_hot_path_is_zero_heap() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(1.0, 0.0, 0.0);
    let c = Point3::new(0.0, 1.0, 0.0);
    let d = Point3::new(0.0, 0.0, 1.0);
    let e = Point3::new(0.25, 0.25, 0.25);
    let _ = insphere(a, b, c, d, e); // warm up
    assert_zero_alloc("insphere", || {
        let _ = insphere(a, b, c, d, e);
        let _ = insphere(a, b, c, d, Point3::new(2.0, 2.0, 2.0));
    });
}

#[test]
fn canopy_radiation_ops_are_zero_heap() {
    let verts = [
        Point3::new(-1.0, -1.0, 1.0),
        Point3::new(1.0, -1.0, 1.0),
        Point3::new(1.0, 1.0, 1.0),
        Point3::new(-1.0, 1.0, 1.0),
    ];
    let tris = [[0u32, 1, 2], [0, 2, 3]];
    let oids = [0u32; 2];
    let mut areas = [0.0f64; 1];
    let _ = organ_surface_areas(&verts, &tris, &oids, &mut areas);
    let mut depth = [0.0f64; 8 * 8];
    let mut ids = [0u32; 8 * 8];
    let mut ds = [0.0f64; 8 * 8];
    let mut is_ = [0u32; 8 * 8];
    let mut sil = [0.0f64; 1];
    let mut par = [0.0f64; 1];
    let mut star = [0.0f64; 1];
    let mut sky = [DirectionalSource {
        direction: Point3::default(),
        weight: 0.0,
    }; 16];
    let _ = soc_sky_dome_16(1.0, &mut sky);
    let mut track = [DirectionalSource {
        direction: Point3::default(),
        weight: 0.0,
    }; 48];
    let _ = solar_track_sources(45.0, 172, 48, 100.0, &mut track);
    let _ = rasterize_orthographic_id_buffer(
        &verts,
        &tris,
        &oids,
        Point3::new(0.0, 0.0, -1.0),
        8,
        8,
        &mut depth,
        &mut ids,
    );
    let _ = periodize_id_depth_buffer(8, 8, 4, 0, 0.0, &mut depth, &mut ids, &mut ds, &mut is_);
    let _ = integrate_canopy_radiation(
        &verts, &tris, &oids, &sky, 8, 8, &mut depth, &mut ids, &mut sil, &areas, 4.0, &mut par,
        &mut star,
    );
    assert_zero_alloc("canopy_radiation", || {
        let _ = organ_surface_areas(&verts, &tris, &oids, &mut areas);
        let _ = soc_sky_dome_16(1.0, &mut sky);
        let _ = solar_track_sources(45.0, 172, 48, 100.0, &mut track);
        let _ = rasterize_orthographic_id_buffer(
            &verts,
            &tris,
            &oids,
            Point3::new(0.0, 0.0, -1.0),
            8,
            8,
            &mut depth,
            &mut ids,
        );
        let _ = periodize_id_depth_buffer(8, 8, 4, 0, 0.0, &mut depth, &mut ids, &mut ds, &mut is_);
        let _ = integrate_canopy_radiation(
            &verts, &tris, &oids, &sky, 8, 8, &mut depth, &mut ids, &mut sil, &areas, 4.0,
            &mut par, &mut star,
        );
    });
}

#[test]
fn sweep_extrusion_ops_are_zero_heap() {
    let axis = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(0.5, 0.0, 2.0),
    ];
    let section = [
        Point2::new(0.1, 0.0),
        Point2::new(0.0, 0.1),
        Point2::new(-0.1, 0.0),
        Point2::new(0.0, -0.1),
    ];
    let mut frames = [AffineFrame3 {
        origin: Point3::default(),
        e0: Point3::default(),
        e1: Point3::default(),
        e2: Point3::default(),
    }; 3];
    let mut verts = [Point3::default(); 64];
    let mut tris = [[0u32; 3]; 64];
    assert_zero_alloc("sweep_extrusion", || {
        let _ = compute_bishop_frames(&axis, None, &mut frames);
        let _ = extrude_profile_along_path(
            &axis,
            &section,
            None,
            true,
            true,
            &mut frames,
            &mut verts,
            &mut tris,
        );
        let _ = frustum_into(1.0, 0.5, 2.0, 8, &mut verts, &mut tris);
    });
}

#[test]
fn botanical_hulls_ops_are_zero_heap() {
    let p = AsymmetricHullParams::default();
    let (cap_v, cap_t) =
        super::super::botanical_hulls::asymmetric_hull_capacity(p.slices, p.stacks);
    let mut verts = vec![Point3::default(); cap_v];
    let mut tris = vec![[0u32; 3]; cap_t];
    assert_zero_alloc("botanical_hulls", || {
        let _ = asymmetric_hull_into(&p, &mut verts, &mut tris);
    });
}

#[test]
fn turtle_3d_ops_are_zero_heap() {
    let cmds = [
        TurtleCmd::ForwardDraw {
            length: 1.0,
            top_width: None,
        },
        TurtleCmd::TurnLeft(45.0),
        TurtleCmd::ForwardDraw {
            length: 1.0,
            top_width: Some(0.1),
        },
    ];
    let mut verts = [Point3::default(); 256];
    let mut tris = [[0u32; 3]; 256];
    assert_zero_alloc("turtle_3d", || {
        let _ = execute_turtle_3d(&cmds, TurtleState::default(), 8, &mut verts, &mut tris);
    });
}

#[test]
fn poisson_disk_ops_are_zero_heap() {
    let mut grid = [u32::MAX; 64 * 64];
    let mut active = [0u32; 256];
    let mut out2 = [Point2::default(); 256];
    let mut grid3 = [u32::MAX; 16 * 16 * 16];
    let mut active3 = [0u32; 64];
    let mut out3 = [Point3::default(); 64];
    assert_zero_alloc("poisson_disk", || {
        let _ = poisson_disk_2d(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 4.0),
            0.5,
            42,
            30,
            &mut grid,
            &mut active,
            &mut out2,
        );
        let _ = poisson_disk_3d(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(4.0, 4.0, 4.0),
            0.8,
            42,
            30,
            &mut grid3,
            &mut active3,
            &mut out3,
        );
    });
}

#[test]
fn differential_growth_ops_are_zero_heap() {
    let mut verts = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.5, 1.0, 0.0),
    ];
    let edges = [(0u32, 1u32), (1, 2), (2, 0)];
    let growth = [1.0, 0.5, 0.0];
    let mut rest = [0.0f64; 3];
    assert_zero_alloc("differential_growth", || {
        let _ = edge_rest_lengths(&verts, &edges, &mut rest);
        let _ = differential_growth_step(&mut verts, &edges, &growth, &mut rest, 0.5, 0.1);
    });
}

#[test]
fn curve_skeleton_ops_are_zero_heap() {
    let verts = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.5, 0.0, 1.0),
        Point3::new(0.5, 1.0, 0.5),
    ];
    let tris = [[0u32, 1, 3], [1, 2, 3], [2, 0, 3]];
    let mut sum = [Point3::default(); 4];
    let mut cnt = [0u32; 4];
    let mut work = [Point3::default(); 4];
    let mut nov = [0u32; 4];
    let mut nodes = [Point3::default(); 4];
    let mut parents = [0u32; 4];
    let mut queue = [0u32; 4];
    assert_zero_alloc("curve_skeleton", || {
        let _ = contract_to_skeleton(&verts, &tris, 3, 0.2, &mut sum, &mut cnt, &mut work);
        let _ = extract_skeleton_graph(
            &work,
            &tris,
            0.3,
            &mut nov,
            &mut nodes,
            &mut parents,
            &mut queue,
        );
    });
}

#[test]
fn dual_quaternion_skinning_ops_are_zero_heap() {
    let q = quaternion_normalize(Quaternion {
        w: 0.9238795,
        x: 0.3826834,
        y: 0.0,
        z: 0.0,
    });
    let bones = [
        DualQuaternion::IDENTITY,
        DualQuaternion::from_rotation_translation(q, Point3::new(0.0, 0.0, 1.0)),
    ];
    let weights = [0.5, 0.5];
    let p = Point3::new(0.0, 1.0, 0.0);
    assert_zero_alloc("dual_quaternion_skinning", || {
        let _ = dqs_skin_vertex(p, &bones, &weights);
        let _ = DualQuaternion::sclerp(bones[0], bones[1], 0.3);
    });
}

#[test]
fn pbd_softbody_ops_are_zero_heap() {
    let mut pos = [Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)];
    let mut prev = pos;
    let inv = [1.0, 1.0];
    let cons = [PbdConstraint::Distance {
        a: 0,
        b: 1,
        rest: 1.0,
    }];
    assert_zero_alloc("pbd_softbody", || {
        let _ = pbd_step(
            &mut pos,
            &mut prev,
            &inv,
            &cons,
            Point3::new(0.0, 0.0, -9.8),
            0.0,
            0.016,
            4,
            f64::NAN,
        );
    });
}

#[test]
fn sdf_ops_are_zero_heap() {
    let p = Point3::new(0.3, 0.4, 0.1);
    let centers = [Point3::new(0.0, 0.0, 0.0), Point3::new(0.5, 0.0, 0.0)];
    let radii2 = [1.0, 1.0];
    assert_zero_alloc("sdf_ops", || {
        let a = sdf_sphere(p, Point3::default(), 1.0);
        let b = sdf_capsule(
            p,
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            0.5,
        );
        let _ = sdf_compose_smooth(a, b, SdfOp::Union, 0.25);
        let _ = sdf_compose_smooth(a, b, SdfOp::Difference, 0.25);
        let _ = sdf_rounded_box(p, Point3::default(), Point3::new(1.0, 1.0, 1.0), 0.1);
        let _ = sdf_metaballs(p, &centers, &radii2, 1.0);
    });
}
