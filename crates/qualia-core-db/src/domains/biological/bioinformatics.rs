//! SIMD-Accelerated Bioinformatics Engine.
//!
//! Production-quality sequence alignment with hardware dispatch:
//! - Smith-Waterman local alignment (affine gap penalties)
//! - Needleman-Wunsch global alignment
//! - BLOSUM62 / nucleotide substitution matrices
//! - K-mer frequency analysis + MinHash sketching
//! - FASTA record validation
//! - Tanimoto metabolite fingerprint similarity
//!
//! `qualia:alignNucleotideSequence` → `align_nucleotide()`
//! `qualia:alignProteinSequence`    → `align_protein()`
//! `qualia:computeKmerFrequency`    → `kmer_frequencies()`
//! `qualia:validateFastaRecord`     → `validate_fasta_record()`
//! `qualia:computeMetaboliteSimilarity` → `tanimoto_similarity()`

#![allow(unused_imports)]
#![allow(unused_unsafe)]

// ─── Core types ───────────────────────────────────────────────────────────────

/// Lightweight backward-compatible score wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentScore {
    pub score: i32,
}

/// Full alignment result with traceback and statistics.
#[derive(Debug, Clone)]
pub struct AlignmentResult {
    /// Smith-Waterman or Needleman-Wunsch score.
    pub score: i32,
    /// Aligned query sequence (gaps encoded as b'-').
    pub aligned_query: Vec<u8>,
    /// Aligned target sequence (gaps encoded as b'-').
    pub aligned_target: Vec<u8>,
    /// Percent identity over the aligned region.
    pub identity_pct: f32,
    pub num_matches: usize,
    pub num_gaps: usize,
}

/// Affine gap penalty model (BLAST defaults: open=-11, extend=-1).
#[derive(Debug, Clone, Copy)]
pub struct GapPenalty {
    pub open: i32,
    pub extend: i32,
}

impl Default for GapPenalty {
    fn default() -> Self {
        Self {
            open: -11,
            extend: -1,
        }
    }
}

/// Nucleotide substitution scores.
#[derive(Debug, Clone, Copy)]
pub struct NucleotideMatrix {
    pub match_score: i32,
    pub mismatch_score: i32,
}

impl Default for NucleotideMatrix {
    fn default() -> Self {
        Self {
            match_score: 2,
            mismatch_score: -3,
        }
    }
}

// ─── BLOSUM62 ─────────────────────────────────────────────────────────────────
// Canonical NCBI BLOSUM62.  Row/column order: A C D E F G H I K L M N P Q R S T V W Y

static BLOSUM62_ORDER: &[u8] = b"ACDEFGHIKLMNPQRSTVWY";

