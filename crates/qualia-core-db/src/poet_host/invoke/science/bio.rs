//! Nucleotide Smith–Waterman — `domains::biological::bioinformatics`.

use super::super::args;
use crate::domains::biological::bioinformatics::{
    align_nucleotide, align_protein, build_upgma_tree, kmer_frequencies, minhash_sketch,
    needleman_wunsch, tanimoto_similarity, validate_fasta_record, GapPenalty, PhyloMerge,
    MAX_PHYLO_TAXA,
};
use vibe::{Diagnostic, Span, Value};

pub fn align(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (q, t) = pair_seq(args_v, span)?;
    let r = align_nucleotide(q.as_bytes(), t.as_bytes());
    Ok(args::record([
        ("score", Value::I64(r.score as i64)),
        ("identity_pct", Value::F64(r.identity_pct as f64)),
        ("gaps", Value::U64(r.num_gaps as u64)),
    ]))
}

fn sequence_pair(args_v: &Value, span: Span) -> Result<(String, String), Diagnostic> {
    let first =
        args::rec_str(args_v, "seq1").ok_or_else(|| args::bad(span, "bio operation needs seq1"))?;
    let second =
        args::rec_str(args_v, "seq2").ok_or_else(|| args::bad(span, "bio operation needs seq2"))?;
    Ok((first.to_string(), second.to_string()))
}

fn alignment_value(result: crate::domains::biological::bioinformatics::AlignmentResult) -> Value {
    args::record([
        ("score", Value::I64(result.score as i64)),
        ("identity_pct", Value::F64(result.identity_pct as f64)),
        ("matches", Value::U64(result.num_matches as u64)),
        ("gaps", Value::U64(result.num_gaps as u64)),
        (
            "aligned_query",
            Value::String(String::from_utf8_lossy(&result.aligned_query).into_owned()),
        ),
        (
            "aligned_target",
            Value::String(String::from_utf8_lossy(&result.aligned_target).into_owned()),
        ),
    ])
}

