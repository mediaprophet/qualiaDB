use super::*;

#[test]
fn malformed_pbd_edge_is_rejected_without_panicking() {
    let args = args::record([
        ("positions", args::f64_list_value(vec![0.0, 0.0, 0.0])),
        (
            "edges",
            Value::List(vec![args::f64_list_value(vec![1.0, 0.0])]),
        ),
        ("dt", Value::F64(0.01)),
    ]);
    assert!(pbd_step_host(&args, Span::new(0, 0)).is_err());
}

#[test]
fn fractional_triangle_index_is_rejected() {
    let triangles = args::f64_list_value(vec![0.0, 1.5, 2.0]);
    assert!(tri3(&triangles, Span::new(0, 0), "triangles").is_err());
}

#[test]
fn tiny_poisson_radius_is_rejected_before_allocation() {
    let args = args::record([
        ("min", args::f64_list_value(vec![0.0, 0.0, 0.0])),
        ("max", args::f64_list_value(vec![1.0, 1.0, 1.0])),
        ("radius", Value::F64(1e-100)),
    ]);
    assert!(poisson_disk_3d_host(&args, Span::new(0, 0)).is_err());
}

#[cfg(target_pointer_width = "64")]
#[test]
fn poisson_host_accepts_high_detail_budget_without_reserving_it_up_front() {
    let args = args::record([
        ("min", args::f64_list_value(vec![0.0, 0.0])),
        ("max", args::f64_list_value(vec![1.0, 1.0])),
        ("radius", Value::F64(2.0)),
        ("grid_budget_bytes", Value::U64(3 * 1024 * 1024 * 1024)),
    ]);
    assert!(poisson_disk_2d_host(&args, Span::new(0, 0)).is_ok());
}