#[rustfmt::skip]
static BLOSUM62: [[i8; 20]; 20] = [
//   A   C   D   E   F   G   H   I   K   L   M   N   P   Q   R   S   T   V   W   Y
    [ 4, -1, -2, -1, -2,  0, -2, -1, -1, -1, -1, -2, -1, -1, -1,  1,  0,  0, -3, -2], // A
    [-1,  9, -3, -4, -2, -3, -3, -1, -3, -1, -1, -3, -3, -3, -3, -1, -1, -1, -2, -2], // C
    [-2, -3,  6,  2, -3, -1, -1, -3, -1, -4, -3,  1, -1,  0, -2,  0, -1, -3, -4, -3], // D
    [-1, -4,  2,  5, -3, -2,  0, -3,  1, -3, -2,  0, -1,  2,  0,  0, -1, -2, -3, -2], // E
    [-2, -2, -3, -3,  6, -3, -1,  0, -3,  0,  0, -3, -4, -3, -3, -2, -2, -1,  1,  3], // F
    [ 0, -3, -1, -2, -3,  6, -2, -4, -2, -4, -3,  0, -2, -2, -2,  0, -2, -3, -2, -3], // G
    [-2, -3, -1,  0, -1, -2,  8, -3, -1, -3, -2,  1, -2,  0,  0, -1, -2, -3, -2,  2], // H
    [-1, -1, -3, -3,  0, -4, -3,  4, -3,  2,  1, -3, -3, -3, -3, -2, -1,  3, -3, -1], // I
    [-1, -3, -1,  1, -3, -2, -1, -3,  5, -2, -1,  0, -1,  1,  2,  0, -1, -2, -3, -2], // K
    [-1, -1, -4, -3,  0, -4, -3,  2, -2,  4,  2, -3, -3, -2, -2, -2, -1,  1, -2, -1], // L
    [-1, -1, -3, -2,  0, -3, -2,  1, -1,  2,  5, -2, -2,  0, -1, -1, -1,  1, -1, -1], // M
    [-2, -3,  1,  0, -3,  0,  1, -3,  0, -3, -2,  6, -2,  0,  0,  1,  0, -3, -4, -2], // N
    [-1, -3, -1, -1, -4, -2, -2, -3, -1, -3, -2, -2,  7, -1, -2, -1, -1, -2, -4, -3], // P
    [-1, -3,  0,  2, -3, -2,  0, -3,  1, -2,  0,  0, -1,  5,  1,  0, -1, -2, -2, -1], // Q
    [-1, -3, -2,  0, -3, -2,  0, -3,  2, -2, -1,  0, -2,  1,  5, -1, -1, -3, -3, -2], // R
    [ 1, -1,  0,  0, -2,  0, -1, -2,  0, -2, -1,  1, -1,  0, -1,  4,  1, -2, -3, -2], // S
    [ 0, -1, -1, -1, -2, -2, -2, -1, -1, -1, -1,  0, -1, -1, -1,  1,  5,  0, -2, -2], // T
    [ 0, -1, -3, -2, -1, -3, -3,  3, -2,  1,  1, -3, -2, -2, -3, -2,  0,  4, -3, -1], // V
    [-3, -2, -4, -3,  1, -2, -2, -3, -3, -2, -1, -4, -4, -2, -3, -3, -2, -3, 11,  2], // W
    [-2, -2, -3, -2,  3, -3,  2, -1, -2, -1, -1, -2, -3, -1, -2, -2, -2, -1,  2,  7], // Y
];

#[inline]
fn blosum62_idx(aa: u8) -> Option<usize> {
    BLOSUM62_ORDER
        .iter()
        .position(|&c| c == aa.to_ascii_uppercase())
}

/// BLOSUM62 score for two amino acid bytes.  Unknown residues → -4.
#[inline]
pub fn blosum62_score(a: u8, b: u8) -> i32 {
    match (blosum62_idx(a), blosum62_idx(b)) {
        (Some(i), Some(j)) => BLOSUM62[i][j] as i32,
        _ => -4,
    }
}

// ─── Smith-Waterman (affine gap) ──────────────────────────────────────────────

/// Maximum sequence length accepted to prevent OOM on untrusted input.
const MAX_SEQ_LEN: usize = 50_000;

