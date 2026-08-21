//! Frame morphisms (Galilean → Lorentz) (W5).

use crate::value::Frame;

/// A Galilean frame morphism — the low-velocity limit.
///
/// t' = t
/// x' = x - v*t
///
/// No time dilation, no length contraction. Valid for v << c.
#[derive(Debug, Clone, PartialEq)]
pub struct GalileanMorphism {
    /// Velocity of the target frame relative to the source frame [vx, vy, vz].
    pub velocity: Vec<f64>,
}

impl GalileanMorphism {
    pub fn new(velocity: Vec<f64>) -> Self {
        Self { velocity }
    }

    /// Transform a position [x, y, z, t] from source to target frame.
    /// Returns [x', y', z', t'].
    pub fn transform(&self, position: &[f64]) -> Vec<f64> {
        if position.len() < 4 {
            return position.to_vec();
        }
        let t = position[3];
        let mut result = Vec::with_capacity(4);
        for i in 0..3 {
            let v = self.velocity.get(i).copied().unwrap_or(0.0);
            result.push(position[i] - v * t);
        }
        result.push(t); // t' = t
        result
    }

    /// The inverse morphism (target → source).
    pub fn inverse(&self) -> Self {
        Self {
            velocity: self.velocity.iter().map(|v| -v).collect(),
        }
    }
}

/// A Lorentz frame morphism — the relativistic transform.
///
/// Along the x-axis with velocity v:
/// t' = gamma * (t - v*x/c²)
/// x' = gamma * (x - v*t)
///
/// where gamma = 1/sqrt(1 - v²/c²).
///
/// Time dilation and length contraction are explicit. Valid for all
/// v < c.
#[derive(Debug, Clone, PartialEq)]
pub struct LorentzMorphism {
    /// Velocity of the target frame along the x-axis (m/s).
    pub velocity: f64,
    /// Speed of light (m/s). Default: 299_792_458.0.
    pub c: f64,
}

impl LorentzMorphism {
    pub const C: f64 = 299_792_458.0;

    pub fn new(velocity: f64) -> Self {
        Self {
            velocity,
            c: Self::C,
        }
    }

    /// Create with a custom speed of light (for testing).
    pub fn with_c(velocity: f64, c: f64) -> Self {
        Self { velocity, c }
    }

    /// Lorentz factor gamma = 1/sqrt(1 - v²/c²).
    pub fn gamma(&self) -> f64 {
        let beta_sq = (self.velocity * self.velocity) / (self.c * self.c);
        1.0 / (1.0 - beta_sq).sqrt()
    }

    /// Transform [t, x, y, z] from source to target frame.
    /// Returns [t', x', y', z'].
    pub fn transform(&self, coords: &[f64]) -> Vec<f64> {
        if coords.len() < 4 {
            return coords.to_vec();
        }
        let t = coords[0];
        let x = coords[1];
        let y = coords[2];
        let z = coords[3];
        let g = self.gamma();
        let v = self.velocity;
        let c = self.c;
        let t_prime = g * (t - v * x / (c * c));
        let x_prime = g * (x - v * t);
        vec![t_prime, x_prime, y, z]
    }

    /// The inverse morphism (target → source).
    pub fn inverse(&self) -> Self {
        Self {
            velocity: -self.velocity,
            c: self.c,
        }
    }
}

/// Apply a frame morphism to a Frame, producing the transformed frame.
pub fn transform_frame(frame: &Frame, morphism: &GalileanMorphism) -> Frame {
    let origin = morphism.transform(&frame.origin);
    let basis = frame.basis.clone(); // basis unchanged for Galilean
    Frame {
        origin,
        basis,
        parent: frame.parent.clone(),
    }
}
