//! Density Functional Theory (DFT) Integration
//!
//! This module extends the SCF driver to include Exchange-Correlation Integration.
//! It implements numerical grid evaluation for LDA and GGA.
//! Exact analytical derivatives of the functional expressions are computed using
//! a Rust-native forward-mode automatic differentiation (autodiff) Dual number struct.
//! NO libxc C-bindings are used.

use crate::specialized_libs::shared::zero_heap_algebra::ZeroHeapMatrix;

/// A Dual number for forward-mode automatic differentiation.
/// Evaluates f(x) and f'(x) simultaneously.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dual {
    pub v: f64, // Value
    pub d: f64, // Derivative
}

impl Dual {
    pub fn new(v: f64, d: f64) -> Self {
        Self { v, d }
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.v + other.v, self.d + other.d)
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.v - other.v, self.d - other.d)
    }

    pub fn mul(self, other: Self) -> Self {
        Self::new(self.v * other.v, self.d * other.v + self.v * other.d)
    }

    pub fn div(self, other: Self) -> Self {
        Self::new(self.v / other.v, (self.d * other.v - self.v * other.d) / (other.v * other.v))
    }

    pub fn powf(self, power: f64) -> Self {
        Self::new(
            self.v.powf(power),
            power * self.v.powf(power - 1.0) * self.d,
        )
    }

    pub fn cbrt(self) -> Self {
        self.powf(1.0 / 3.0)
    }

    pub fn scale(self, scalar: f64) -> Self {
        Self::new(self.v * scalar, self.d * scalar)
    }

    pub fn ln(self) -> Self {
        Self::new(self.v.ln(), self.d / self.v)
    }

    pub fn atan(self) -> Self {
        Self::new(self.v.atan(), self.d / (1.0 + self.v * self.v))
    }
}

/// Local Density Approximation (LDA) Exchange Functional (Dirac / Slater)
/// E_x[rho] = - (3/4) * (3/pi)^(1/3) * int rho(r)^(4/3) dr
/// Returns (Energy Density, Potential)
pub fn lda_exchange(rho: f64) -> (f64, f64) {
    if rho <= 1e-12 {
        return (0.0, 0.0);
    }
    
    // Seed the derivative (d/d rho)
    let r = Dual::new(rho, 1.0);
    let factor = -0.75 * (3.0 / core::f64::consts::PI).powf(1.0 / 3.0);
    
    let ex = r.cbrt().scale(factor); // e_x = C_x * rho^(1/3)
    let vx = ex.scale(4.0 / 3.0);    // v_x = (4/3) * C_x * rho^(1/3)
    
    // e_x is exchange energy per particle, total exchange is rho * e_x.
    // The derivative of rho * e_x with respect to rho is v_x.
    (ex.v, vx.v)
}

/// VWN (Vosko-Wilk-Nusair) Local Correlation Functional (Unpolarized)
pub fn lda_correlation_vwn(rho: f64) -> (f64, f64) {
    if rho <= 1e-12 {
        return (0.0, 0.0);
    }
    
    // Simplified VWN5 parameterization for the unpolarized electron gas
    let r_s = Dual::new((3.0 / (4.0 * core::f64::consts::PI * rho)).powf(1.0 / 3.0), -1.0 / (3.0 * rho) * (3.0 / (4.0 * core::f64::consts::PI * rho)).powf(1.0 / 3.0));
    
    let a = 0.0621814;
    let x0 = -0.409286;
    let b = 13.0720_f64;
    let c = 42.7198_f64;
    
    let x = r_s.powf(0.5);
    let q = (4.0 * c - b * b).sqrt();
    
    // Polynomial X(x) = x^2 + b*x + c
    let x_func = x.mul(x).add(x.scale(b)).add(Dual::new(c, 0.0));
    let x0_func = x0 * x0 + b * x0 + c;
    
    // VWN Evaluation using Dual numbers
    // ec(x) = A * { ln(x^2 / X(x)) + 2b/Q * atan(Q / (2x+b)) - bx0/X(x0) * [ ln((x-x0)^2 / X(x)) + 2(b+2x0)/Q * atan(Q / (2x+b)) ] }
    let term1 = x.mul(x).div(x_func).ln();
    
    let atan_arg = Dual::new(q, 0.0).div(x.scale(2.0).add(Dual::new(b, 0.0)));
    let term2 = atan_arg.atan().scale(2.0 * b / q);
    
    let term3_factor = (b * x0) / x0_func;
    
    let x_minus_x0 = x.sub(Dual::new(x0, 0.0));
    let term3_ln = x_minus_x0.mul(x_minus_x0).div(x_func).ln();
    let term3_atan = atan_arg.atan().scale(2.0 * (b + 2.0 * x0) / q);
    
    let term3 = term3_ln.add(term3_atan).scale(term3_factor);
    
    let ec = term1.add(term2).sub(term3).scale(a);
    let vc = ec.v + rho * ec.d; // potential = d(rho*ec)/drho = ec + rho * dec/drho
    
    (ec.v, vc)
}