/// Smith-Waterman local alignment with affine gap penalties.
/// Allocates O(m×n) DP tables — caller should respect MAX_SEQ_LEN.
pub fn smith_waterman(
    query: &[u8],
    target: &[u8],
    gap: GapPenalty,
    score_fn: impl Fn(u8, u8) -> i32,
) -> AlignmentResult {
    let m = query.len();
    let n = target.len();
    if m == 0 || n == 0 || m > MAX_SEQ_LEN || n > MAX_SEQ_LEN {
        return empty_result();
    }

    let neg_inf = i32::MIN / 2;

    // H: best score ending here; E: gap in target (horizontal); F: gap in query (vertical)
    let mut h = vec![vec![0i32; n + 1]; m + 1];
    let mut e = vec![vec![neg_inf; n + 1]; m + 1];
    let mut f = vec![vec![neg_inf; n + 1]; m + 1];

    // Traceback: 0=stop, 1=diag, 2=left(E), 3=up(F)
    let mut tb = vec![vec![0u8; n + 1]; m + 1];

    let mut best_score = 0i32;
    let mut best_i = 0usize;
    let mut best_j = 0usize;

    for i in 1..=m {
        for j in 1..=n {
            e[i][j] = (h[i][j - 1].saturating_add(gap.open + gap.extend))
                .max(e[i][j - 1].saturating_add(gap.extend));
            f[i][j] = (h[i - 1][j].saturating_add(gap.open + gap.extend))
                .max(f[i - 1][j].saturating_add(gap.extend));

            let diag = h[i - 1][j - 1].saturating_add(score_fn(query[i - 1], target[j - 1]));
            let cell = diag.max(e[i][j]).max(f[i][j]).max(0);
            h[i][j] = cell;

            tb[i][j] = if cell == 0 {
                0
            } else if cell == diag {
                1
            } else if cell == e[i][j] {
                2
            } else {
                3
            };

            if cell > best_score {
                best_score = cell;
                best_i = i;
                best_j = j;
            }
        }
    }

    traceback_local(&h, &tb, query, target, best_i, best_j, best_score)
}

fn traceback_local(
    h: &[Vec<i32>],
    tb: &[Vec<u8>],
    query: &[u8],
    target: &[u8],
    mut i: usize,
    mut j: usize,
    score: i32,
) -> AlignmentResult {
    let mut aq = Vec::new();
    let mut at = Vec::new();
    let mut matches = 0usize;
    let mut gaps = 0usize;

    while i > 0 && j > 0 && h[i][j] > 0 {
        match tb[i][j] {
            1 => {
                aq.push(query[i - 1]);
                at.push(target[j - 1]);
                if query[i - 1].eq_ignore_ascii_case(&target[j - 1]) {
                    matches += 1;
                }
                i -= 1;
                j -= 1;
            }
            2 => {
                aq.push(b'-');
                at.push(target[j - 1]);
                gaps += 1;
                j -= 1;
            }
            3 => {
                aq.push(query[i - 1]);
                at.push(b'-');
                gaps += 1;
                i -= 1;
            }
            _ => break,
        }
    }
    aq.reverse();
    at.reverse();
    let aln_len = aq.len();
    AlignmentResult {
        score,
        identity_pct: if aln_len > 0 {
            100.0 * matches as f32 / aln_len as f32
        } else {
            0.0
        },
        aligned_query: aq,
        aligned_target: at,
        num_matches: matches,
        num_gaps: gaps,
    }
}

fn empty_result() -> AlignmentResult {
    AlignmentResult {
        score: 0,
        aligned_query: vec![],
        aligned_target: vec![],
        identity_pct: 0.0,
        num_matches: 0,
        num_gaps: 0,
    }
}

// ─── Needleman-Wunsch (linear gap) ───────────────────────────────────────────

/// Needleman-Wunsch global alignment with linear gap penalty.
pub fn needleman_wunsch(
    query: &[u8],
    target: &[u8],
    gap: GapPenalty,
    score_fn: impl Fn(u8, u8) -> i32,
) -> AlignmentResult {
    let m = query.len();
    let n = target.len();
    if m == 0 || n == 0 || m > MAX_SEQ_LEN || n > MAX_SEQ_LEN {
        return empty_result();
    }
    let g = gap.open + gap.extend;
    let mut dp = vec![vec![0i32; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i as i32 * g;
    }
    for j in 0..=n {
        dp[0][j] = j as i32 * g;
    }

    for i in 1..=m {
        for j in 1..=n {
            let sub = dp[i - 1][j - 1] + score_fn(query[i - 1], target[j - 1]);
            let del = dp[i - 1][j] + g;
            let ins = dp[i][j - 1] + g;
            dp[i][j] = sub.max(del).max(ins);
        }
    }

    let mut aq = Vec::new();
    let mut at = Vec::new();
    let mut matches = 0usize;
    let mut gaps = 0usize;
    let (mut i, mut j) = (m, n);

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + score_fn(query[i - 1], target[j - 1]) {
            aq.push(query[i - 1]);
            at.push(target[j - 1]);
            if query[i - 1].eq_ignore_ascii_case(&target[j - 1]) {
                matches += 1;
            }
            i -= 1;
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j] == dp[i - 1][j] + g) {
            aq.push(query[i - 1]);
            at.push(b'-');
            gaps += 1;
            i -= 1;
        } else {
            aq.push(b'-');
            at.push(target[j - 1]);
            gaps += 1;
            j -= 1;
        }
    }
    aq.reverse();
    at.reverse();
    let aln_len = aq.len();
    AlignmentResult {
        score: dp[m][n],
        identity_pct: if aln_len > 0 {
            100.0 * matches as f32 / aln_len as f32
        } else {
            0.0
        },
        aligned_query: aq,
        aligned_target: at,
        num_matches: matches,
        num_gaps: gaps,
    }
}

