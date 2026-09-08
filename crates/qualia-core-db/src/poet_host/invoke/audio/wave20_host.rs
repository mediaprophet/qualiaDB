//! Wave-20 Host binds: Audio DSP / TF scalar maps.
//!
//! Pure CPU paths from `audio::dsp_kernel` and `audio::tf_surface` —
//! no forge / `caps()` / CUDA.

use super::super::args;
use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;
use crate::audio::dsp_kernel::{
    epistemic_fm_index, parametric_sample, sigma_dominant_frequency, ParametricVoiceState,
};
use crate::audio::tf_surface::TfSurface;
use vibe::{Diagnostic, Span, Value};

/// `Audio.epistemic_fm_index` — FM index from epistemic `q` and carrier `μ`.
/// Args: `{ q: f64, mu: f64 }`. Out: `{ fm_index: f64 }`.
pub fn epistemic_fm_index_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::rec_f64(args_v, "q")
        .ok_or_else(|| args::bad(span, "Audio.epistemic_fm_index needs q"))?;
    let mu = args::rec_f64(args_v, "mu")
        .ok_or_else(|| args::bad(span, "Audio.epistemic_fm_index needs mu"))?;
    let idx = epistemic_fm_index(q as f32, mu as f32) as f64;
    Ok(args::record([("fm_index", Value::F64(idx))]))
}

/// `Audio.sigma_dominant_frequency` — map σ preview bins → fundamental Hz.
/// Args: `{ bins: [f64] (len=64), base_hz?: f64 }`. Out: `{ frequency_hz: f64 }`.
pub fn sigma_dominant_frequency_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let bins_in = args::rec_f64_list(args_v, "bins")
        .ok_or_else(|| args::bad(span, "Audio.sigma_dominant_frequency needs bins: [f64]"))?;
    if bins_in.len() != SPECTRAL_PREVIEW_BINS {
        return Err(args::bad(
            span,
            format!(
                "Audio.sigma_dominant_frequency needs exactly {SPECTRAL_PREVIEW_BINS} bins, got {}",
                bins_in.len()
            ),
        ));
    }
    let mut bins = [0.0_f32; SPECTRAL_PREVIEW_BINS];
    for (i, &v) in bins_in.iter().enumerate() {
        bins[i] = v as f32;
    }
    let base_hz = args::rec_f64(args_v, "base_hz").unwrap_or(220.0) as f32;
    let hz = sigma_dominant_frequency(&bins, base_hz) as f64;
    Ok(args::record([("frequency_hz", Value::F64(hz))]))
}

/// `Audio.parametric_sample` — one sample from a parametric voice state.
/// Args: `{ phase, frequency_hz, gain, fm_index, sample_rate?: f64 }`.
/// Out: `{ sample, phase, frequency_hz, gain, fm_index }`.
pub fn parametric_sample_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let phase = args::rec_f64(args_v, "phase")
        .ok_or_else(|| args::bad(span, "Audio.parametric_sample needs phase"))?
        as f32;
    let frequency_hz = args::rec_f64(args_v, "frequency_hz")
        .ok_or_else(|| args::bad(span, "Audio.parametric_sample needs frequency_hz"))?
        as f32;
    let gain = args::rec_f64(args_v, "gain")
        .ok_or_else(|| args::bad(span, "Audio.parametric_sample needs gain"))?
        as f32;
    let fm_index = args::rec_f64(args_v, "fm_index").unwrap_or(0.0) as f32;
    let sample_rate = args::rec_f64(args_v, "sample_rate").unwrap_or(48_000.0) as f32;
    let mut state = ParametricVoiceState {
        phase,
        frequency_hz,
        gain,
        fm_index,
    };
    let sample = parametric_sample(&mut state, sample_rate) as f64;
    Ok(args::record([
        ("sample", Value::F64(sample)),
        ("phase", Value::F64(state.phase as f64)),
        ("frequency_hz", Value::F64(state.frequency_hz as f64)),
        ("gain", Value::F64(state.gain as f64)),
        ("fm_index", Value::F64(state.fm_index as f64)),
    ]))
}

/// `Audio.bin_to_freq_linear` — STFT bin → Hz.
/// Args: `{ bin: u64, sample_rate: u64, bin_count: u64 }`. Out: `{ frequency_hz: f64 }`.
pub fn bin_to_freq_linear_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let bin = args::rec_u64(args_v, "bin")
        .ok_or_else(|| args::bad(span, "Audio.bin_to_freq_linear needs bin"))?
        as usize;
    let sample_rate = args::rec_u64(args_v, "sample_rate")
        .ok_or_else(|| args::bad(span, "Audio.bin_to_freq_linear needs sample_rate"))?
        as u32;
    let bin_count = args::rec_u64(args_v, "bin_count")
        .ok_or_else(|| args::bad(span, "Audio.bin_to_freq_linear needs bin_count"))?
        as usize;
    // Empty raster / zero frames — only sample_rate / bin_count are used.
    let raster: &[f32] = &[];
    let surface = TfSurface::new(raster, 0, bin_count, sample_rate, 1);
    let hz = surface.bin_to_freq_linear(bin) as f64;
    Ok(args::record([("frequency_hz", Value::F64(hz))]))
}