pub fn bio_compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let operation = args::rec_str(args_v, "operation")
        .ok_or_else(|| args::bad(span, "Bioinformatics.compute needs operation"))?;
    match operation {
        "nucleotide_align" => {
            let (query, target) = sequence_pair(args_v, span)?;
            Ok(alignment_value(align_nucleotide(
                query.as_bytes(),
                target.as_bytes(),
            )))
        }
        "protein_align" => {
            let (query, target) = sequence_pair(args_v, span)?;
            Ok(alignment_value(align_protein(
                query.as_bytes(),
                target.as_bytes(),
            )))
        }
        "needleman_wunsch" => {
            let (query, target) = sequence_pair(args_v, span)?;
            Ok(alignment_value(needleman_wunsch(
                query.as_bytes(),
                target.as_bytes(),
                GapPenalty::default(),
                |a, b| {
                    if a.eq_ignore_ascii_case(&b) {
                        2
                    } else {
                        -1
                    }
                },
            )))
        }
        "kmer_frequency" => {
            let sequence = args::rec_str(args_v, "sequence")
                .ok_or_else(|| args::bad(span, "kmer_frequency needs sequence"))?;
            let k = args::rec_u64(args_v, "k").unwrap_or(3).min(32) as usize;
            Ok(Value::List(
                kmer_frequencies(sequence.as_bytes(), k)
                    .into_iter()
                    .take(2048)
                    .map(|(hash, count)| {
                        args::record([
                            ("hash", Value::U64(hash)),
                            ("count", Value::U64(count as u64)),
                        ])
                    })
                    .collect(),
            ))
        }
        "fasta_validate" => {
            let header = args::rec_str(args_v, "header")
                .ok_or_else(|| args::bad(span, "fasta_validate needs header"))?;
            let sequence = args::rec_str(args_v, "sequence")
                .ok_or_else(|| args::bad(span, "fasta_validate needs sequence"))?;
            let result = validate_fasta_record(header, sequence.as_bytes());
            Ok(args::record([
                ("valid", Value::Bool(result.is_valid)),
                ("alphabet", Value::String(format!("{:?}", result.alphabet))),
                ("length", Value::U64(result.sequence.len() as u64)),
                (
                    "invalid_characters",
                    Value::List(
                        result
                            .invalid_chars
                            .into_iter()
                            .map(|character| Value::String(character.to_string()))
                            .collect(),
                    ),
                ),
            ]))
        }
        "gene_expression" => {
            let baseline = args::rec_f64(args_v, "baseline")
                .ok_or_else(|| args::bad(span, "gene_expression needs baseline"))?;
            let treatment = args::rec_f64(args_v, "treatment")
                .ok_or_else(|| args::bad(span, "gene_expression needs treatment"))?;
            let threshold = args::rec_f64(args_v, "threshold").unwrap_or(2.0);
            let gene = args::rec_str(args_v, "gene").unwrap_or("unspecified");
            let result = crate::clinical_engine::evaluate_gene_expression(
                crate::q_hash(gene),
                baseline,
                treatment,
                threshold,
            );
            Ok(args::record([
                ("gene", Value::String(gene.to_string())),
                ("fold_change", Value::F64(result.fold_change)),
                ("log2_fold_change", Value::F64(result.log2_fold_change)),
                ("significant", Value::Bool(result.is_significant)),
                (
                    "direction",
                    Value::String(format!("{:?}", result.direction)),
                ),
            ]))
        }
        "metabolite_similarity" => {
            let first = args::rec_f64_list(args_v, "fingerprint1")
                .ok_or_else(|| args::bad(span, "similarity needs fingerprint1"))?;
            let second = args::rec_f64_list(args_v, "fingerprint2")
                .ok_or_else(|| args::bad(span, "similarity needs fingerprint2"))?;
            if first.len() != second.len() || first.len() > 4096 {
                return Err(args::bad(
                    span,
                    "fingerprints must have equal length of at most 4096 words",
                ));
            }
            let first = first
                .into_iter()
                .map(|value| value as u64)
                .collect::<Vec<_>>();
            let second = second
                .into_iter()
                .map(|value| value as u64)
                .collect::<Vec<_>>();
            Ok(Value::F64(tanimoto_similarity(&first, &second) as f64))
        }
        "minhash" => {
            let sequence = args::rec_str(args_v, "sequence")
                .ok_or_else(|| args::bad(span, "minhash needs sequence"))?;
            let k = args::rec_u64(args_v, "k").unwrap_or(5).min(32) as usize;
            let size = args::rec_u64(args_v, "sketch_size").unwrap_or(64).min(512) as usize;
            Ok(Value::List(
                minhash_sketch(sequence.as_bytes(), k, size)
                    .into_iter()
                    .map(Value::U64)
                    .collect(),
            ))
        }
        "upgma_tree" => {
            let distances = args::rec_f64_list(args_v, "distances")
                .ok_or_else(|| args::bad(span, "upgma_tree needs flattened distances"))?;
            let n = args::rec_u64(args_v, "n").unwrap_or(0) as usize;
            if n < 2 || n > MAX_PHYLO_TAXA || distances.len() != n * n {
                return Err(args::bad(
                    span,
                    format!("UPGMA needs an n×n matrix with 2 <= n <= {MAX_PHYLO_TAXA}"),
                ));
            }
            let distances = distances
                .into_iter()
                .map(|value| value as f32)
                .collect::<Vec<_>>();
            let empty = PhyloMerge {
                cluster_a: 0,
                cluster_b: 0,
                height: 0.0,
                merged_id: 0,
            };
            let mut output = [empty; MAX_PHYLO_TAXA - 1];
            let count = build_upgma_tree(&distances, n, &mut output);
            Ok(Value::List(
                output[..count]
                    .iter()
                    .map(|merge| {
                        args::record([
                            ("cluster_a", Value::U64(merge.cluster_a as u64)),
                            ("cluster_b", Value::U64(merge.cluster_b as u64)),
                            ("height", Value::F64(merge.height as f64)),
                            ("merged_id", Value::U64(merge.merged_id as u64)),
                        ])
                    })
                    .collect(),
            ))
        }
        _ => Err(args::bad(
            span,
            format!("unknown bioinformatics operation `{operation}`"),
        )),
    }
}

fn pair_seq(args_v: &Value, span: Span) -> Result<(String, String), Diagnostic> {
    if let Some(xs) = args::list(args_v) {
        let q = xs
            .first()
            .and_then(args::as_str)
            .ok_or_else(|| args::bad(span, "align needs [query, target]"))?;
        let t = xs
            .get(1)
            .and_then(args::as_str)
            .ok_or_else(|| args::bad(span, "align needs [query, target]"))?;
        return Ok((q.to_string(), t.to_string()));
    }
    let q = args::rec_str(args_v, "query").ok_or_else(|| args::bad(span, "align needs query"))?;
    let t = args::rec_str(args_v, "target").ok_or_else(|| args::bad(span, "align needs target"))?;
    Ok((q.to_string(), t.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_scores_positive() {
        let args = Value::List(vec![
            Value::String("ACGT".into()),
            Value::String("ACGT".into()),
        ]);
        match align(&args, Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("score") {
                Some(Value::I64(s)) => assert!(*s > 0),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn nucleotide_workbench_returns_alignment_strings() {
        let args = Value::Record(std::collections::BTreeMap::from([
            ("operation".into(), Value::String("nucleotide_align".into())),
            ("seq1".into(), Value::String("ACGTACGT".into())),
            ("seq2".into(), Value::String("ACGTTCGT".into())),
        ]));
        let Value::Record(result) = bio_compute(&args, Span::new(0, 0)).unwrap() else {
            panic!("expected alignment record");
        };
        assert!(matches!(result.get("score"), Some(Value::I64(score)) if *score > 0));
        assert!(
            matches!(result.get("aligned_query"), Some(Value::String(value)) if !value.is_empty())
        );
    }
}
