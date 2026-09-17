//! Bounded HU window/level adapter for POET's lightweight DICOM panel.

use super::super::args;
use crate::specialized_libs::computer_vision::bio::apply_hu_window_f32;
use vibe::{Diagnostic, Span, Value};

const MAX_SLICE_PIXELS: usize = 65_536;

pub fn hu_window(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let study_uid = args::rec_str(args_v, "study_uid")
        .ok_or_else(|| args::bad(span, "HU windowing needs `study_uid`"))?;
    let width = args::rec_u64(args_v, "width")
        .ok_or_else(|| args::bad(span, "HU windowing needs `width`"))? as usize;
    let height = args::rec_u64(args_v, "height")
        .ok_or_else(|| args::bad(span, "HU windowing needs `height`"))? as usize;
    let pixels = args::rec_f64_list(args_v, "pixels")
        .ok_or_else(|| args::bad(span, "HU windowing needs `pixels=[...]`"))?;
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| args::bad(span, "slice dimensions overflow"))?;
    if width == 0 || height == 0 || expected > MAX_SLICE_PIXELS || pixels.len() != expected {
        return Err(args::bad(
            span,
            "slice must contain width*height pixels with at most 65,536 samples",
        ));
    }
    if pixels.iter().any(|pixel| !pixel.is_finite()) {
        return Err(args::bad(span, "HU pixels must be finite"));
    }
    let window = args::rec_f64(args_v, "window")
        .ok_or_else(|| args::bad(span, "HU windowing needs positive `window`"))?;
    let level = args::rec_f64(args_v, "level")
        .ok_or_else(|| args::bad(span, "HU windowing needs finite `level`"))?;
    if !window.is_finite() || window <= 0.0 || !level.is_finite() {
        return Err(args::bad(span, "window must be positive and level finite"));
    }
    let samples = pixels.iter().map(|value| *value as f32).collect::<Vec<_>>();
    let mut grayscale = vec![0u8; expected];
    apply_hu_window_f32(&samples, window as f32, level as f32, &mut grayscale)
        .map_err(|error| args::bad(span, format!("HU windowing failed: {error}")))?;
    let min = grayscale.iter().copied().min().unwrap_or(0);
    let max = grayscale.iter().copied().max().unwrap_or(0);
    Ok(args::record([
        ("study_uid", Value::String(study_uid.to_string())),
        ("width", Value::U64(width as u64)),
        ("height", Value::U64(height as u64)),
        ("window", Value::F64(window)),
        ("level", Value::F64(level)),
        ("display_min", Value::U64(min as u64)),
        ("display_max", Value::U64(max as u64)),
        (
            "grayscale",
            Value::List(
                grayscale
                    .into_iter()
                    .map(|value| Value::U64(value as u64))
                    .collect(),
            ),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_real_hu_samples() {
        let input = args::record([
            ("study_uid", Value::String("1.2.3".into())),
            ("width", Value::U64(2)),
            ("height", Value::U64(2)),
            (
                "pixels",
                Value::List(
                    vec![-160.0, 40.0, 240.0, 1_000.0]
                        .into_iter()
                        .map(Value::F64)
                        .collect(),
                ),
            ),
            ("window", Value::F64(400.0)),
            ("level", Value::F64(40.0)),
        ]);
        let Value::Record(result) = hu_window(&input, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert_eq!(args::as_u64(result.get("display_min").unwrap()), Some(0));
        assert_eq!(args::as_u64(result.get("display_max").unwrap()), Some(255));
    }
}