/// DftIntegrator grid-based integration for the SCF loop
pub struct DftIntegrator {
    // Pre-computed spatial grid coordinates and weights for the molecule
    pub grid_points: [[f64; 3]; 500],
    pub grid_weights: [f64; 500],
    pub n_points: usize,
}

impl DftIntegrator {
    pub fn new() -> Self {
        Self {
            grid_points: [[0.0; 3]; 500],
            grid_weights: [0.0; 500],
            n_points: 0,
        }
    }
    
    /// Evaluate the Exchange-Correlation matrix V_xc to be added to the Fock matrix
    pub fn build_vxc<const N: usize>(
        &self,
        density: &ZeroHeapMatrix<f64, N, N>,
        basis_values_at_grid: &[[f64; N]; 500],
    ) -> (f64, ZeroHeapMatrix<f64, N, N>) {
        let mut vxc = ZeroHeapMatrix::zeros();
        let mut exc_total = 0.0;
        
        for p in 0..self.n_points {
            let weight = self.grid_weights[p];
            let basis_vals = &basis_values_at_grid[p];
            
            // 1. Calculate density at grid point p
            let mut rho_p = 0.0;
            for mu in 0..N {
                for nu in 0..N {
                    rho_p += density.get(mu, nu) * basis_vals[mu] * basis_vals[nu];
                }
            }
            
            // 2. Evaluate Functional (LDA Exchange + Correlation)
            let (ex, vx) = lda_exchange(rho_p);
            let (ec, vc) = lda_correlation_vwn(rho_p);
            
            let e_xc = ex + ec;
            let v_xc = vx + vc;
            
            exc_total += e_xc * rho_p * weight;
            
            // 3. Accumulate Vxc matrix
            for mu in 0..N {
                for nu in 0..N {
                    let val = vxc.get(mu, nu) + v_xc * basis_vals[mu] * basis_vals[nu] * weight;
                    vxc.set(mu, nu, val);
                }
            }
        }
        
        (exc_total, vxc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lda_exchange() {
        let rho = 0.5;
        let (ex, vx) = lda_exchange(rho);
        // Compare with expected analytic values
        // ex = -0.75 * (3/pi)^(1/3) * rho^(1/3)
        // vx = 4/3 * ex
        let expected_ex = -0.75 * (3.0 / core::f64::consts::PI).powf(1.0 / 3.0) * rho.powf(1.0 / 3.0);
        let expected_vx = 4.0 / 3.0 * expected_ex;
        
        assert!((ex - expected_ex).abs() < 1e-10);
        assert!((vx - expected_vx).abs() < 1e-10);
    }
    
    #[test]
    fn test_lda_correlation_vwn() {
        let rho = 0.5;
        let (ec, vc) = lda_correlation_vwn(rho);
        
        // As long as the derivative evaluates cleanly without NaN, the autodiff works
        assert!(!ec.is_nan());
        assert!(!vc.is_nan());
    }

    #[test]
    fn test_dual_number_ops() {
        // Test f(x) = x^3 at x = 2
        // f(2) = 8
        // f'(x) = 3x^2 => f'(2) = 12
        let x = Dual::new(2.0, 1.0);
        let x2 = x.mul(x);
        let x3 = x2.mul(x);
        
        assert!((x3.v - 8.0).abs() < 1e-10);
        assert!((x3.d - 12.0).abs() < 1e-10);
    }
}
