//! Numeric line and surface integrals over parametric curves/surfaces given as
//! closures. Derivatives of the parametrization are taken by central finite difference,
//! so the caller supplies only the position map. Trapezoidal quadrature.

const FD: f64 = 1e-6;

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn norm(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}
fn cross3(a: &[f64], b: &[f64]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Central-difference derivative of a curve `r(t)`.
fn dcurve<C: Fn(f64) -> Vec<f64>>(curve: &C, t: f64) -> Vec<f64> {
    let p = curve(t + FD);
    let m = curve(t - FD);
    p.iter().zip(&m).map(|(a, b)| (a - b) / (2.0 * FD)).collect()
}

/// Scalar line integral `∫_C f ds = ∫ f(r(t)) |r'(t)| dt`.
pub fn line_integral_scalar<F, C>(f: F, curve: C, t0: f64, t1: f64, steps: usize) -> f64
where
    F: Fn(&[f64]) -> f64,
    C: Fn(f64) -> Vec<f64>,
{
    let h = (t1 - t0) / steps as f64;
    let g = |t: f64| f(&curve(t)) * norm(&dcurve(&curve, t));
    let mut sum = 0.5 * (g(t0) + g(t1));
    for i in 1..steps {
        sum += g(t0 + i as f64 * h);
    }
    sum * h
}

/// Work / vector line integral `∫_C F·dr = ∫ F(r(t))·r'(t) dt`.
pub fn line_integral_work<F, C>(field: F, curve: C, t0: f64, t1: f64, steps: usize) -> f64
where
    F: Fn(&[f64]) -> Vec<f64>,
    C: Fn(f64) -> Vec<f64>,
{
    let h = (t1 - t0) / steps as f64;
    let g = |t: f64| dot(&field(&curve(t)), &dcurve(&curve, t));
    let mut sum = 0.5 * (g(t0) + g(t1));
    for i in 1..steps {
        sum += g(t0 + i as f64 * h);
    }
    sum * h
}

/// Flux of a 3-D field through a parametric surface `r(u,v)`:
/// `∫∫ F·(r_u × r_v) du dv`. The orientation is that of `r_u × r_v`.
pub fn surface_flux<F, S>(
    field: F,
    surf: S,
    u0: f64,
    u1: f64,
    v0: f64,
    v1: f64,
    steps: usize,
) -> f64
where
    F: Fn(&[f64]) -> Vec<f64>,
    S: Fn(f64, f64) -> Vec<f64>,
{
    let hu = (u1 - u0) / steps as f64;
    let hv = (v1 - v0) / steps as f64;
    let integrand = |u: f64, v: f64| -> f64 {
        let ru: Vec<f64> = {
            let p = surf(u + FD, v);
            let m = surf(u - FD, v);
            p.iter().zip(&m).map(|(a, b)| (a - b) / (2.0 * FD)).collect()
        };
        let rv: Vec<f64> = {
            let p = surf(u, v + FD);
            let m = surf(u, v - FD);
            p.iter().zip(&m).map(|(a, b)| (a - b) / (2.0 * FD)).collect()
        };
        let n = cross3(&ru, &rv);
        dot(&field(&surf(u, v)), &n)
    };
    // Trapezoidal over the 2-D grid.
    let mut sum = 0.0;
    for i in 0..=steps {
        for j in 0..=steps {
            let w = {
                let wu = if i == 0 || i == steps { 0.5 } else { 1.0 };
                let wv = if j == 0 || j == steps { 0.5 } else { 1.0 };
                wu * wv
            };
            sum += w * integrand(u0 + i as f64 * hu, v0 + j as f64 * hv);
        }
    }
    sum * hu * hv
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::PI;

    #[test]
    fn conservative_work_is_potential_difference() {
        // F = (y, x) = ∇(xy). Work from (0,0)→(1,1) along the diagonal = xy|=1.
        let f = |p: &[f64]| vec![p[1], p[0]];
        let curve = |t: f64| vec![t, t];
        let w = line_integral_work(f, curve, 0.0, 1.0, 400);
        assert!((w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn greens_theorem_on_the_unit_circle() {
        // ∮ F·dr with F = (−y, x) around the unit circle = 2·area = 2π.
        let f = |p: &[f64]| vec![-p[1], p[0]];
        let circle = |t: f64| vec![t.cos(), t.sin()];
        let w = line_integral_work(f, circle, 0.0, 2.0 * PI, 2000);
        assert!((w - 2.0 * PI).abs() < 1e-4);
    }

    #[test]
    fn arc_length_via_scalar_line_integral() {
        // ∫_C 1 ds over the unit circle = circumference = 2π.
        let len = line_integral_scalar(|_| 1.0, |t: f64| vec![t.cos(), t.sin()], 0.0, 2.0 * PI, 2000);
        assert!((len - 2.0 * PI).abs() < 1e-4);
    }

    #[test]
    fn divergence_theorem_flux_through_sphere() {
        // Flux of F = (x,y,z) through the unit sphere = ∫∫∫ div F dV = 3·(4/3 π) = 4π.
        // Parametrize with polar angle u∈[0,π] first, azimuth v∈[0,2π] second, so
        // r_u × r_v is the *outward* normal (the divergence theorem's orientation).
        let f = |p: &[f64]| vec![p[0], p[1], p[2]];
        let sphere = |u: f64, v: f64| vec![u.sin() * v.cos(), u.sin() * v.sin(), u.cos()];
        let flux = surface_flux(f, sphere, 0.0, PI, 0.0, 2.0 * PI, 120);
        assert!((flux - 4.0 * PI).abs() < 1e-2, "flux {flux} vs 4π");
    }
}
