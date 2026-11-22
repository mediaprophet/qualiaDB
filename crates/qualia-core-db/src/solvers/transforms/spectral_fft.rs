//! Exact `f64` O(N log N) Cooley-Tukey radix-2 & Bluestein chirp-z arbitrary-length
//! 1D/2D/3D FFT/IFFT, spectrum shifts, analytic Hilbert envelope, and windows.
use crate::solvers::transforms::fourier::Cplx;
use core::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Boxcar,
    Hanning,
    Hamming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralError {
    EmptyInput,
    ShapeMismatch,
    NonFiniteInput,
    SizeLimit,
}

impl std::fmt::Display for SpectralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "spectral transform error: {self:?}")
    }
}
impl std::error::Error for SpectralError {}

#[inline]
fn cmul(a: Cplx, b: Cplx) -> Cplx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
fn cadd(a: Cplx, b: Cplx) -> Cplx {
    (a.0 + b.0, a.1 + b.1)
}

#[inline]
fn csub(a: Cplx, b: Cplx) -> Cplx {
    (a.0 - b.0, a.1 - b.1)
}

pub fn validate_finite_complex(buf: &[Cplx]) -> Result<(), SpectralError> {
    if buf.iter().any(|&(re, im)| !re.is_finite() || !im.is_finite()) {
        return Err(SpectralError::NonFiniteInput);
    }
    Ok(())
}

/// In-place iterative Cooley-Tukey radix-2 DIT FFT (`sign = -1.0` forward, `+1.0` inverse; unscaled).
fn fft_pow2_inplace(buf: &mut [Cplx], sign: f64) {
    let n = buf.len();
    if n <= 1 {
        return;
    }
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while (j & bit) != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            buf.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = sign * 2.0 * PI / len as f64;
        let wlen = (ang.cos(), ang.sin());
        let half = len >> 1;
        for start in (0..n).step_by(len) {
            let mut w = (1.0, 0.0);
            for k in 0..half {
                let u = buf[start + k];
                let v = cmul(buf[start + k + half], w);
                buf[start + k] = cadd(u, v);
                buf[start + k + half] = csub(u, v);
                w = cmul(w, wlen);
            }
        }
        len <<= 1;
    }
}

/// Bluestein Chirp-Z O(M log M) arbitrary-length 1D unscaled transform.
fn fft_bluestein(buf: &[Cplx], sign: f64) -> Result<Vec<Cplx>, SpectralError> {
    let n = buf.len();
    let period = n.checked_mul(2).ok_or(SpectralError::SizeLimit)?;
    let m = (period - 1).checked_next_power_of_two().ok_or(SpectralError::SizeLimit)?;
    let mut a = vec![(0.0, 0.0); m];
    let mut b = vec![(0.0, 0.0); m];
    let mut chirps = Vec::with_capacity(n);

    for k in 0..n {
        let k_wide = k as u128;
        let phase_index = (k_wide * k_wide) % period as u128;
        let theta = sign * PI * phase_index as f64 / n as f64;
        let w = (theta.cos(), theta.sin());
        chirps.push(w);
        a[k] = cmul(buf[k], w);
        b[k] = (w.0, -w.1);
        if k > 0 {
            b[m - k] = b[k];
        }
    }

    fft_pow2_inplace(&mut a, -1.0);
    fft_pow2_inplace(&mut b, -1.0);
    for (ai, bi) in a.iter_mut().zip(b.iter()) {
        *ai = cmul(*ai, *bi);
    }
    fft_pow2_inplace(&mut a, 1.0);

    let inv_m = 1.0 / m as f64;
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let conv = (a[k].0 * inv_m, a[k].1 * inv_m);
        out.push(cmul(conv, chirps[k]));
    }
    Ok(out)
}

/// Forward 1D DFT (`X[k] = sum_n x[n] exp(-2*pi*i*k*n/N)`), matching `solvers::transforms::fourier::dft`.
pub fn fft_1d(x: &[Cplx]) -> Result<Vec<Cplx>, SpectralError> {
    if x.is_empty() {
        return Err(SpectralError::EmptyInput);
    }
    validate_finite_complex(x)?;
    if x.len().is_power_of_two() {
        let mut out = x.to_vec();
        fft_pow2_inplace(&mut out, -1.0);
        Ok(out)
    } else {
        fft_bluestein(x, -1.0)
    }
}

/// Inverse 1D DFT (`x[n] = (1/N) * sum_k X[k] exp(+2*pi*i*k*n/N)`), matching `solvers::transforms::fourier::idft`.
pub fn ifft_1d(x: &[Cplx]) -> Result<Vec<Cplx>, SpectralError> {
    if x.is_empty() {
        return Err(SpectralError::EmptyInput);
    }
    validate_finite_complex(x)?;
    let n = x.len();
    let mut out = if n.is_power_of_two() {
        let mut v = x.to_vec();
        fft_pow2_inplace(&mut v, 1.0);
        v
    } else {
        fft_bluestein(x, 1.0)?
    };
    let scale = 1.0 / n as f64;
    for v in &mut out {
        v.0 *= scale;
        v.1 *= scale;
    }
    Ok(out)
}

pub fn fftshift_1d<T: Copy>(data: &[T], out: &mut [T]) {
    let n = data.len();
    if n == 0 {
        return;
    }
    let split = n / 2;
    let tail = n - split;
    out[..tail].copy_from_slice(&data[split..]);
    out[tail..].copy_from_slice(&data[..split]);
}

