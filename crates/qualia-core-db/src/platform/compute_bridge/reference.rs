//! CPU reference microkernels — one per [`KernelClass`](super::kernel_class::KernelClass).
//!
//! These serve two roles at once (plan §3 + §5 step 4):
//! 1. **The panel's CPU measurement** — each is the representative microkernel timed
//!    to give the CPU's per-class throughput row.
//! 2. **The correctness reference** — any GPU/NPU/vendor kernel for a class must
//!    match its CPU reference within the class tolerance before it may be the
//!    default. "A faster wrong answer is a regression."
//!
//! They are plain, correct, scalar/`rayon` implementations over caller-owned slices.
//! The CPU path is always present and never hard-fails (plan §7).

use rayon::prelude::*;

/// `DenseLinear`: GEMV `y = W·x`, `W` row-major `n×n`. `y.len()==n`, `x.len()==n`.
pub fn gemv(w: &[f32], x: &[f32], y: &mut [f32]) {
    let n = x.len();
    debug_assert_eq!(w.len(), n * n);
    debug_assert_eq!(y.len(), n);
    y.par_iter_mut().enumerate().for_each(|(i, o)| {
        let row = &w[i * n..(i + 1) * n];
        *o = row.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
    });
}

/// `ElementwiseMap`: fused `y = a·x + b` over a large vector.
pub fn axpb(a: f32, x: &[f32], b: f32, y: &mut [f32]) {
    debug_assert_eq!(x.len(), y.len());
    y.par_iter_mut().zip(x.par_iter()).for_each(|(o, &xi)| *o = a * xi + b);
}

/// `Reduction`: sum of a large vector (pairwise/parallel; deterministic enough for
/// the tolerance gate).
pub fn reduce_sum(x: &[f32]) -> f64 {
    x.par_iter().map(|&v| v as f64).sum()
}

/// `Stencil`: 1-D 3-point Laplacian `y[i] = x[i-1] - 2·x[i] + x[i+1]`, with the
/// ends clamped (one-sided zero). `y.len()==x.len()`.
pub fn stencil3(x: &[f32], y: &mut [f32]) {
    let n = x.len();
    debug_assert_eq!(y.len(), n);
    if n == 0 {
        return;
    }
    if n == 1 {
        y[0] = 0.0;
        return;
    }
    y[0] = x[1] - x[0];
    for i in 1..n - 1 {
        y[i] = x[i - 1] - 2.0 * x[i] + x[i + 1];
    }
    y[n - 1] = x[n - 2] - x[n - 1];
}

/// `AllPairs`: total pairwise inverse-distance potential `Σ_{i<j} 1/|p_i − p_j|`
/// over 3-D points (`pts.len()==3·n`). A representative N-body reduction.
pub fn allpairs_potential(pts: &[f32]) -> f64 {
    let n = pts.len() / 3;
    (0..n)
        .into_par_iter()
        .map(|i| {
            let (xi, yi, zi) = (pts[3 * i] as f64, pts[3 * i + 1] as f64, pts[3 * i + 2] as f64);
            let mut acc = 0.0;
            for j in (i + 1)..n {
                let dx = xi - pts[3 * j] as f64;
                let dy = yi - pts[3 * j + 1] as f64;
                let dz = zi - pts[3 * j + 2] as f64;
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                if r > 0.0 {
                    acc += 1.0 / r;
                }
            }
            acc
        })
        .sum()
}