/// `Audio.bin_to_freq_log` — CQT-style log bin → Hz.
/// Args: `{ bin: u64, f_min: f64, bins_per_octave: u64 }`. Out: `{ frequency_hz: f64 }`.
pub fn bin_to_freq_log_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let bin = args::rec_u64(args_v, "bin")
        .ok_or_else(|| args::bad(span, "Audio.bin_to_freq_log needs bin"))?
        as usize;
    let f_min = args::rec_f64(args_v, "f_min")
        .ok_or_else(|| args::bad(span, "Audio.bin_to_freq_log needs f_min"))?
        as f32;
    let bins_per_octave = args::rec_u64(args_v, "bins_per_octave")
        .ok_or_else(|| args::bad(span, "Audio.bin_to_freq_log needs bins_per_octave"))?
        as usize;
    let raster: &[f32] = &[];
    let surface = TfSurface::new(raster, 0, 0, 0, 0);
    let hz = surface.bin_to_freq_log(bin, f_min, bins_per_octave) as f64;
    Ok(args::record([("frequency_hz", Value::F64(hz))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave20_epistemic_fm_index_zero() {
        let mut m = BTreeMap::new();
        m.insert("q".into(), Value::F64(0.0));
        m.insert("mu".into(), Value::F64(0.0));
        let out = epistemic_fm_index_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_f64(&out, "fm_index").unwrap(), 0.0);
    }

    #[test]
    fn wave20_epistemic_fm_index_clamped() {
        let mut m = BTreeMap::new();
        m.insert("q".into(), Value::F64(2.0));
        m.insert("mu".into(), Value::F64(20.0));
        let out = epistemic_fm_index_host(&Value::Record(m), span()).unwrap();
        let idx = args::rec_f64(&out, "fm_index").unwrap();
        assert!((idx - 8.0).abs() < 1e-6);
    }

    #[test]
    fn wave20_sigma_dominant_frequency_audible() {
        let mut bins = vec![0.0_f64; SPECTRAL_PREVIEW_BINS];
        bins[32] = 1.0;
        let mut m = BTreeMap::new();
        m.insert("bins".into(), args::f64_list_value(bins));
        m.insert("base_hz".into(), Value::F64(220.0));
        let out = sigma_dominant_frequency_host(&Value::Record(m), span()).unwrap();
        let hz = args::rec_f64(&out, "frequency_hz").unwrap();
        assert!(hz >= 55.0 && hz <= 8_000.0);
    }

    #[test]
    fn wave20_parametric_sample_bounded() {
        let mut m = BTreeMap::new();
        m.insert("phase".into(), Value::F64(0.0));
        m.insert("frequency_hz".into(), Value::F64(440.0));
        m.insert("gain".into(), Value::F64(1.0));
        m.insert("fm_index".into(), Value::F64(0.0));
        m.insert("sample_rate".into(), Value::F64(48_000.0));
        let out = parametric_sample_host(&Value::Record(m), span()).unwrap();
        let s = args::rec_f64(&out, "sample").unwrap();
        assert!((-1.0..=1.0).contains(&s));
        let phase = args::rec_f64(&out, "phase").unwrap();
        assert!(phase > 0.0);
    }

    #[test]
    fn wave20_bin_to_freq_linear_nyquist_half() {
        let mut m = BTreeMap::new();
        m.insert("bin".into(), Value::U64(512));
        m.insert("sample_rate".into(), Value::U64(44_100));
        m.insert("bin_count".into(), Value::U64(1024));
        let out = bin_to_freq_linear_host(&Value::Record(m), span()).unwrap();
        let hz = args::rec_f64(&out, "frequency_hz").unwrap();
        // bin * sr / (2 * bin_count) = 512 * 44100 / 2048 = 11025
        assert!((hz - 11_025.0).abs() < 1e-6);
    }

    #[test]
    fn wave20_bin_to_freq_log_octave() {
        let mut m = BTreeMap::new();
        m.insert("bin".into(), Value::U64(12));
        m.insert("f_min".into(), Value::F64(440.0));
        m.insert("bins_per_octave".into(), Value::U64(12));
        let out = bin_to_freq_log_host(&Value::Record(m), span()).unwrap();
        let hz = args::rec_f64(&out, "frequency_hz").unwrap();
        assert!((hz - 880.0).abs() < 1e-4);
    }
}
