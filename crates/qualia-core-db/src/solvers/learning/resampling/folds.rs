//! Resampling index generators — k-fold / LOOCV splits and train/test partition,
//! with a deterministic shuffle so results are reproducible.

/// Deterministic LCG (Numerical Recipes constants) for reproducible shuffles —
/// no RNG dependency.
struct Lcg(u64);
impl Lcg {
    fn next_below(&mut self, bound: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }
}

/// In-place Fisher–Yates shuffle of `idx` seeded by `seed`.
fn shuffle(idx: &mut [usize], seed: u64) {
    let mut rng = Lcg(seed ^ 0x9E3779B97F4A7C15);
    for i in (1..idx.len()).rev() {
        let j = rng.next_below(i + 1);
        idx.swap(i, j);
    }
}

/// One train/test split by row index.
#[derive(Debug, Clone)]
pub struct Fold {
    pub train: Vec<usize>,
    pub test: Vec<usize>,
}

/// `k`-fold splits over `n` rows. `shuffle_rows` randomizes fold membership
/// (seeded); otherwise folds are contiguous blocks. Empty if `k < 2` or `k > n`.
pub fn k_fold(n: usize, k: usize, shuffle_rows: bool, seed: u64) -> Vec<Fold> {
    if k < 2 || k > n {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..n).collect();
    if shuffle_rows {
        shuffle(&mut order, seed);
    }
    // Fold sizes: the first `n % k` folds get one extra element.
    let base = n / k;
    let rem = n % k;
    let mut folds = Vec::with_capacity(k);
    let mut start = 0;
    for f in 0..k {
        let len = base + usize::from(f < rem);
        let test: Vec<usize> = order[start..start + len].to_vec();
        let train: Vec<usize> = order[..start]
            .iter()
            .chain(order[start + len..].iter())
            .copied()
            .collect();
        folds.push(Fold { train, test });
        start += len;
    }
    folds
}

/// Leave-one-out cross-validation = `n`-fold (each test set is a single row).
pub fn loocv(n: usize) -> Vec<Fold> {
    (0..n)
        .map(|i| Fold {
            test: vec![i],
            train: (0..n).filter(|&j| j != i).collect(),
        })
        .collect()
}

/// A single train/test partition: `test_fraction` of the (shuffled) rows go to
/// test. `None` if `test_fraction` is not in `(0,1)` or `n < 2`.
pub fn train_test_split(n: usize, test_fraction: f64, seed: u64) -> Option<(Vec<usize>, Vec<usize>)> {
    if n < 2 || !(0.0..1.0).contains(&test_fraction) || test_fraction <= 0.0 {
        return None;
    }
    let n_test = ((n as f64 * test_fraction).round() as usize).clamp(1, n - 1);
    let mut order: Vec<usize> = (0..n).collect();
    shuffle(&mut order, seed);
    let test = order[..n_test].to_vec();
    let train = order[n_test..].to_vec();
    Some((train, test))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn k_fold_partitions_every_row_once_as_test() {
        let folds = k_fold(10, 3, true, 42);
        assert_eq!(folds.len(), 3);
        let mut all_test: Vec<usize> = folds.iter().flat_map(|f| f.test.iter().copied()).collect();
        all_test.sort_unstable();
        assert_eq!(all_test, (0..10).collect::<Vec<_>>(), "every row is a test exactly once");
        // train and test are disjoint and together cover all rows.
        for f in &folds {
            let t: HashSet<usize> = f.test.iter().copied().collect();
            assert!(f.train.iter().all(|i| !t.contains(i)), "train/test disjoint");
            assert_eq!(f.train.len() + f.test.len(), 10);
        }
    }

    #[test]
    fn k_fold_sizes_balanced() {
        // 10 rows, 3 folds → sizes 4,3,3.
        let folds = k_fold(10, 3, false, 0);
        let mut sizes: Vec<usize> = folds.iter().map(|f| f.test.len()).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![3, 3, 4]);
    }

    #[test]
    fn loocv_has_n_folds_of_one() {
        let folds = loocv(5);
        assert_eq!(folds.len(), 5);
        assert!(folds.iter().all(|f| f.test.len() == 1 && f.train.len() == 4));
    }

    #[test]
    fn train_test_split_sizes_and_disjoint() {
        let (train, test) = train_test_split(100, 0.25, 7).unwrap();
        assert_eq!(test.len(), 25);
        assert_eq!(train.len(), 75);
        let ts: HashSet<usize> = test.iter().copied().collect();
        assert!(train.iter().all(|i| !ts.contains(i)));
        assert!(train_test_split(1, 0.5, 0).is_none());
        assert!(train_test_split(10, 1.5, 0).is_none());
    }

    #[test]
    fn guards() {
        assert!(k_fold(5, 1, false, 0).is_empty());
        assert!(k_fold(3, 5, false, 0).is_empty());
    }
}
