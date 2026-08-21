//! Medical computing invoke seams.
//!
//! Exposes `specialized_libs::medical_computing` functions through VibeScript
//! invoke IDs in the `Medical.*` namespace.

use super::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `Medical.tanimoto` — Tanimoto similarity between two boolean fingerprints.
/// Args: { a: [bool], b: [bool] }
pub fn tanimoto(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_bool_list(args, "a")
        .ok_or_else(|| args::bad(span, "Medical.tanimoto needs a (bool list)"))?;
    let b = args::rec_bool_list(args, "b")
        .ok_or_else(|| args::bad(span, "Medical.tanimoto needs b (bool list)"))?;
    if a.len() != b.len() {
        return Err(args::bad(
            span,
            "Medical.tanimoto: fingerprints must have equal length",
        ));
    }
    let sim = crate::specialized_libs::medical_computing::tanimoto_bits(&a, &b);
    Ok(args::record([("tanimoto_similarity", Value::F64(sim))]))
}

/// `Medical.structural_fingerprint` — compute a structural fingerprint from a SMILES string.
/// Args: { smiles: string }
pub fn structural_fingerprint(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let smiles = args::rec_str(args, "smiles")
        .ok_or_else(|| args::bad(span, "Medical.structural_fingerprint needs smiles"))?;
    let fp = crate::specialized_libs::medical_computing::structural_fingerprint(smiles);
    Ok(args::record([
        (
            "fingerprint",
            Value::List(fp.iter().map(|b| Value::Bool(*b)).collect()),
        ),
        ("bit_count", Value::U64(fp.len() as u64)),
    ]))
}

/// `Medical.analyze_intensity_grid` — DSP analysis of an intensity grid.
/// Args: { data: [f64], width: u64, height: u64, bins: u64, threshold_mode: "otsu"|"fixed", threshold?: f64, window_level?: f64, window_width?: f64 }
pub fn analyze_intensity_grid(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::medical_computing::{
        analyze_intensity_grid as backend, SegmentationThreshold,
    };

    let data = args::rec_f64_list(args, "data")
        .ok_or_else(|| args::bad(span, "Medical.analyze_intensity_grid needs data"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "Medical.analyze_intensity_grid needs width"))?
        as usize;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "Medical.analyze_intensity_grid needs height"))?
        as usize;
    let bins = args::rec_u64(args, "bins")
        .ok_or_else(|| args::bad(span, "Medical.analyze_intensity_grid needs bins"))?
        as usize;
    let threshold_mode = args::rec_str(args, "threshold_mode").unwrap_or("otsu");
    let threshold = if threshold_mode == "fixed" {
        SegmentationThreshold::Fixed(args::rec_f64(args, "threshold").unwrap_or(0.0))
    } else {
        SegmentationThreshold::Otsu
    };
    let window = match (
        args::rec_f64(args, "window_level"),
        args::rec_f64(args, "window_width"),
    ) {
        (Some(level), Some(width)) => Some((level, width)),
        _ => None,
    };

    match backend(&data, width, height, bins, threshold, window) {
        Ok(result) => Ok(args::record([
            (
                "epistemic_status",
                Value::String(result.epistemic_status.to_string()),
            ),
            ("width", Value::U64(result.width as u64)),
            ("height", Value::U64(result.height as u64)),
            ("min", Value::F64(result.min)),
            ("max", Value::F64(result.max)),
            ("mean", Value::F64(result.mean)),
            ("std_dev", Value::F64(result.std_dev)),
            (
                "histogram",
                Value::List(
                    result
                        .histogram
                        .iter()
                        .map(|v| Value::U64(*v as u64))
                        .collect(),
                ),
            ),
            ("threshold", Value::F64(result.threshold)),
            ("segmented_area", Value::U64(result.segmented_area as u64)),
            (
                "segmented_mean_intensity",
                Value::F64(result.segmented_mean_intensity),
            ),
        ])),
        Err(e) => Err(args::bad(span, format!("analyze_intensity_grid: {e:?}"))),
    }
}
