//! k-means clustering (ISL ch 12.4, PRML ch 9.1) — Lloyd's algorithm with
//! k-means++ seeding, over a row-major feature matrix.
//!
//! Assign each point to its nearest centroid (squared Euclidean), recompute each
//! centroid as the mean of its members, repeat to convergence. k-means++ seeding
//! spreads the initial centroids to avoid poor local minima. Kernel-class
//! `AllPairs` (the point↔centroid distances), dispatch-ready; deterministic given
//! the seed.

use crate::solvers::learning::LearningError;

/// A fitted k-means model.
#[derive(Debug, Clone)]
pub struct KMeansModel {
    /// `k × p` centroids, row-major.
    pub centroids: Vec<f64>,
    /// Cluster assignment per input row.
    pub labels: Vec<usize>,
    /// Within-cluster sum of squared distances (the objective; lower is tighter).
    pub inertia: f64,
    pub k: usize,
    pub p: usize,
    pub n_iter: usize,
    pub converged: bool,
}

impl KMeansModel {
    /// Index of the nearest centroid to a feature row.
    pub fn predict_row(&self, x_row: &[f64]) -> usize {
        nearest(&self.centroids, self.k, self.p, x_row).0
    }
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

#[inline]
fn sq_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

/// (index, squared distance) of the nearest of `k` row-major centroids to `point`.
fn nearest(centroids: &[f64], k: usize, p: usize, point: &[f64]) -> (usize, f64) {
    let mut best = 0;
    let mut best_d = f64::INFINITY;
    for c in 0..k {
        let d = sq_dist(&centroids[c * p..(c + 1) * p], point);
        if d < best_d {
            best_d = d;
            best = c;
        }
    }
    (best, best_d)
}

/// k-means++ seeding: choose `k` initial centroids spread by D²-weighting.
fn kmeans_pp(x: &[f64], n: usize, p: usize, k: usize, rng: &mut Lcg) -> Vec<f64> {
    let mut centroids = vec![0.0; k * p];
    // First centroid uniformly at random.
    let first = ((rng.unit() * n as f64) as usize).min(n - 1);
    centroids[..p].copy_from_slice(&x[first * p..(first + 1) * p]);
    let mut d2 = vec![0.0; n];
    for c in 1..k {
        // D²(point) = squared distance to the nearest chosen centroid.
        let mut total = 0.0;
        for i in 0..n {
            let (_, d) = nearest(&centroids[..c * p], c, p, &x[i * p..(i + 1) * p]);
            d2[i] = d;
            total += d;
        }
        // Sample proportional to D².
        let target = rng.unit() * total;
        let mut acc = 0.0;
        let mut chosen = n - 1;
        for i in 0..n {
            acc += d2[i];
            if acc >= target {
                chosen = i;
                break;
            }
        }
        centroids[c * p..(c + 1) * p].copy_from_slice(&x[chosen * p..(chosen + 1) * p]);
    }
    centroids
}

/// Fit k-means with `k` clusters. Fails closed: `InvalidDimension`,
/// `InsufficientData` (`k == 0` or `k > n`).
pub fn fit(
    x: &[f64],
    n: usize,
    p: usize,
    k: usize,
    max_iter: usize,
    seed: u64,
) -> Result<KMeansModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p {
        return Err(LearningError::InvalidDimension);
    }
    if k == 0 || k > n {
        return Err(LearningError::InsufficientData);
    }

    let mut rng = Lcg(seed ^ 0x9E3779B97F4A7C15);
    let mut centroids = kmeans_pp(x, n, p, k, &mut rng);
    let mut labels = vec![0usize; n];
    let mut converged = false;
    let mut iters = 0;