/// `Fft`: in-place iterative radix-2 Cooley–Tukey FFT of a complex signal held as
/// parallel `re`/`im` slices. `re.len()==im.len()` must be a power of two.
/// `inverse=false` is the forward transform (no 1/N scaling — matches the textbook
/// DFT the tests check against).
pub fn fft_radix2(re: &mut [f32], im: &mut [f32], inverse: bool) {
    let n = re.len();
    debug_assert_eq!(im.len(), n);
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two(), "fft_radix2 requires a power-of-two length");

    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Danielson–Lanczos butterflies.
    let sign = if inverse { 1.0f64 } else { -1.0f64 };
    let mut len = 2usize;
    while len <= n {
        let ang = sign * 2.0 * std::f64::consts::PI / len as f64;
        let (wlen_re, wlen_im) = (ang.cos(), ang.sin());
        let mut i = 0usize;
        while i < n {
            let (mut w_re, mut w_im) = (1.0f64, 0.0f64);
            for k in 0..len / 2 {
                let u_re = re[i + k] as f64;
                let u_im = im[i + k] as f64;
                let v_re = re[i + k + len / 2] as f64 * w_re - im[i + k + len / 2] as f64 * w_im;
                let v_im = re[i + k + len / 2] as f64 * w_im + im[i + k + len / 2] as f64 * w_re;
                re[i + k] = (u_re + v_re) as f32;
                im[i + k] = (u_im + v_im) as f32;
                re[i + k + len / 2] = (u_re - v_re) as f32;
                im[i + k + len / 2] = (u_im - v_im) as f32;
                let nw_re = w_re * wlen_re - w_im * wlen_im;
                w_im = w_re * wlen_im + w_im * wlen_re;
                w_re = nw_re;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// `Scan`: inclusive prefix sum `y[i] = Σ_{k≤i} x[k]`. `y.len()==x.len()`.
pub fn prefix_sum(x: &[f32], y: &mut [f32]) {
    debug_assert_eq!(x.len(), y.len());
    let mut acc = 0.0f64;
    for (o, &xi) in y.iter_mut().zip(x.iter()) {
        acc += xi as f64;
        *o = acc as f32;
    }
}

/// `Divergent`: a branch-heavy Monte-Carlo step — estimate π by the fraction of
/// `steps` deterministic-LCG samples landing inside the unit circle (×4). The branch
/// (`inside ? ... : ...`) is the divergence this class represents. Deterministic so
/// it is reproducible as a correctness reference.
pub fn monte_carlo_pi(steps: usize) -> f64 {
    let mut state = 0x2545F4914F6CDD1Du64;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let mut inside = 0usize;
    for _ in 0..steps {
        let x = next();
        let y = next();
        if x * x + y * y <= 1.0 {
            inside += 1;
        }
    }
    if steps == 0 {
        0.0
    } else {
        4.0 * inside as f64 / steps as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemv_matches_hand() {
        // W = [[1,2],[3,4]], x = [1,1] → y = [3,7].
        let w = [1.0, 2.0, 3.0, 4.0];
        let x = [1.0, 1.0];
        let mut y = [0.0; 2];
        gemv(&w, &x, &mut y);
        assert!((y[0] - 3.0).abs() < 1e-5 && (y[1] - 7.0).abs() < 1e-5);
    }

    #[test]
    fn axpb_is_affine() {
        let x = [1.0, 2.0, 3.0];
        let mut y = [0.0; 3];
        axpb(2.0, &x, 1.0, &mut y);
        assert_eq!(y, [3.0, 5.0, 7.0]);
    }

    #[test]
    fn reduce_and_scan_agree_on_total() {
        let x: Vec<f32> = (1..=100).map(|i| i as f32).collect();
        let total = reduce_sum(&x);
        let mut ps = vec![0.0f32; x.len()];
        prefix_sum(&x, &mut ps);
        assert!((total - 5050.0).abs() < 1e-6);
        assert!((*ps.last().unwrap() as f64 - total).abs() < 1e-3);
        assert!((ps[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn stencil_of_linear_is_zero_interior() {
        // A linear ramp has zero second difference in the interior.
        let x: Vec<f32> = (0..16).map(|i| 2.0 * i as f32 + 1.0).collect();
        let mut y = vec![0.0f32; x.len()];
        stencil3(&x, &mut y);
        for v in &y[1..x.len() - 1] {
            assert!(v.abs() < 1e-4, "interior second difference of a ramp must be ~0, got {v}");
        }
    }

    #[test]
    fn fft_matches_naive_dft() {
        let n = 8usize;
        let signal: Vec<f32> = (0..n).map(|i| (i as f32 * 0.7).sin()).collect();
        let mut re: Vec<f32> = signal.clone();
        let mut im = vec![0.0f32; n];
        fft_radix2(&mut re, &mut im, false);
        // Naive DFT reference.
        for k in 0..n {
            let mut dr = 0.0f64;
            let mut di = 0.0f64;
            for (t, &s) in signal.iter().enumerate() {
                let ang = -2.0 * std::f64::consts::PI * (k * t) as f64 / n as f64;
                dr += s as f64 * ang.cos();
                di += s as f64 * ang.sin();
            }
            assert!((re[k] as f64 - dr).abs() < 1e-3, "re[{k}] {} vs {dr}", re[k]);
            assert!((im[k] as f64 - di).abs() < 1e-3, "im[{k}] {} vs {di}", im[k]);
        }
    }

    #[test]
    fn fft_inverse_recovers_signal() {
        let n = 16usize;
        let signal: Vec<f32> = (0..n).map(|i| (i as f32).cos()).collect();
        let mut re = signal.clone();
        let mut im = vec![0.0f32; n];
        fft_radix2(&mut re, &mut im, false);
        fft_radix2(&mut re, &mut im, true);
        for (i, &s) in signal.iter().enumerate() {
            assert!((re[i] / n as f32 - s).abs() < 1e-3, "ifft[{i}] {} vs {s}", re[i] / n as f32);
        }
    }

    #[test]
    fn monte_carlo_pi_is_in_range_and_deterministic() {
        let a = monte_carlo_pi(1 << 16);
        let b = monte_carlo_pi(1 << 16);
        assert_eq!(a, b, "must be deterministic to serve as a reference");
        assert!((a - std::f64::consts::PI).abs() < 0.1, "π estimate {a} too far off");
    }

    #[test]
    fn allpairs_two_points_is_inverse_distance() {
        // Two points 3 apart → potential = 1/3.
        let pts = [0.0, 0.0, 0.0, 3.0, 0.0, 0.0];
        let pot = allpairs_potential(&pts);
        assert!((pot - 1.0 / 3.0).abs() < 1e-6, "{pot}");
    }
}
