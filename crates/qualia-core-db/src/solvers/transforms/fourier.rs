//! Discrete Fourier transform and its inverse over complex samples.

/// A complex number as `(real, imaginary)`.
pub type Cplx = (f64, f64);

#[inline]
fn cadd(a: Cplx, b: Cplx) -> Cplx {
    (a.0 + b.0, a.1 + b.1)
}
#[inline]
fn cmul(a: Cplx, b: Cplx) -> Cplx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Forward DFT: `X[k] = Σ_n x[n] · e^{−2πi kn/N}`.
///
/// This is the **f64-exact** reference: a naive O(N²) DFT computed entirely on
/// the CPU in `f64`. It is un-normalized with the forward sign convention
/// `e^{−2πi kn/N}`. Its bit-for-bit f64 behaviour is a contract — [`idft`] round-
/// trips against it to ~1e-9 — so it is deliberately **not** routed through the
/// f32 GPU forge. Spectral callers that accept f32 precision should call
/// [`dft_accelerated`] instead, which uses the WGSL forge when available.
pub fn dft(x: &[Cplx]) -> Vec<Cplx> {
    dft_cpu(x)
}

/// CPU-only forward DFT — the exact `f64` reference math, factored out so both
/// [`dft`] and the CPU floor of [`dft_accelerated`] share one definition.
fn dft_cpu(x: &[Cplx]) -> Vec<Cplx> {
    let n = x.len();
    let mut out = vec![(0.0, 0.0); n];
    if n == 0 {
        return out;
    }
    let w = -2.0 * core::f64::consts::PI / n as f64;
    for (k, ok) in out.iter_mut().enumerate() {
        let mut acc = (0.0, 0.0);
        for (j, &xj) in x.iter().enumerate() {
            let ang = w * (k * j) as f64;
            acc = cadd(acc, cmul(xj, (ang.cos(), ang.sin())));
        }
        *ok = acc;
    }
    out
}

/// Best-path forward DFT for **spectral callers that accept `f32` precision**:
/// `X[k] = Σ_n x[n] · e^{−2πi kn/N}`, same un-normalized forward convention as
/// [`dft`], but accelerated on the GPU when possible.
///
/// # Why this is a separate function (and `dft` is not silently swapped)
///
/// The forge FFT is `f32` (WGSL has no `f64`), so an accelerated result carries
/// **f32 precision** (≈ 1e-3 .. 1e-2 of the spectral magnitude for `N ≤ 1024`),
/// not f64-exact bits. Routing the public [`dft`] through it would break callers
/// that rely on its f64-exact contract (e.g. the [`idft`] round-trip). So the
/// fast path is exposed here as an **explicit opt-in**: callers doing
/// audio / magnitude / feature-extraction style work — where f32 is the
/// universal norm — choose it knowingly. The inverse [`idft`] stays wholly on
/// the CPU (the forge FFT is forward-only).
///
/// # Convention match (no rescale)
///
/// The forge's radix-2 FFT and its CPU DFT oracle
/// ([`crate::wgsl_forge::dispatch::fft_f32`]) use the **identical** un-normalized
/// forward sign convention `e^{−2πi kn/N}`. So the forge result is the same
/// transform as [`dft`] — **no sign flip, no `1/N` / `1/√N` rescale** is applied,
/// only an `f64 → f32 → f64` width conversion.
///
/// # Path selection
///
/// The WGSL forge runs only when **all** of:
/// * `N = x.len()` is a power of two and `2 ≤ N ≤ 1024` (the forge runs ONE
///   workgroup of `N` threads, so `N` is bounded by the single-workgroup size);
/// * a wgpu accelerator is present on this machine
///   ([`caps().wgpu`](crate::wgsl_forge::dispatch::caps)).
///
/// Otherwise — and on **any** forge error — it falls through to the f64 CPU DFT
/// ([`dft`]'s exact math). The result is always a valid forward DFT; only the
/// precision (f32 vs f64) and the compute device differ between the two paths.
pub fn dft_accelerated(x: &[Cplx]) -> Vec<Cplx> {
    let n = x.len();

    // ── Accelerated fast path: WGSL forge forward FFT (f32), same convention. ──
    // Eligible only for a power-of-two N in [2, 1024] on a machine with a wgpu
    // adapter; any forge error falls straight through to the exact CPU DFT.
    #[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
    if n.is_power_of_two() && (2..=1024).contains(&n) && crate::wgsl_forge::dispatch::caps().wgpu {
        // f64 (re, im) -> interleaved f32 [re0, im0, re1, im1, …].
        let mut interleaved = Vec::with_capacity(2 * n);
        for &(re, im) in x {
            interleaved.push(re as f32);
            interleaved.push(im as f32);
        }
        if let Ok(spectrum) = crate::wgsl_forge::dispatch::fft_f32(&interleaved) {
            // The forge guarantees a 2*n interleaved result; widen back to f64.
            if spectrum.len() == 2 * n {
                let mut out = vec![(0.0, 0.0); n];
                for (k, ok) in out.iter_mut().enumerate() {
                    *ok = (spectrum[2 * k] as f64, spectrum[2 * k + 1] as f64);
                }
                return out;
            }
        }
        // Forge ineligible/failed or returned an unexpected length — fall through
        // to the exact CPU DFT (never broken).
    }

    dft_cpu(x)
}

