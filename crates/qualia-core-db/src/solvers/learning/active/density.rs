//! Information density — weight raw uncertainty by how *representative* a point is, so
//! the query strategy is not lured into labelling unrepresentative outliers (which are
//! uncertain but teach the model little about the bulk of the data).
//!
//! `density_i = uncertainty_i · ( mean_j similarity(i, j) )^β`. With `β = 0` this
//! reduces to plain uncertainty; larger `β` favours points in dense regions.

use super::{argsort_desc, ActiveError};

/// Cosine similarity of two equal-length feature vectors, in `[-1, 1]`. Zero vectors
/// have undefined direction → similarity `0`.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> Result<f64, ActiveError> {
    if a.len() != b.len() || a.is_empty() {
        return Err(ActiveError::InvalidDimension);
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na <= 0.0 || nb <= 0.0 {
        return Ok(0.0);
    }
    Ok(dot / (na.sqrt() * nb.sqrt()))
}

/// Mean representativeness of each point: the average similarity of point `i` to all
/// *other* points in the pool. `features` is `n_samples × n_features`.
pub fn representativeness(features: &[Vec<f64>]) -> Result<Vec<f64>, ActiveError> {
    let n = features.len();
    if n < 2 {
        return Err(ActiveError::InsufficientData);
    }
    let mut rep = vec![0.0; n];
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..n {
            if i != j {
                sum += cosine_similarity(&features[i], &features[j])?;
            }
        }
        rep[i] = sum / (n - 1) as f64;
    }
    Ok(rep)
}

/// Information-density scores: `uncertainty_i · representativeness_i^β`. Lengths of
/// `uncertainty` and `features` must match. Representativeness is clamped to `≥ 0`
/// before exponentiation (negative mean-similarity points get no density bonus).
pub fn information_density(
    uncertainty: &[f64],
    features: &[Vec<f64>],
    beta: f64,
) -> Result<Vec<f64>, ActiveError> {
    if uncertainty.len() != features.len() {
        return Err(ActiveError::InvalidDimension);
    }
    let rep = representativeness(features)?;
    Ok(uncertainty
        .iter()
        .zip(rep.iter())
        .map(|(&u, &r)| u * r.max(0.0).powf(beta))
        .collect())
}

/// Rank pool indices by information density, most-informative first.
pub fn rank_by_density(
    uncertainty: &[f64],
    features: &[Vec<f64>],
    beta: f64,
) -> Result<Vec<usize>, ActiveError> {
    Ok(argsort_desc(&information_density(
        uncertainty,
        features,
        beta,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn cosine_basic() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]).unwrap() - 1.0).abs() < EPS);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).unwrap().abs() < EPS);
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]).unwrap() + 1.0).abs() < EPS);
        assert!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]).unwrap().abs() < EPS);
        // zero vector
    }

    #[test]
    fn density_demotes_an_uncertain_outlier() {
        // Three points: 0 and 1 form a dense cluster, 2 is an outlier. All equally
        // uncertain. Density should rank the cluster members above the outlier.
        let features = vec![vec![1.0, 0.0], vec![0.96, 0.28], vec![-1.0, 0.0]];
        let uncertainty = vec![0.5, 0.5, 0.5];
        let ranked = rank_by_density(&uncertainty, &features, 1.0).unwrap();
        assert_ne!(ranked[0], 2, "the outlier must not be the top query");
        assert_eq!(
            ranked[2], 2,
            "the outlier ranks last under density weighting"
        );
    }

    #[test]
    fn beta_zero_is_plain_uncertainty() {
        let features = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let uncertainty = vec![0.3, 0.9];
        let d = information_density(&uncertainty, &features, 0.0).unwrap();
        // r^0 = 1 → density == uncertainty.
        assert!((d[0] - 0.3).abs() < EPS && (d[1] - 0.9).abs() < EPS);
    }

    #[test]
    fn fails_closed_on_mismatch() {
        let features = vec![vec![1.0], vec![1.0]];
        assert_eq!(
            information_density(&[0.5], &features, 1.0).unwrap_err(),
            ActiveError::InvalidDimension
        );
    }
}
