//! Geometric checks independent of deterministic byte equality.
use super::{torus, uv_sphere};
use crate::render::assets::Mesh;
use std::collections::BTreeMap;
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn measures(mesh: &Mesh) -> (f64, f64) {
    let (mut area, mut volume) = (0.0, 0.0);
    for t in &mesh.triangles {
        let p = t.map(|i| mesh.positions[i as usize].map(f64::from));
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        area += dot(n, n).sqrt() * 0.5;
        volume += dot(p[0], cross(p[1], p[2])) / 6.0;
    }
    (area, volume)
}
fn manifold(mesh: &Mesh, euler: i64) {
    // The sphere deliberately duplicates seam vertices for later UV assignment.
    let mut welded = BTreeMap::new();
    let mut ids = Vec::new();
    for p in &mesh.positions {
        let key = p.map(|x| (x as f64 * 1e6).round() as i64);
        let next = welded.len();
        ids.push(*welded.entry(key).or_insert(next));
    }
    let mut edges: BTreeMap<(usize, usize), (usize, i64)> = BTreeMap::new();
    for t in &mesh.triangles {
        let v = t.map(|i| ids[i as usize]);
        assert!(v[0] != v[1] && v[1] != v[2] && v[2] != v[0]);
        for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
            let key = (a.min(b), a.max(b));
            let e = edges.entry(key).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|e| *e == (2, 0)));
    assert_eq!(
        welded.len() as i64 - edges.len() as i64 + mesh.triangles.len() as i64,
        euler
    );
}
#[test]
fn sphere_has_local_outward_faces_correct_measure_and_closed_surface() {
    let radius = 1.0;
    let mesh = uv_sphere(radius, 24, 48).unwrap();
    manifold(&mesh, 2);
    for p in &mesh.positions {
        assert!((p.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt() - 1.0).abs() < 2e-6);
    }
    for t in &mesh.triangles {
        let p = t.map(|i| mesh.positions[i as usize].map(f64::from));
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let center = std::array::from_fn(|i| (p[0][i] + p[1][i] + p[2][i]) / 3.0);
        assert!(dot(n, center) > 0.0);
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let d = sub(p[a], p[b]);
            assert!(dot(d, d).sqrt() < 0.4, "nonlocal sphere edge");
        }
    }
    let (area, volume) = measures(&mesh);
    assert!((area / (4.0 * std::f64::consts::PI) - 1.0).abs() < 0.02);
    assert!((volume / (4.0 * std::f64::consts::PI / 3.0) - 1.0).abs() < 0.02);
    assert!(uv_sphere(f32::NAN, 8, 16).is_err());
}
#[test]
fn torus_faces_are_outward_and_form_the_expected_closed_genus() {
    let mesh = torus(1.0, 0.25, 48, 24).unwrap();
    manifold(&mesh, 0);
    for t in &mesh.triangles {
        let p = t.map(|i| mesh.positions[i as usize].map(f64::from));
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let c = std::array::from_fn(|i| (p[0][i] + p[1][i] + p[2][i]) / 3.0);
        let radial = (c[0] * c[0] + c[2] * c[2]).sqrt();
        let centerline = [c[0] / radial, 0.0, c[2] / radial];
        assert!(dot(n, sub(c, centerline)) > 0.0);
    }
    let (area, volume) = measures(&mesh);
    let pi = std::f64::consts::PI;
    assert!((area / (4.0 * pi * pi * 0.25) - 1.0).abs() < 0.03);
    assert!((volume / (2.0 * pi * pi * 0.25 * 0.25) - 1.0).abs() < 0.03);
    assert!(torus(1.0, f32::NAN, 16, 8).is_err());
}
