use crate::NQuin;

/// Many-valued / fuzzy logic over truth degrees in `[0, 1]`. Distinct from the
/// Bayesian `probabilistic` modality: fuzzy conjunction uses a t-norm (not a
/// product), modelling DEGREES of (partial) satisfaction — e.g. a right that is
/// partially fulfilled. Each proposition carries its truth degree as an f32 in the
/// quin `metadata`. Zero-heap throughout.

/// Gödel t-norm (fuzzy AND) — the minimum.
#[inline]
pub fn t_norm_godel(a: f32, b: f32) -> f32 {
    a.min(b)
}

/// Łukasiewicz t-norm (fuzzy AND) — `max(0, a + b - 1)`.
#[inline]
pub fn t_norm_lukasiewicz(a: f32, b: f32) -> f32 {
    (a + b - 1.0).max(0.0)
}

/// Gödel t-conorm (fuzzy OR) — the maximum.
#[inline]
pub fn t_conorm_godel(a: f32, b: f32) -> f32 {
    a.max(b)
}

/// Read a proposition's fuzzy truth degree (f32 in `metadata`), clamped to [0,1].
#[inline]
pub fn degree(quin: &NQuin) -> f32 {
    f32::from_bits(quin.metadata as u32).clamp(0.0, 1.0)
}

/// Fuzzy conjunction (Gödel t-norm = min) of the truth degrees carried by `quins`.
/// Empty input → 1.0 (the t-norm identity). Zero-heap.
pub fn conjunction(quins: &[NQuin]) -> f32 {
    let mut acc = 1.0f32;
    for q in quins {
        acc = t_norm_godel(acc, degree(q));
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_norms_and_conjunction() {
        assert!((t_norm_godel(0.7, 0.4) - 0.4).abs() < 1e-6);
        assert!((t_norm_lukasiewicz(0.7, 0.4) - 0.1).abs() < 1e-6);
        assert!((t_norm_lukasiewicz(0.3, 0.4) - 0.0).abs() < 1e-6);
        assert!((t_conorm_godel(0.7, 0.4) - 0.7).abs() < 1e-6);

        let mk = |d: f32| {
            let mut q = NQuin::default();
            q.metadata = d.to_bits() as u64;
            q
        };
        // min(0.9, 0.6, 0.8) = 0.6
        assert!((conjunction(&[mk(0.9), mk(0.6), mk(0.8)]) - 0.6).abs() < 1e-6);
        assert!((conjunction(&[]) - 1.0).abs() < 1e-6);
    }
}
