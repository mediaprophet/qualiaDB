//! Dynamic element-wise vector operations — the foundational rank-1 linear algebra the
//! transformer is built from beyond GEMM: the **residual connection** (`x + sublayer(x)`) is
//! vector addition, and the **gated activation** (SwiGLU) uses the **Hadamard product**
//! (element-wise multiply). Plain arithmetic; caller-owned, zero allocation.

use crate::solvers::SolversError;

/// `c[i] = a[i] + b[i]` — vector addition (the residual connection `x + sublayer(x)`).
pub fn add_into(a: &[f64], b: &[f64], c: &mut [f64]) -> Result<(), SolversError> {
    if a.len() != b.len() || a.len() != c.len() {
        return Err(SolversError::InvalidDimension);
    }
    for i in 0..a.len() {
        c[i] = a[i] + b[i];
    }
    Ok(())
}

/// `a[i] += b[i]` — in-place residual add.
pub fn add_assign(a: &mut [f64], b: &[f64]) -> Result<(), SolversError> {
    if a.len() != b.len() {
        return Err(SolversError::InvalidDimension);
    }
    for i in 0..a.len() {
        a[i] += b[i];
    }
    Ok(())
}

/// `c[i] = a[i] · b[i]` — the Hadamard (element-wise) product, e.g. the SwiGLU gate `silu(g) ⊙ u`.
pub fn hadamard_into(a: &[f64], b: &[f64], c: &mut [f64]) -> Result<(), SolversError> {
    if a.len() != b.len() || a.len() != c.len() {
        return Err(SolversError::InvalidDimension);
    }
    for i in 0..a.len() {
        c[i] = a[i] * b[i];
    }
    Ok(())
}

/// `a[i] *= b[i]` — in-place Hadamard product.
pub fn hadamard_assign(a: &mut [f64], b: &[f64]) -> Result<(), SolversError> {
    if a.len() != b.len() {
        return Err(SolversError::InvalidDimension);
    }
    for i in 0..a.len() {
        a[i] *= b[i];
    }
    Ok(())
}

/// `a[i] *= s` — scalar scaling.
pub fn scale(a: &mut [f64], s: f64) {
    for v in a.iter_mut() {
        *v *= s;
    }
}

/// `y[i] += α·x[i]` — the BLAS `axpy`.
pub fn axpy(alpha: f64, x: &[f64], y: &mut [f64]) -> Result<(), SolversError> {
    if x.len() != y.len() {
        return Err(SolversError::InvalidDimension);
    }
    for i in 0..x.len() {
        y[i] += alpha * x[i];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_is_vector_addition() {
        let a = [1.0, 2.0, 3.0];
        let b = [10.0, 20.0, 30.0];
        let mut c = [0.0; 3];
        add_into(&a, &b, &mut c).unwrap();
        assert_eq!(c, [11.0, 22.0, 33.0]);
        let mut x = a;
        add_assign(&mut x, &b).unwrap();
        assert_eq!(x, [11.0, 22.0, 33.0]);
    }

    #[test]
    fn hadamard_is_elementwise_product() {
        let a = [2.0, 3.0, 4.0];
        let b = [5.0, 0.0, -1.0];
        let mut c = [0.0; 3];
        hadamard_into(&a, &b, &mut c).unwrap();
        assert_eq!(c, [10.0, 0.0, -4.0]);
        let mut x = a;
        hadamard_assign(&mut x, &b).unwrap();
        assert_eq!(x, [10.0, 0.0, -4.0]);
    }

    #[test]
    fn scale_and_axpy() {
        let mut a = [1.0, 2.0, 3.0];
        scale(&mut a, 2.0);
        assert_eq!(a, [2.0, 4.0, 6.0]);
        let x = [1.0, 1.0, 1.0];
        let mut y = [10.0, 20.0, 30.0];
        axpy(0.5, &x, &mut y).unwrap();
        assert_eq!(y, [10.5, 20.5, 30.5]);
    }

    #[test]
    fn rejects_length_mismatch() {
        let a = [1.0, 2.0];
        let b = [1.0];
        let mut c = [0.0; 2];
        assert!(matches!(
            add_into(&a, &b, &mut c),
            Err(SolversError::InvalidDimension)
        ));
    }
}
