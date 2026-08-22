//! Audio invoke seam — spectrum analysis from the time-frequency surface engine.

pub mod dsp;

use super::args;
use crate::audio::tf_surface::TfSurface;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Audio.spectrum` — compute spectral flux and energy from a time-frequency
/// raster. Takes `raster` (list of f32), `frame_count`, `bin_count`,
/// `sample_rate`, and `hop_size`. Returns spectral flux array, total energy,
/// and per-frame energy.
pub fn spectrum(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let raster = args::rec_f64_list(args, "raster")
        .ok_or_else(|| args::bad(span, "Audio.spectrum needs raster"))?;
    let frame_count = args::rec_u64(args, "frame_count")
        .ok_or_else(|| args::bad(span, "Audio.spectrum needs frame_count"))?
        as usize;
    let bin_count = args::rec_u64(args, "bin_count")
        .ok_or_else(|| args::bad(span, "Audio.spectrum needs bin_count"))?
        as usize;
    let sample_rate = args::rec_u64(args, "sample_rate")
        .ok_or_else(|| args::bad(span, "Audio.spectrum needs sample_rate"))?
        as u32;
    let hop_size = args::rec_u64(args, "hop_size")
        .ok_or_else(|| args::bad(span, "Audio.spectrum needs hop_size"))?
        as usize;

    let expected = frame_count * bin_count;
    if raster.len() < expected {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!(
                "Audio.spectrum raster has {} elements, need {} (frame_count * bin_count)",
                raster.len(),
                expected
            ),
        ));
    }

    let raster_f32: Vec<f32> = raster.iter().map(|&x| x as f32).collect();
    let surface = TfSurface::new(&raster_f32, frame_count, bin_count, sample_rate, hop_size);

    let mut flux = vec![0.0f32; frame_count];
    let flux_count = surface.spectral_flux(&mut flux);
    let total_energy = surface.total_energy();

    let mut frame_energies = vec![0.0f32; frame_count];
    for f in 0..frame_count {
        frame_energies[f] = surface.frame_energy(f);
    }

    Ok(args::record([
        (
            "flux",
            args::f64_list_value(flux[..flux_count].iter().map(|&x| x as f64)),
        ),
        ("total_energy", Value::F64(total_energy as f64)),
        (
            "frame_energy",
            args::f64_list_value(frame_energies.iter().map(|&x| x as f64)),
        ),
        ("frame_count", Value::U64(frame_count as u64)),
        ("bin_count", Value::U64(bin_count as u64)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn spectrum_computes_energy() {
        // 4 frames x 3 bins = 12 values
        let raster: Vec<f64> = (0..12).map(|i| (i as f64) * 0.1).collect();
        let mut m = BTreeMap::new();
        m.insert(
            "raster".into(),
            Value::List(raster.into_iter().map(Value::F64).collect()),
        );
        m.insert("frame_count".into(), Value::U64(4));
        m.insert("bin_count".into(), Value::U64(3));
        m.insert("sample_rate".into(), Value::U64(44100));
        m.insert("hop_size".into(), Value::U64(512));
        let result = spectrum(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("flux"));
                assert!(rec.contains_key("total_energy"));
                assert!(rec.contains_key("frame_energy"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn spectrum_rejects_short_raster() {
        let mut m = BTreeMap::new();
        m.insert("raster".into(), Value::List(vec![Value::F64(0.0)]));
        m.insert("frame_count".into(), Value::U64(4));
        m.insert("bin_count".into(), Value::U64(3));
        m.insert("sample_rate".into(), Value::U64(44100));
        m.insert("hop_size".into(), Value::U64(512));
        let result = spectrum(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }
}