    for it in 1..=max_iter.max(1) {
        iters = it;
        // Assignment step — assign each point to its nearest centroid.
        //
        // Best-path-with-CPU-floor (mirrors `linear_algebra::gemm`): the point↔centroid
        // squared-distance matrix (`n × k`, the `AllPairs` kernel) is the dominant cost
        // when `n·k·p` is large, so above `GEMM_GPU_THRESHOLD` and with an accelerator
        // present we compute it in one pass via `dispatch::pairwise_sq_dist_f64` (whose
        // cross-term GEMM takes the best path on this machine) and argmin each row. Off
        // accelerator, or sub-threshold, the EXACT per-point `nearest` loop runs —
        // byte-identical to before, including its lowest-index tie-break.
        let mut changed = false;
        // GPU best-path (point↔centroid squared-distance matrix via the forge) only when
        // it's compiled in (native + wgsl-forge). On wasm32 the exact per-point CPU loop runs.
        #[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
        {
            let work = n.saturating_mul(k).saturating_mul(p);
            let caps = crate::wgsl_forge::dispatch::caps();
            if (caps.cuda || caps.wgpu) && work >= crate::wgsl_forge::dispatch::GEMM_GPU_THRESHOLD {
                let dist = crate::wgsl_forge::dispatch::pairwise_sq_dist_f64(x, &centroids, n, k, p);
                for i in 0..n {
                    let row = &dist[i * k..(i + 1) * k];
                    let mut best = 0;
                    let mut best_d = row[0];
                    for (c, &d) in row.iter().enumerate().skip(1) {
                        if d < best_d {
                            best_d = d;
                            best = c;
                        }
                    }
                    if labels[i] != best {
                        labels[i] = best;
                        changed = true;
                    }
                }
            } else {
                for i in 0..n {
                    let (c, _) = nearest(&centroids, k, p, &x[i * p..(i + 1) * p]);
                    if labels[i] != c {
                        labels[i] = c;
                        changed = true;
                    }
                }
            }
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
        {
            for i in 0..n {
                let (c, _) = nearest(&centroids, k, p, &x[i * p..(i + 1) * p]);
                if labels[i] != c {
                    labels[i] = c;
                    changed = true;
                }
            }
        }
        // Update step: centroid = mean of its members; empty clusters keep place.
        let mut sums = vec![0.0; k * p];
        let mut counts = vec![0usize; k];
        for i in 0..n {
            let c = labels[i];
            counts[c] += 1;
            for j in 0..p {
                sums[c * p + j] += x[i * p + j];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..p {
                    centroids[c * p + j] = sums[c * p + j] / counts[c] as f64;
                }
            }
        }
        if !changed && it > 1 {
            converged = true;
            break;
        }
    }

    // Final inertia.
    let mut inertia = 0.0;
    for i in 0..n {
        inertia += sq_dist(
            &centroids[labels[i] * p..(labels[i] + 1) * p],
            &x[i * p..(i + 1) * p],
        );
    }

    Ok(KMeansModel {
        centroids,
        labels,
        inertia,
        k,
        p,
        n_iter: iters,
        converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_three_separated_blobs() {
        // Three tight clusters around (0,0), (10,10), (0,10).
        let mut x = Vec::new();
        let centers = [(0.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        for &(cx, cy) in &centers {
            for d in 0..5 {
                x.push(cx + (d as f64 - 2.0) * 0.1);
                x.push(cy + (d as f64 - 2.0) * 0.1);
            }
        }
        let n = 15;
        let m = fit(&x, n, 2, 3, 100, 1).unwrap();
        assert!(m.converged);
        // Each blob's 5 points share a label.
        for blob in 0..3 {
            let base = blob * 5;
            let l = m.labels[base];
            assert!(
                (base..base + 5).all(|i| m.labels[i] == l),
                "blob {blob} not pure"
            );
        }
        // Three distinct labels used.
        let mut used: Vec<usize> = m.labels.clone();
        used.sort_unstable();
        used.dedup();
        assert_eq!(used.len(), 3);
        // Tight clusters → small inertia.
        assert!(m.inertia < 1.0, "inertia {}", m.inertia);
    }

    #[test]
    fn single_cluster_centroid_is_the_mean() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3 points in 2-D
        let m = fit(&x, 3, 2, 1, 50, 0).unwrap();
        assert!((m.centroids[0] - 3.0).abs() < 1e-9); // mean of x-coords
        assert!((m.centroids[1] - 4.0).abs() < 1e-9); // mean of y-coords
        assert!(m.labels.iter().all(|&l| l == 0));
    }

    #[test]
    fn guards() {
        assert_eq!(
            fit(&[1.0, 2.0], 1, 2, 3, 10, 0).unwrap_err(),
            LearningError::InsufficientData
        );
        assert_eq!(
            fit(&[1.0, 2.0, 3.0], 2, 2, 1, 10, 0).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
