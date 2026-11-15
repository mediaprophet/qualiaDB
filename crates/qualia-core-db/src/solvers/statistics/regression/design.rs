//! Dummy / seasonal design matrices and series transforms.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DummyDesign {
    /// Row-major `n × (n_levels − 1)` (reference = first level dropped).
    pub matrix: Vec<f64>,
    pub n: usize,
    pub n_columns: usize,
    pub level_labels: Vec<String>,
}

/// One-hot dummies with first category as reference (dropped).
pub fn design_dummies(categories: &[u32]) -> Option<DummyDesign> {
    let n = categories.len();
    if n == 0 {
        return None;
    }
    let mut levels: Vec<u32> = categories.to_vec();
    levels.sort_unstable();
    levels.dedup();
    if levels.len() < 2 {
        return None;
    }
    let n_columns = levels.len() - 1;
    let mut matrix = vec![0.0; n * n_columns];
    for (i, &c) in categories.iter().enumerate() {
        if let Some(pos) = levels.iter().position(|&l| l == c) {
            if pos > 0 {
                matrix[i * n_columns + (pos - 1)] = 1.0;
            }
        }
    }
    let level_labels: Vec<String> = levels.iter().map(|l| format!("L{l}")).collect();
    Some(DummyDesign {
        matrix,
        n,
        n_columns,
        level_labels,
    })
}

/// Additive seasonal dummies for period `period` (e.g. 4 or 12); drops season 0.
pub fn seasonal_dummies(n: usize, period: usize) -> Option<DummyDesign> {
    if n == 0 || period < 2 {
        return None;
    }
    let cats: Vec<u32> = (0..n).map(|i| (i % period) as u32).collect();
    design_dummies(&cats)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    Log,
    Log1p,
    Sqrt,
    Square,
    Reciprocal,
    Exp,
}

/// Element-wise transform; `None` if any domain violation (e.g. log of ≤0).
pub fn transform_series(values: &[f64], kind: TransformKind) -> Option<Vec<f64>> {
    let mut out = Vec::with_capacity(values.len());
    for &v in values {
        let t = match kind {
            TransformKind::Log => {
                if v <= 0.0 {
                    return None;
                }
                v.ln()
            }
            TransformKind::Log1p => {
                if v <= -1.0 {
                    return None;
                }
                (1.0 + v).ln()
            }
            TransformKind::Sqrt => {
                if v < 0.0 {
                    return None;
                }
                v.sqrt()
            }
            TransformKind::Square => v * v,
            TransformKind::Reciprocal => {
                if v.abs() < 1e-15 {
                    return None;
                }
                1.0 / v
            }
            TransformKind::Exp => v.exp(),
        };
        if !t.is_finite() {
            return None;
        }
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummies_drop_reference() {
        let cats = [0u32, 1, 0, 2, 1];
        let d = design_dummies(&cats).unwrap();
        assert_eq!(d.n_columns, 2);
        assert_eq!(d.matrix[0], 0.0); // L0
        assert_eq!(d.matrix[2], 1.0); // row1 → L1 → col0
    }

    #[test]
    fn log_rejects_nonpositive() {
        assert!(transform_series(&[1.0, 0.0], TransformKind::Log).is_none());
        let t = transform_series(&[1.0, std::f64::consts::E], TransformKind::Log).unwrap();
        assert!((t[1] - 1.0).abs() < 1e-12);
    }
}