pub fn ifftshift_1d<T: Copy>(data: &[T], out: &mut [T]) {
    let n = data.len();
    if n == 0 {
        return;
    }
    let split = n.div_ceil(2);
    let tail = n - split;
    out[..tail].copy_from_slice(&data[split..]);
    out[tail..].copy_from_slice(&data[..split]);
}

pub fn fft_2d(data: &[Cplx], rows: usize, cols: usize, inverse: bool) -> Result<Vec<Cplx>, SpectralError> {
    let total = rows.checked_mul(cols).ok_or(SpectralError::SizeLimit)?;
    if total == 0 || data.len() != total {
        return Err(SpectralError::ShapeMismatch);
    }
    validate_finite_complex(data)?;
    let mut work = vec![(0.0, 0.0); total];
    for r in 0..rows {
        let row = &data[r * cols..(r + 1) * cols];
        let tf = if inverse { ifft_1d(row)? } else { fft_1d(row)? };
        work[r * cols..(r + 1) * cols].copy_from_slice(&tf);
    }
    let mut col_buf = vec![(0.0, 0.0); rows];
    for c in 0..cols {
        for r in 0..rows {
            col_buf[r] = work[r * cols + c];
        }
        let tf = if inverse { ifft_1d(&col_buf)? } else { fft_1d(&col_buf)? };
        for r in 0..rows {
            work[r * cols + c] = tf[r];
        }
    }
    Ok(work)
}

pub fn fft_3d(
    data: &[Cplx],
    slices: usize,
    rows: usize,
    cols: usize,
    inverse: bool,
) -> Result<Vec<Cplx>, SpectralError> {
    let plane = rows.checked_mul(cols).ok_or(SpectralError::SizeLimit)?;
    let total = slices.checked_mul(plane).ok_or(SpectralError::SizeLimit)?;
    if slices == 0 || data.len() != total {
        return Err(SpectralError::ShapeMismatch);
    }
    let mut work = vec![(0.0, 0.0); total];
    for s in 0..slices {
        let sl = fft_2d(&data[s * plane..(s + 1) * plane], rows, cols, inverse)?;
        work[s * plane..(s + 1) * plane].copy_from_slice(&sl);
    }
    let mut z_buf = vec![(0.0, 0.0); slices];
    for idx in 0..plane {
        for s in 0..slices {
            z_buf[s] = work[s * plane + idx];
        }
        let tf = if inverse { ifft_1d(&z_buf)? } else { fft_1d(&z_buf)? };
        for s in 0..slices {
            work[s * plane + idx] = tf[s];
        }
    }
    Ok(work)
}

pub fn fftshift_3d<T: Copy + Default>(
    data: &[T],
    slices: usize,
    rows: usize,
    cols: usize,
    inverse: bool,
) -> Result<Vec<T>, SpectralError> {
    let plane = rows.checked_mul(cols).ok_or(SpectralError::SizeLimit)?;
    let total = slices.checked_mul(plane).ok_or(SpectralError::SizeLimit)?;
    if data.len() != total {
        return Err(SpectralError::ShapeMismatch);
    }
    if total == 0 {
        return Ok(Vec::new());
    }
    let sz = if inverse { slices.div_ceil(2) } else { slices / 2 };
    let sy = if inverse { rows.div_ceil(2) } else { rows / 2 };
    let sx = if inverse { cols.div_ceil(2) } else { cols / 2 };
    let mut out = vec![T::default(); data.len()];
    for s in 0..slices {
        let ds = (s + sz) % slices;
        for r in 0..rows {
            let dr = (r + sy) % rows;
            for c in 0..cols {
                let dc = (c + sx) % cols;
                out[ds * plane + dr * cols + dc] = data[s * plane + r * cols + c];
            }
        }
    }
    Ok(out)
}

/// Exact Marple frequency-domain analytic signal envelope (`|x_a[n]| = hypot(x, H{x})`).
pub fn analytic_hilbert_envelope_1d(signal: &[f64]) -> Result<Vec<f64>, SpectralError> {
    if signal.is_empty() {
        return Err(SpectralError::EmptyInput);
    }
    if signal.iter().any(|v| !v.is_finite()) {
        return Err(SpectralError::NonFiniteInput);
    }
    let n = signal.len();
    let cx: Vec<Cplx> = signal.iter().map(|&v| (v, 0.0)).collect();
    let mut spec = fft_1d(&cx)?;
    if n > 1 {
        let half = n / 2;
        for k in 1..half {
            spec[k] = (spec[k].0 * 2.0, spec[k].1 * 2.0);
        }
        if n % 2 == 1 {
            spec[half] = (spec[half].0 * 2.0, spec[half].1 * 2.0);
        }
        for k in (half + 1)..n {
            spec[k] = (0.0, 0.0);
        }
    }
    let analytic = ifft_1d(&spec)?;
    Ok(analytic.into_iter().map(|(re, im)| re.hypot(im)).collect())
}

/// Normalized symmetric apodization window (`[0..n]`, max `1.0`).
pub fn apodization_weight(kind: WindowKind, t_norm_abs: f64) -> f64 {
    if t_norm_abs > 1.0 {
        return 0.0;
    }
    match kind {
        WindowKind::Boxcar => 1.0,
        WindowKind::Hanning => 0.5 * (1.0 + (PI * t_norm_abs).cos()),
        WindowKind::Hamming => 0.54 + 0.46 * (PI * t_norm_abs).cos(),
    }
}