// ─── Convenience entry points ─────────────────────────────────────────────────

/// DNA/RNA Smith-Waterman with BLAST nucleotide defaults.
pub fn align_nucleotide(query: &[u8], target: &[u8]) -> AlignmentResult {
    let mat = NucleotideMatrix::default();
    smith_waterman(query, target, GapPenalty::default(), move |a, b| {
        if a.to_ascii_uppercase() == b.to_ascii_uppercase() {
            mat.match_score
        } else {
            mat.mismatch_score
        }
    })
}

/// Protein Smith-Waterman with BLOSUM62.
pub fn align_protein(query: &[u8], target: &[u8]) -> AlignmentResult {
    smith_waterman(query, target, GapPenalty::default(), blosum62_score)
}

/// Backward-compatible entry point returning the legacy `AlignmentScore`.
pub fn align_sequences(query: &[u8], target: &[u8]) -> AlignmentScore {
    #[cfg(all(feature = "neon_simd_unroll", target_arch = "x86_64"))]
    {
        return simd_align_x86_64(query, target);
    }

    #[cfg(all(feature = "neon_simd_unroll", target_arch = "aarch64"))]
    {
        return simd_align_aarch64(query, target);
    }

    AlignmentScore {
        score: align_nucleotide(query, target).score,
    }
}

// ─── K-mer analysis ───────────────────────────────────────────────────────────

/// Counts all k-mer occurrences; returns (kmer_fnv1a_hash, count) sorted by hash.
pub fn kmer_frequencies(sequence: &[u8], k: usize) -> Vec<(u64, u32)> {
    if k == 0 || k > sequence.len() {
        return vec![];
    }
    let mut counts = std::collections::HashMap::<u64, u32>::new();
    for window in sequence.windows(k) {
        let hash = window.iter().fold(0xcbf29ce484222325u64, |h, &b| {
            (h ^ b.to_ascii_uppercase() as u64).wrapping_mul(0x100000001b3)
        });
        *counts.entry(hash).or_insert(0) += 1;
    }
    let mut out: Vec<(u64, u32)> = counts.into_iter().collect();
    out.sort_unstable_by_key(|&(h, _)| h);
    out
}

/// MinHash sketch: the `sketch_size` smallest k-mer hashes.
pub fn minhash_sketch(sequence: &[u8], k: usize, sketch_size: usize) -> Vec<u64> {
    let mut hashes: Vec<u64> = kmer_frequencies(sequence, k)
        .into_iter()
        .map(|(h, _)| h)
        .collect();
    hashes.sort_unstable();
    hashes.truncate(sketch_size);
    hashes
}

