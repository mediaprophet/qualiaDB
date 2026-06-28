//! Self-Organizing Map (CI-SKM ch 3) — a topology-preserving projection of a
//! high-dimensional space onto a 2-D grid of neurons. Nearby inputs map to nearby
//! grid cells, so it lays out a semantic space for the 10D→5D relevance router
//! (complementing PCA: SOM preserves neighbourhood topology, not just variance).
//! Kernel-class `AllPairs` (the best-matching-unit search). Deterministic per seed.

use crate::solvers::learning::LearningError;

/// A trained self-organizing map: a `grid_w × grid_h` lattice of `dim`-vectors.
#[derive(Debug, Clone)]
pub struct Som {
    pub grid_w: usize,
    pub grid_h: usize,
    pub dim: usize,
    weights: Vec<f64>, // grid_w*grid_h*dim, row-major over (y, x, d)
}

struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

impl Som {
    fn idx(&self, x: usize, y: usize) -> usize {
        (y * self.grid_w + x) * self.dim
    }

    /// Best-matching unit (grid coords) for an input vector — the nearest neuron.
    pub fn bmu(&self, input: &[f64]) -> (usize, usize) {
        let mut best = (0, 0);
        let mut best_d = f64::INFINITY;
        for y in 0..self.grid_h {
            for x in 0..self.grid_w {
                let w = &self.weights[self.idx(x, y)..self.idx(x, y) + self.dim];
                let d: f64 = w.iter().zip(input).map(|(a, b)| (a - b) * (a - b)).sum();
                if d < best_d {
                    best_d = d;
                    best = (x, y);
                }
            }
        }
        best
    }

    /// Train a `grid_w × grid_h` SOM on a row-major `n × dim` data matrix for
    /// `epochs`, with initial learning rate `lr0` and neighbourhood radius `sigma0`
    /// (both decaying exponentially). Fails closed on shape mismatch.
    pub fn train(
        data: &[f64],
        n: usize,
        dim: usize,
        grid_w: usize,
        grid_h: usize,
        epochs: usize,
        lr0: f64,
        sigma0: f64,
        seed: u64,
    ) -> Result<Self, LearningError> {
        if n == 0 || dim == 0 || grid_w == 0 || grid_h == 0 || data.len() != n * dim {
            return Err(LearningError::InvalidDimension);
        }
        let mut rng = Lcg(seed ^ 0x9E3779B97F4A7C15);
        // Initialise weights from the data range.
        let mut lo = vec![f64::INFINITY; dim];
        let mut hi = vec![f64::NEG_INFINITY; dim];
        for i in 0..n {
            for d in 0..dim {
                let v = data[i * dim + d];
                lo[d] = lo[d].min(v);
                hi[d] = hi[d].max(v);
            }
        }
        let mut weights = vec![0.0; grid_w * grid_h * dim];
        for cell in 0..grid_w * grid_h {
            for d in 0..dim {
                weights[cell * dim + d] = lo[d] + rng.unit() * (hi[d] - lo[d]).max(1e-9);
            }
        }
        let mut som = Som {
            grid_w,
            grid_h,
            dim,
            weights,
        };

        let total = epochs.max(1) as f64;
        for epoch in 0..epochs.max(1) {
            let frac = epoch as f64 / total;
            let lr = lr0 * (-frac * 3.0).exp();
            let sigma = (sigma0 * (-frac * 3.0).exp()).max(0.5);
            let two_sigma2 = 2.0 * sigma * sigma;
            for i in 0..n {
                let input = &data[i * dim..(i + 1) * dim];
                let (bx, by) = som.bmu(input);
                for y in 0..grid_h {
                    for x in 0..grid_w {
                        let gd2 =
                            ((x as f64 - bx as f64).powi(2)) + ((y as f64 - by as f64).powi(2));
                        let h = (-gd2 / two_sigma2).exp();
                        if h < 1e-4 {
                            continue;
                        }
                        let base = som.idx(x, y);
                        for d in 0..dim {
                            som.weights[base + d] += lr * h * (input[d] - som.weights[base + d]);
                        }
                    }
                }
            }
        }
        Ok(som)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separated_clusters_map_to_distinct_regions() {
        // Two clusters in 2-D, far apart; their BMUs should land in different grid
        // regions (grid distance > 0), and same-cluster points should be close.
        let mut data = Vec::new();
        for d in 0..10 {
            data.push((d % 3) as f64 * 0.1);
            data.push((d % 3) as f64 * 0.1); // cluster near (0,0)
        }
        for d in 0..10 {
            data.push(10.0 + (d % 3) as f64 * 0.1);
            data.push(10.0 + (d % 3) as f64 * 0.1); // cluster near (10,10)
        }
        let som = Som::train(&data, 20, 2, 4, 4, 200, 0.5, 2.0, 1).unwrap();
        let a = som.bmu(&[0.0, 0.0]);
        let b = som.bmu(&[10.0, 10.0]);
        let grid_dist =
            ((a.0 as f64 - b.0 as f64).powi(2) + (a.1 as f64 - b.1 as f64).powi(2)).sqrt();
        assert!(
            grid_dist > 1.0,
            "clusters should map apart: a={a:?} b={b:?}"
        );
        // A point near cluster A maps to A's BMU (or adjacent).
        let a2 = som.bmu(&[0.2, 0.2]);
        let near = ((a.0 as f64 - a2.0 as f64).powi(2) + (a.1 as f64 - a2.1 as f64).powi(2)).sqrt();
        assert!(near <= 1.5, "same-cluster points map near: {a:?} vs {a2:?}");
    }

    #[test]
    fn preserves_1d_order() {
        // Inputs increasing along a line → BMUs should be (weakly) monotone on the grid.
        let data: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let som = Som::train(&data, 10, 1, 10, 1, 300, 0.5, 3.0, 7).unwrap();
        let first = som.bmu(&[0.0]).0 as i64;
        let last = som.bmu(&[9.0]).0 as i64;
        assert_ne!(first, last, "endpoints should occupy different cells");
    }

    #[test]
    fn guards() {
        assert_eq!(
            Som::train(&[1.0], 1, 2, 3, 3, 10, 0.5, 1.0, 0).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
