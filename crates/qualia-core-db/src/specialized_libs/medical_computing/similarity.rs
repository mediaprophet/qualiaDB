/// Tanimoto similarity of the overlapping prefix; an empty union returns zero.
pub fn tanimoto_bits(a: &[bool], b: &[bool]) -> f64 {
    let n = a.len().min(b.len());
    let mut intersection = 0usize;
    let mut union = 0usize;
    for i in 0..n {
        if a[i] && b[i] {
            intersection += 1;
        }
        if a[i] || b[i] {
            union += 1;
        }
    }
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::tanimoto_bits;

    #[test]
    fn boolean_similarity_preserves_overlap_and_empty_union() {
        assert_eq!(
            tanimoto_bits(&[true, false, true], &[true, true, false]),
            1.0 / 3.0
        );
        assert_eq!(tanimoto_bits(&[true, true], &[true]), 1.0);
        assert_eq!(tanimoto_bits(&[false], &[false]), 0.0);
        assert_eq!(tanimoto_bits(&[], &[true]), 0.0);
    }
}
