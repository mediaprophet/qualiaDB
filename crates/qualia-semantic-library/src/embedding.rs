//! Embedding storage codec + similarity. Always compiled (the library indexes
//! and searches vectors even when built without the HTTP LLM backend).
//!
//! Vectors are stored in the container's `embeddings/vectors.f32` asset as a
//! flat, row-major little-endian `f32` matrix of shape `rows × dim`.

/// Encode an embedding matrix (`rows × dim`, row-major) to little-endian f32 bytes.
pub fn encode_f32_matrix(rows: &[Vec<f32>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows.iter().map(|r| r.len() * 4).sum());
    for row in rows {
        for &v in row {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// Decode a flat little-endian f32 buffer into `rows × dim`.
pub fn decode_f32_matrix(bytes: &[u8], dim: usize) -> Vec<Vec<f32>> {
    if dim == 0 {
        return Vec::new();
    }
    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    floats.chunks_exact(dim).map(|c| c.to_vec()).collect()
}

/// Cosine similarity in [-1, 1]; 0 if either vector is degenerate or mismatched.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Mean of a set of vectors (centroid). Empty input → empty vector.
pub fn centroid(rows: &[Vec<f32>]) -> Vec<f32> {
    let dim = rows.first().map(|r| r.len()).unwrap_or(0);
    if dim == 0 {
        return Vec::new();
    }
    let mut acc = vec![0.0f32; dim];
    let mut n = 0usize;
    for r in rows {
        if r.len() != dim {
            continue;
        }
        for i in 0..dim {
            acc[i] += r[i];
        }
        n += 1;
    }
    if n == 0 {
        return Vec::new();
    }
    for v in &mut acc {
        *v /= n as f32;
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_round_trips() {
        let m = vec![vec![1.0, 2.0, 3.0], vec![-1.0, 0.5, 0.0]];
        let bytes = encode_f32_matrix(&m);
        let back = decode_f32_matrix(&bytes, 3);
        assert_eq!(m, back);
    }

    #[test]
    fn cosine_basics() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0);
    }
}
