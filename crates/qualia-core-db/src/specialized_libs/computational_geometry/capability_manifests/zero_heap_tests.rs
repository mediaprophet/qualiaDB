use super::super::allocation_counter::assert_zero_alloc;
use super::super::incircle::incircle;
use super::super::insphere::insphere;
use super::super::orient3d::orient_3d;
use super::super::primitives::{orientation_2, Point2, Point3};

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
