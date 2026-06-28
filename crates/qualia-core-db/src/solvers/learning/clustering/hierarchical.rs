//! Agglomerative hierarchical clustering (ISL ch 12.4.2) — bottom-up merging of the
//! two closest clusters under a linkage rule, producing a dendrogram that can be cut
//! into any number of clusters. Kernel-class `AllPairs` (the cluster distances).

use crate::solvers::learning::LearningError;

/// How the distance between two clusters is defined from member point distances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    /// Nearest pair (min).
    Single,
    /// Farthest pair (max).
    Complete,
    /// Mean over all cross pairs.
    Average,
}

/// A fitted dendrogram: the sequence of `n−1` merges, each recorded as two
/// representative original-point indices.
#[derive(Debug, Clone)]
pub struct Hierarchical {
    merges: Vec<(usize, usize)>,
    n: usize,
}

fn sq_dist(x: &[f64], p: usize, i: usize, j: usize) -> f64 {
    let a = &x[i * p..(i + 1) * p];
    let b = &x[j * p..(j + 1) * p];
    a.iter().zip(b).map(|(u, v)| (u - v) * (u - v)).sum()
}

fn cluster_distance(x: &[f64], p: usize, ca: &[usize], cb: &[usize], linkage: Linkage) -> f64 {
    let mut acc = match linkage {
        Linkage::Single => f64::INFINITY,
        Linkage::Complete => f64::NEG_INFINITY,
        Linkage::Average => 0.0,
    };
    for &i in ca {
        for &j in cb {
            let d = sq_dist(x, p, i, j);
            match linkage {
                Linkage::Single => acc = acc.min(d),
                Linkage::Complete => acc = acc.max(d),
                Linkage::Average => acc += d,
            }
        }
    }
    if linkage == Linkage::Average {
        acc /= (ca.len() * cb.len()) as f64;
    }
    acc
}

impl Hierarchical {
    /// Build the full dendrogram for a row-major `n × p` matrix under `linkage`.
    pub fn fit(x: &[f64], n: usize, p: usize, linkage: Linkage) -> Result<Self, LearningError> {
        if n == 0 || p == 0 || x.len() != n * p {
            return Err(LearningError::InvalidDimension);
        }
        let mut clusters: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
        let mut merges = Vec::with_capacity(n.saturating_sub(1));
        while clusters.len() > 1 {
            // Find the closest pair of active clusters.
            let mut best = f64::INFINITY;
            let (mut bi, mut bj) = (0, 1);
            for a in 0..clusters.len() {
                for b in (a + 1)..clusters.len() {
                    let d = cluster_distance(x, p, &clusters[a], &clusters[b], linkage);
                    if d < best {
                        best = d;
                        bi = a;
                        bj = b;
                    }
                }
            }
            // Record the merge by a representative point from each cluster.
            merges.push((clusters[bi][0], clusters[bj][0]));
            // Merge bj into bi, then drop bj.
            let moved = clusters.remove(bj);
            clusters[bi].extend(moved);
        }
        Ok(Self { merges, n })
    }

    /// Cut the dendrogram into `k` clusters and return a label per original point
    /// (labels are `0..k`, assigned in first-appearance order). `k` is clamped to
    /// `1..=n`.
    pub fn labels(&self, k: usize) -> Vec<usize> {
        let k = k.clamp(1, self.n);
        // Union-find over the first (n − k) merges.
        let mut parent: Vec<usize> = (0..self.n).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let n_unions = self.n.saturating_sub(k);
        for &(a, b) in self.merges.iter().take(n_unions) {
            let ra = find(&mut parent, a);
            let rb = find(&mut parent, b);
            if ra != rb {
                parent[ra] = rb;
            }
        }
        // Relabel roots to 0..k in first-appearance order.
        let mut label_of = std::collections::HashMap::new();
        let mut next = 0;
        let mut out = vec![0usize; self.n];
        for i in 0..self.n {
            let r = find(&mut parent, i);
            let l = *label_of.entry(r).or_insert_with(|| {
                let v = next;
                next += 1;
                v
            });
            out[i] = l;
        }
        out
    }

    pub fn n_merges(&self) -> usize {
        self.merges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_two_obvious_groups() {
        // Two tight groups far apart → cutting into 2 clusters splits them.
        let x = [
            0.0, 0.0, 0.2, 0.1, -0.1, 0.2, 10.0, 10.0, 10.2, 9.9, 9.8, 10.1,
        ];
        let h = Hierarchical::fit(&x, 6, 2, Linkage::Average).unwrap();
        assert_eq!(h.n_merges(), 5);
        let labels = h.labels(2);
        // First 3 share a label; last 3 share the other.
        assert!(labels[0] == labels[1] && labels[1] == labels[2]);
        assert!(labels[3] == labels[4] && labels[4] == labels[5]);
        assert_ne!(labels[0], labels[3]);
    }

    #[test]
    fn k_equals_n_is_all_singletons_and_k_one_is_all_together() {
        let x = [0.0, 1.0, 2.0, 3.0]; // 4 points in 1-D
        let h = Hierarchical::fit(&x, 4, 1, Linkage::Single).unwrap();
        let singletons = h.labels(4);
        let mut sorted = singletons.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 4); // every point its own cluster
        let one = h.labels(1);
        assert!(one.iter().all(|&l| l == 0)); // all together
    }

    #[test]
    fn complete_and_single_linkage_both_run() {
        let x = [0.0, 0.0, 1.0, 1.0, 5.0, 5.0, 6.0, 6.0];
        for linkage in [Linkage::Single, Linkage::Complete, Linkage::Average] {
            let h = Hierarchical::fit(&x, 4, 2, linkage).unwrap();
            let labels = h.labels(2);
            assert_eq!(labels[0], labels[1]);
            assert_eq!(labels[2], labels[3]);
            assert_ne!(labels[0], labels[2]);
        }
    }

    #[test]
    fn guards() {
        assert_eq!(
            Hierarchical::fit(&[1.0, 2.0, 3.0], 2, 2, Linkage::Single).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