/// Inverse DFT: `x[n] = (1/N) Σ_k X[k] · e^{+2πi kn/N}`.
pub fn idft(spectrum: &[Cplx]) -> Vec<Cplx> {
    let n = spectrum.len();
    let mut out = vec![(0.0, 0.0); n];
    if n == 0 {
        return out;
    }
    let w = 2.0 * core::f64::consts::PI / n as f64;
    let inv = 1.0 / n as f64;
    for (j, oj) in out.iter_mut().enumerate() {
        let mut acc = (0.0, 0.0);
        for (k, &xk) in spectrum.iter().enumerate() {
            let ang = w * (k * j) as f64;
            acc = cadd(acc, cmul(xk, (ang.cos(), ang.sin())));
        }
        *oj = (acc.0 * inv, acc.1 * inv);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    fn re(v: &[f64]) -> Vec<Cplx> {
        v.iter().map(|&r| (r, 0.0)).collect()
    }

    #[test]
    fn dft_of_constant_is_an_impulse() {
        // DFT([1,1,1,1]) = [4,0,0,0]
        let x = dft(&re(&[1.0, 1.0, 1.0, 1.0]));
        assert!((x[0].0 - 4.0).abs() < EPS && x[0].1.abs() < EPS);
        for k in 1..4 {
            assert!(x[k].0.abs() < EPS && x[k].1.abs() < EPS);
        }
    }

    #[test]
    fn dft_of_impulse_is_constant() {
        // DFT([1,0,0,0]) = [1,1,1,1]
        let x = dft(&re(&[1.0, 0.0, 0.0, 0.0]));
        for k in 0..4 {
            assert!((x[k].0 - 1.0).abs() < EPS && x[k].1.abs() < EPS);
        }
    }

    /// The wired forward transform [`dft_accelerated`] of a power-of-two signal
    /// must match the known analytic spectrum whether the WGSL forge fast path or
    /// the CPU floor runs (so it is validated on a GPU box and a GPU-less box
    /// alike). Two cases, both at N=8 (a power of two ≤ 1024, so the forge path is
    /// eligible when an adapter is present):
    ///
    /// 1. A real unit impulse `x[0]=1` (rest 0) → flat spectrum, every bin `(1,0)`.
    /// 2. A real cosine at integer frequency f=1, `x[j]=cos(2π·1·j/8)`, has all its
    ///    energy in the two conjugate-symmetric bins k=1 and k=N−1=7, each
    ///    `(N/2, 0) = (4, 0)`, and zero elsewhere.
    ///
    /// Tolerance is f32-appropriate (1e-2): the accelerated path carries f32
    /// precision, so this validates the wired path on a GPU box without being so
    /// tight that f32 rounding trips it.
    #[test]
    fn accelerated_dft_matches_known_spectrum() {
        const TOL: f64 = 1e-2;

        // Case 1: impulse → all-ones.
        let imp = re(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let s = dft_accelerated(&imp);
        assert_eq!(s.len(), 8);
        for (k, b) in s.iter().enumerate() {
            assert!(
                (b.0 - 1.0).abs() < TOL,
                "impulse bin {k} re should be 1, got {}",
                b.0
            );
            assert!(
                b.1.abs() < TOL,
                "impulse bin {k} im should be 0, got {}",
                b.1
            );
        }

        // Case 2: cos(2π·1·j/8) → energy only in k=1 and k=7, each (4, 0).
        let n = 8usize;
        let cosine: Vec<Cplx> = (0..n)
            .map(|j| {
                let ang = 2.0 * core::f64::consts::PI * (j as f64) / (n as f64);
                (ang.cos(), 0.0)
            })
            .collect();
        let cs = dft_accelerated(&cosine);
        for (k, b) in cs.iter().enumerate() {
            let expect_re = if k == 1 || k == n - 1 {
                (n as f64) / 2.0
            } else {
                0.0
            };
            assert!(
                (b.0 - expect_re).abs() < TOL,
                "cos bin {k} re: expected {expect_re}, got {}",
                b.0
            );
            assert!(b.1.abs() < TOL, "cos bin {k} im should be ~0, got {}", b.1);
        }
    }

    /// The wired accelerated path must agree with the exact CPU [`dft`] reference
    /// to f32 tolerance on a non-trivial power-of-two signal — pinning that the
    /// fast path is the SAME transform (same sign/scale), only in f32. Holds on a
    /// GPU box (forge ran) and a GPU-less box (both are the CPU DFT, exact).
    #[test]
    fn accelerated_dft_agrees_with_cpu_reference() {
        let x = re(&[3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]); // N=8
        let exact = dft(&x);
        let fast = dft_accelerated(&x);
        assert_eq!(exact.len(), fast.len());
        for (k, (e, f)) in exact.iter().zip(&fast).enumerate() {
            // f32 magnitudes here are O(10); a 1e-2 absolute tol comfortably
            // covers the f32 rounding while still catching any sign/scale error.
            assert!(
                (e.0 - f.0).abs() < 1e-2,
                "bin {k} re: exact {} vs fast {}",
                e.0,
                f.0
            );
            assert!(
                (e.1 - f.1).abs() < 1e-2,
                "bin {k} im: exact {} vs fast {}",
                e.1,
                f.1
            );
        }
    }

    #[test]
    fn inverse_round_trips() {
        let x = re(&[3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]);
        let back = idft(&dft(&x));
        for (a, b) in x.iter().zip(&back) {
            assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        }
    }
}