/// Jaccard similarity (0.0–1.0) between two MinHash sketches.
pub fn jaccard_similarity(a: &[u64], b: &[u64]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.iter().filter(|&&x| b.binary_search(&x).is_ok()).count();
    let union = a.len() + b.len() - intersection;
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

// ─── FASTA validation ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceAlphabet {
    DNA,
    RNA,
    Protein,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FastaRecord {
    pub header: String,
    pub sequence: Vec<u8>,
    pub alphabet: SequenceAlphabet,
    pub is_valid: bool,
    pub invalid_chars: Vec<char>,
}

/// Validates and classifies a FASTA record.
pub fn validate_fasta_record(header: &str, sequence: &[u8]) -> FastaRecord {
    let dna_alphabet: &[u8] = b"ACGTNacgtn-";
    let rna_alphabet: &[u8] = b"ACGUNacgun-";
    let protein_alphabet: &[u8] = b"ACDEFGHIKLMNPQRSTVWYXacdefghiklmnpqrstvwyx*-";

    let is_dna = sequence.iter().all(|c| dna_alphabet.contains(c));
    let is_rna = sequence.iter().all(|c| rna_alphabet.contains(c));
    let is_protein = sequence.iter().all(|c| protein_alphabet.contains(c));

    let mut invalid: Vec<char> = sequence
        .iter()
        .filter(|c| !protein_alphabet.contains(c))
        .map(|&c| c as char)
        .collect();
    invalid.dedup();

    let alphabet = if is_dna {
        SequenceAlphabet::DNA
    } else if is_rna {
        SequenceAlphabet::RNA
    } else if is_protein {
        SequenceAlphabet::Protein
    } else {
        SequenceAlphabet::Unknown
    };

    FastaRecord {
        header: header.to_string(),
        sequence: sequence.to_vec(),
        is_valid: invalid.is_empty() && !sequence.is_empty() && !header.is_empty(),
        invalid_chars: invalid,
        alphabet,
    }
}

// ─── Metabolite fingerprint similarity ───────────────────────────────────────

/// Tanimoto (Jaccard) coefficient between two Morgan fingerprints encoded as u64 bitmasks.
/// Multiple words can represent a full extended fingerprint.
#[inline]
pub fn tanimoto_similarity(fp_a: &[u64], fp_b: &[u64]) -> f32 {
    assert_eq!(fp_a.len(), fp_b.len(), "fingerprint lengths must match");
    let intersection: u32 = fp_a
        .iter()
        .zip(fp_b)
        .map(|(a, b)| (a & b).count_ones())
        .sum();
    let union: u32 = fp_a
        .iter()
        .zip(fp_b)
        .map(|(a, b)| (a | b).count_ones())
        .sum();
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Dice coefficient between two binary fingerprints.
#[inline]
pub fn dice_similarity(fp_a: &[u64], fp_b: &[u64]) -> f32 {
    assert_eq!(fp_a.len(), fp_b.len());
    let intersection: u32 = fp_a
        .iter()
        .zip(fp_b)
        .map(|(a, b)| (a & b).count_ones())
        .sum();
    let sum_a: u32 = fp_a.iter().map(|a| a.count_ones()).sum();
    let sum_b: u32 = fp_b.iter().map(|b| b.count_ones()).sum();
    if sum_a + sum_b == 0 {
        1.0
    } else {
        2.0 * intersection as f32 / (sum_a + sum_b) as f32
    }
}

// ─── SIMD fast-paths (feature-gated) ───────────────────────────────────

#[cfg(all(feature = "neon_simd_unroll", target_arch = "x86_64"))]
pub fn simd_align_x86_64(query: &[u8], target: &[u8]) -> AlignmentScore {
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        let min_len = query.len().min(target.len());
        let mut score = 0i32;
        let mut i = 0;

        // Exact match fast-path using AVX2 (256-bit / 32-byte chunks)
        while i + 32 <= min_len {
            let q_vec = _mm256_loadu_si256(query.as_ptr().add(i) as *const __m256i);
            let t_vec = _mm256_loadu_si256(target.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(q_vec, t_vec);
            let mask = _mm256_movemask_epi8(cmp);
            let matches = mask.count_ones() as i32;
            let mismatches = 32 - matches;

            score += matches * 2; // match score
            score -= mismatches * 3; // mismatch score
            i += 32;
        }

        if i < min_len {
            score += align_nucleotide(&query[i..], &target[i..]).score;
        }
        return AlignmentScore { score };
    }

    #[cfg(not(target_feature = "avx2"))]
    AlignmentScore {
        score: align_nucleotide(query, target).score,
    }
}

#[cfg(all(feature = "neon_simd_unroll", target_arch = "aarch64"))]
pub fn simd_align_aarch64(query: &[u8], target: &[u8]) -> AlignmentScore {
    #[cfg(target_feature = "neon")]
    unsafe {
        use std::arch::aarch64::*;
        let min_len = query.len().min(target.len());
        let mut score = 0i32;
        let mut i = 0;

        // Exact match fast-path using NEON (128-bit / 16-byte chunks)
        while i + 16 <= min_len {
            let q_vec = vld1q_u8(query.as_ptr().add(i));
            let t_vec = vld1q_u8(target.as_ptr().add(i));
            let cmp = vceqq_u8(q_vec, t_vec);

            let mut v = [0u8; 16];
            vst1q_u8(v.as_mut_ptr(), cmp);
            let mut matches = 0;
            for &b in &v {
                if b == 0xFF {
                    matches += 1;
                }
            }
            let mismatches = 16 - matches;

            score += matches * 2;
            score -= mismatches * 3;
            i += 16;
        }

        if i < min_len {
            score += align_nucleotide(&query[i..], &target[i..]).score;
        }
        return AlignmentScore { score };
    }

    #[cfg(not(target_feature = "neon"))]
    AlignmentScore {
        score: align_nucleotide(query, target).score,
    }
}

// ─── DNA to Protein Translation ──────────────────────────────────────────────

/// Translates a DNA sequence into an amino acid sequence using the standard genetic code.
/// Writes directly into the caller-provided `out` buffer to avoid allocation.
/// Returns the number of amino acids written.
pub fn translate_dna_to_protein(dna: &[u8], out: &mut [u8]) -> usize {
    let mut written = 0;
    for i in (0..dna.len()).step_by(3) {
        if i + 2 >= dna.len() {
            break;
        }
        if written >= out.len() {
            break;
        }

        let codon = (
            dna[i].to_ascii_uppercase(),
            dna[i + 1].to_ascii_uppercase(),
            dna[i + 2].to_ascii_uppercase(),
        );
        let aa = match codon {
            (b'G', b'C', _) => b'A',                         // Alanine
            (b'T', b'G', b'C') | (b'T', b'G', b'T') => b'C', // Cysteine
            (b'G', b'A', b'C') | (b'G', b'A', b'T') => b'D', // Aspartic Acid
            (b'G', b'A', b'A') | (b'G', b'A', b'G') => b'E', // Glutamic Acid
            (b'T', b'T', b'C') | (b'T', b'T', b'T') => b'F', // Phenylalanine
            (b'G', b'G', _) => b'G',                         // Glycine
            (b'C', b'A', b'C') | (b'C', b'A', b'T') => b'H', // Histidine
            (b'A', b'T', b'C') | (b'A', b'T', b'T') | (b'A', b'T', b'A') => b'I', // Isoleucine
            (b'A', b'A', b'A') | (b'A', b'A', b'G') => b'K', // Lysine
            (b'C', b'T', _) | (b'T', b'T', b'A') | (b'T', b'T', b'G') => b'L', // Leucine
            (b'A', b'T', b'G') => b'M',                      // Methionine (Start)
            (b'A', b'A', b'C') | (b'A', b'A', b'T') => b'N', // Asparagine
            (b'C', b'C', _) => b'P',                         // Proline
            (b'C', b'A', b'A') | (b'C', b'A', b'G') => b'Q', // Glutamine
            (b'C', b'G', _) | (b'A', b'G', b'A') | (b'A', b'G', b'G') => b'R', // Arginine
            (b'T', b'C', _) | (b'A', b'G', b'C') | (b'A', b'G', b'T') => b'S', // Serine
            (b'A', b'C', _) => b'T',                         // Threonine
            (b'G', b'T', _) => b'V',                         // Valine
            (b'T', b'G', b'G') => b'W',                      // Tryptophan
            (b'T', b'A', b'C') | (b'T', b'A', b'T') => b'Y', // Tyrosine
            (b'T', b'A', b'A') | (b'T', b'A', b'G') | (b'T', b'G', b'A') => b'*', // Stop
            _ => b'X',                                       // Unknown
        };
        out[written] = aa;
        written += 1;
    }
    written
}

// ─── Protein Isoelectric Point ───────────────────────────────────────────────

/// Estimates the Isoelectric Point (pI) of a protein sequence using
/// the Henderson-Hasselbalch equation and basic pKa values.
pub fn calculate_isoelectric_point(protein: &[u8]) -> f64 {
    let c_term = 1; // Alpha-COOH
    let n_term = 1; // Alpha-NH2
    let mut d = 0; // Aspartic acid (D)
    let mut e = 0; // Glutamic acid (E)
    let mut c = 0; // Cysteine (C)
    let mut y = 0; // Tyrosine (Y)
    let mut h = 0; // Histidine (H)
    let mut k = 0; // Lysine (K)
    let mut r = 0; // Arginine (R)

    for &aa in protein {
        match aa.to_ascii_uppercase() {
            b'D' => d += 1,
            b'E' => e += 1,
            b'C' => c += 1,
            b'Y' => y += 1,
            b'H' => h += 1,
            b'K' => k += 1,
            b'R' => r += 1,
            _ => {}
        }
    }

    // pKa values (Bjellqvist etc. simplified)
    let pka_c_term = 3.65;
    let pka_d = 3.90;
    let pka_e = 4.07;
    let pka_c = 8.18;
    let pka_y = 10.46;

    let pka_n_term = 8.20;
    let pka_h = 6.04;
    let pka_k = 10.53;
    let pka_r = 12.48;

    let net_charge = |ph: f64| -> f64 {
        let neg = (c_term as f64) / (1.0 + 10.0_f64.powf(pka_c_term - ph))
            + (d as f64) / (1.0 + 10.0_f64.powf(pka_d - ph))
            + (e as f64) / (1.0 + 10.0_f64.powf(pka_e - ph))
            + (c as f64) / (1.0 + 10.0_f64.powf(pka_c - ph))
            + (y as f64) / (1.0 + 10.0_f64.powf(pka_y - ph));

        let pos = (n_term as f64) / (1.0 + 10.0_f64.powf(ph - pka_n_term))
            + (h as f64) / (1.0 + 10.0_f64.powf(ph - pka_h))
            + (k as f64) / (1.0 + 10.0_f64.powf(ph - pka_k))
            + (r as f64) / (1.0 + 10.0_f64.powf(ph - pka_r));

        pos - neg
    };

    // Bisection method to find pH where net_charge == 0
    let mut low = 0.0;
    let mut high = 14.0;
    for _ in 0..50 {
        let mid = (low + high) / 2.0;
        let charge = net_charge(mid);
        if charge > 0.0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

// ─── Peptide Cleavage Prediction ─────────────────────────────────────────────

/// Predicts cleavage sites for Trypsin (cleaves after K or R, unless followed by P).
/// Writes the indices of cleavage into the caller-provided `out_indices` buffer.
/// Returns the number of cleavage sites found.
pub fn predict_peptide_cleavage(protein: &[u8], out_indices: &mut [usize]) -> usize {
    let mut count = 0;
    for i in 0..protein.len() {
        if count >= out_indices.len() {
            break;
        }
        let aa = protein[i].to_ascii_uppercase();
        if aa == b'K' || aa == b'R' {
            if i + 1 < protein.len() && protein[i + 1].to_ascii_uppercase() == b'P' {
                continue; // Trypsin generally doesn't cleave if followed by Proline
            }
            out_indices[count] = i;
            count += 1;
        }
    }
    count
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sw_identical_nucleotide() {
        let r = align_nucleotide(b"ACGTACGT", b"ACGTACGT");
        assert!(r.score > 0);
        assert!((r.identity_pct - 100.0).abs() < 0.01);
    }

    #[test]
    fn sw_one_mismatch() {
        let r = align_nucleotide(b"ACGTACGT", b"ACGTCCGT");
        assert!(r.score > 0);
        assert!(r.identity_pct > 80.0);
    }

    #[test]
    fn blosum62_diagonal_positive() {
        for aa in b"ACDEFGHIKLMNPQRSTVWY" {
            assert!(
                blosum62_score(*aa, *aa) > 0,
                "diagonal should be positive for {}",
                *aa as char
            );
        }
    }

    #[test]
    fn blosum62_w_max_diagonal() {
        assert_eq!(blosum62_score(b'W', b'W'), 11);
    }

    #[test]
    fn protein_align_identical() {
        let r = align_protein(b"ACDEFGHIK", b"ACDEFGHIK");
        assert!(r.score > 0);
        assert!((r.identity_pct - 100.0).abs() < 0.01);
    }

    #[test]
    fn nw_global_fills_gaps() {
        let mat = NucleotideMatrix::default();
        let r = needleman_wunsch(b"ACGT", b"ACGTTTT", GapPenalty::default(), move |a, b| {
            if a.to_ascii_uppercase() == b.to_ascii_uppercase() {
                mat.match_score
            } else {
                mat.mismatch_score
            }
        });
        assert_eq!(r.aligned_query.len(), r.aligned_target.len());
    }

    #[test]
    fn kmer_frequency_counts() {
        let f = kmer_frequencies(b"ATCGATCG", 3);
        assert!(!f.is_empty());
        let total: u32 = f.iter().map(|&(_, c)| c).sum();
        assert_eq!(total, 6); // 8 - 3 + 1 = 6 tri-mers
    }

    #[test]
    fn fasta_dna_valid() {
        let r = validate_fasta_record(">seq1", b"ATCGATCG");
        assert_eq!(r.alphabet, SequenceAlphabet::DNA);
        assert!(r.is_valid);
    }

    #[test]
    fn invalid_fasta_alphabets() {
        let rec = validate_fasta_record(">test", b"ATCGXZATCG");
        assert!(!rec.is_valid);
    }

    #[test]
    fn test_translate_dna() {
        let dna = b"ATGGCCATTGTAATGGGCCGCTGAAAGGGTGCCCGATAG";
        let mut out = [0u8; 100];
        let n = translate_dna_to_protein(dna, &mut out);
        assert_eq!(&out[..n], b"MAIVMGR*KGAR*");
    }

    #[test]
    fn test_isoelectric_point() {
        let protein = b"MGRKGAR"; // basic protein, expect high pI
        let pi = calculate_isoelectric_point(protein);
        assert!(pi > 10.0, "pI was {}", pi);
    }

    #[test]
    fn test_peptide_cleavage() {
        let protein = b"MGRKGAR"; // R at 2, K at 3, R at 6
        let mut out = [0usize; 10];
        let n = predict_peptide_cleavage(protein, &mut out);
        assert_eq!(n, 3);
        assert_eq!(&out[..n], &[2, 3, 6]);
    }

    #[test]
    fn fasta_invalid_chars() {
        let r = validate_fasta_record(">seq2", b"ATCG123");
        assert!(!r.is_valid);
        assert!(!r.invalid_chars.is_empty());
    }

    #[test]
    fn tanimoto_identical() {
        let fp = vec![0xFFFFFFFFFFFFFFFFu64; 2];
        assert!((tanimoto_similarity(&fp, &fp) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tanimoto_disjoint() {
        let a = vec![0x00000000FFFFFFFFu64];
        let b = vec![0xFFFFFFFF00000000u64];
        assert!((tanimoto_similarity(&a, &b)).abs() < 1e-6);
    }
}
