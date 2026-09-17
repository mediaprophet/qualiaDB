//! Dual-path Tool Chest actions for curated Host-bound `Audio.*` ids (wave 21).
//!
//! Includes wave-19/20 DSP Host binds plus MIDI/grid scalars. No Host widen —
//! scopes must already exist in `poet_host/invoke/ids.rs`. Local sketches mirror
//! Host CPU algebra from `audio::dsp_kernel` / `tf_surface` / MIDI helpers.

use serde_json::json;
use web_sys::{Document, Element};

const SPECTRAL_PREVIEW_BINS: usize = 64;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(256)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn u64_attr(el: Option<&Element>, name: &str) -> Option<u64> {
    numeric_attr(el, name).map(|v| v.max(0.0) as u64)
}

fn i64_attr(el: Option<&Element>, name: &str) -> Option<i64> {
    numeric_attr(el, name).map(|v| v as i64)
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn local_epistemic_temperature_from_q(q: f64) -> f64 {
    (q * q).clamp(0.0, 4.0)
}

fn local_epistemic_fm_index(q: f64, mu: f64) -> f64 {
    let tau = local_epistemic_temperature_from_q(q);
    (tau * 0.25 + mu.abs() * 0.5).clamp(0.0, 8.0)
}

fn local_sigma_dominant_frequency(bins: &[f64], base_hz: f64) -> f64 {
    let n = bins.len().min(SPECTRAL_PREVIEW_BINS);
    let mut peak = 0usize;
    let mut max_e = 0.0_f64;
    for (i, &e) in bins.iter().take(n).enumerate() {
        let a = e.abs();
        if a > max_e {
            max_e = a;
            peak = i;
        }
    }
    let ratio = (peak as f64 + 1.0) / SPECTRAL_PREVIEW_BINS as f64;
    (base_hz * (0.5 + ratio * 3.0)).clamp(55.0, 8_000.0)
}

fn local_parametric_sample(
    phase: f64,
    frequency_hz: f64,
    gain: f64,
    fm_index: f64,
    sample_rate: f64,
) -> (f64, f64) {
    let dt = 1.0 / sample_rate.max(1.0);
    let mod_phase = phase * (1.0 + fm_index * 0.01);
    let sample = (mod_phase * std::f64::consts::TAU).sin() * gain;
    let mut next_phase = phase + frequency_hz * dt;
    if next_phase > 1.0 {
        next_phase -= next_phase.floor();
    }
    (sample, next_phase)
}

fn local_bin_to_freq_linear(bin: u64, sample_rate: u64, bin_count: u64) -> f64 {
    if sample_rate == 0 || bin_count == 0 {
        return 0.0;
    }
    bin as f64 * (sample_rate as f64 / (2.0 * bin_count as f64))
}

fn local_bin_to_freq_log(bin: u64, f_min: f64, bins_per_octave: u64) -> f64 {
    if bins_per_octave == 0 {
        return 0.0;
    }
    f_min * 2.0_f64.powf(bin as f64 / bins_per_octave as f64)
}

fn local_midi_note_to_freq(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0)
}

fn local_quantize(position: f64, grid: f64) -> f64 {
    if grid <= 0.0 {
        return position;
    }
    (position / grid).round() * grid
}

fn local_transpose(note: u8, semitones: i32) -> u8 {
    (note as i32 + semitones).clamp(0, 127) as u8
}

/// `Audio.epistemic_temperature_from_q` — `{ q }` → `{ temperature }`.
pub(super) fn run_epistemic_temperature_from_q(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let q = numeric_attr(container.as_ref(), "data-q")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.5);
    let t = local_epistemic_temperature_from_q(q);
    invoke_dual(
        document,
        label,
        "Audio.epistemic_temperature_from_q",
        format!("tau sketch q={q:.4} -> temperature={t:.6}"),
        json!({ "q": q }),
    );
}

/// `Audio.epistemic_fm_index` — `{ q, mu }` → `{ fm_index }`.
pub(super) fn run_epistemic_fm_index(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let q = numeric_attr(container.as_ref(), "data-q")
        .or_else(|| nums.first().copied())
        .unwrap_or(2.0);
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(20.0);
    let idx = local_epistemic_fm_index(q, mu);
    invoke_dual(
        document,
        label,
        "Audio.epistemic_fm_index",
        format!("fm_index sketch q={q:.4} mu={mu:.4} -> {idx:.6}"),
        json!({ "q": q, "mu": mu }),
    );
}

/// `Audio.sigma_dominant_frequency` — `{ bins[64], base_hz? }` → `{ frequency_hz }`.
pub(super) fn run_sigma_dominant_frequency(document: &Document, label: &str) {
    let container = selected_container(document);
    let mut bins = parse_numbers(&selected_source(document).unwrap_or_default());
    if bins.len() != SPECTRAL_PREVIEW_BINS {
        bins = vec![0.0; SPECTRAL_PREVIEW_BINS];
        bins[32] = 1.0;
    }
    let base_hz = numeric_attr(container.as_ref(), "data-base-hz").unwrap_or(220.0);
    let hz = local_sigma_dominant_frequency(&bins, base_hz);
    invoke_dual(
        document,
        label,
        "Audio.sigma_dominant_frequency",
        format!("sigma-dominant sketch -> frequency_hz={hz:.4} (base={base_hz})"),
        json!({ "bins": bins, "base_hz": base_hz }),
    );
}

/// `Audio.parametric_sample` — voice state → one sample + advanced phase.
pub(super) fn run_parametric_sample(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let phase = numeric_attr(container.as_ref(), "data-phase")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let frequency_hz = numeric_attr(container.as_ref(), "data-frequency-hz")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(440.0);
    let gain = numeric_attr(container.as_ref(), "data-gain")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(1.0);
    let fm_index = numeric_attr(container.as_ref(), "data-fm-index")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.0);
    let sample_rate = numeric_attr(container.as_ref(), "data-sample-rate").unwrap_or(48_000.0);
    let (sample, next_phase) =
        local_parametric_sample(phase, frequency_hz, gain, fm_index, sample_rate);
    invoke_dual(
        document,
        label,
        "Audio.parametric_sample",
        format!("parametric sketch sample={sample:.6} phase->{next_phase:.6}"),
        json!({
            "phase": phase,
            "frequency_hz": frequency_hz,
            "gain": gain,
            "fm_index": fm_index,
            "sample_rate": sample_rate,
        }),
    );
}

/// `Audio.bin_to_freq_linear` — STFT bin → Hz.
pub(super) fn run_bin_to_freq_linear(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let bin = u64_attr(container.as_ref(), "data-bin")
        .or_else(|| nums.first().map(|v| v.max(0.0) as u64))
        .unwrap_or(512);
    let sample_rate = u64_attr(container.as_ref(), "data-sample-rate")
        .or_else(|| nums.get(1).map(|v| v.max(0.0) as u64))
        .unwrap_or(44_100);
    let bin_count = u64_attr(container.as_ref(), "data-bin-count")
        .or_else(|| nums.get(2).map(|v| v.max(0.0) as u64))
        .unwrap_or(1024);
    let hz = local_bin_to_freq_linear(bin, sample_rate, bin_count);
    invoke_dual(
        document,
        label,
        "Audio.bin_to_freq_linear",
        format!("bin->Hz linear sketch bin={bin} -> {hz:.4}"),
        json!({ "bin": bin, "sample_rate": sample_rate, "bin_count": bin_count }),
    );
}

/// `Audio.bin_to_freq_log` — CQT-style log bin → Hz.
pub(super) fn run_bin_to_freq_log(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let bin = u64_attr(container.as_ref(), "data-bin")
        .or_else(|| nums.first().map(|v| v.max(0.0) as u64))
        .unwrap_or(12);
    let f_min = numeric_attr(container.as_ref(), "data-f-min")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(440.0);
    let bins_per_octave = u64_attr(container.as_ref(), "data-bins-per-octave")
        .or_else(|| nums.get(2).map(|v| v.max(0.0) as u64))
        .unwrap_or(12);
    let hz = local_bin_to_freq_log(bin, f_min, bins_per_octave);
    invoke_dual(
        document,
        label,
        "Audio.bin_to_freq_log",
        format!("bin->Hz log sketch bin={bin} -> {hz:.4}"),
        json!({ "bin": bin, "f_min": f_min, "bins_per_octave": bins_per_octave }),
    );
}

/// `Audio.midi_note` — `to_freq` action (A4=440).
pub(super) fn run_midi_note(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let note = u64_attr(container.as_ref(), "data-note")
        .or_else(|| nums.first().map(|v| v.max(0.0) as u64))
        .unwrap_or(69)
        .min(127) as u8;
    let freq = local_midi_note_to_freq(note);
    invoke_dual(
        document,
        label,
        "Audio.midi_note",
        format!("midi_note to_freq sketch note={note} -> {freq:.4} Hz"),
        json!({ "action": "to_freq", "note": note as u64 }),
    );
}

/// `Audio.quantize` — beat position → grid.
pub(super) fn run_quantize(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let position = numeric_attr(container.as_ref(), "data-position")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.37);
    let grid = numeric_attr(container.as_ref(), "data-grid")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.25);
    let result = local_quantize(position, grid);
    invoke_dual(
        document,
        label,
        "Audio.quantize",
        format!("quantize sketch {position:.4} @ grid={grid:.4} -> {result:.4}"),
        json!({ "position": position, "grid": grid }),
    );
}

/// `Audio.transpose` — MIDI note ± semitones (clamped 0–127).
pub(super) fn run_transpose(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let note = u64_attr(container.as_ref(), "data-note")
        .or_else(|| nums.first().map(|v| v.max(0.0) as u64))
        .unwrap_or(60)
        .min(127) as u8;
    let semitones = i64_attr(container.as_ref(), "data-semitones")
        .or_else(|| nums.get(1).map(|v| *v as i64))
        .unwrap_or(12) as i32;
    let result = local_transpose(note, semitones);
    invoke_dual(
        document,
        label,
        "Audio.transpose",
        format!("transpose sketch note={note} +{semitones} -> {result}"),
        json!({ "note": note as u64, "semitones": semitones as i64 }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_audio_wave21_dsp_match_known() {
        assert!((local_epistemic_temperature_from_q(0.0) - 0.0).abs() < 1e-12);
        assert!((local_epistemic_temperature_from_q(1.5) - 2.25).abs() < 1e-12);
        assert!((local_epistemic_fm_index(2.0, 20.0) - 8.0).abs() < 1e-12);

        let mut bins = vec![0.0; SPECTRAL_PREVIEW_BINS];
        bins[32] = 1.0;
        let hz = local_sigma_dominant_frequency(&bins, 220.0);
        assert!(hz >= 55.0 && hz <= 8_000.0);

        let (s, phase) = local_parametric_sample(0.0, 440.0, 1.0, 0.0, 48_000.0);
        assert!((-1.0..=1.0).contains(&s));
        assert!(phase > 0.0);

        assert!((local_bin_to_freq_linear(512, 44_100, 1024) - 11_025.0).abs() < 1e-9);
        assert!((local_bin_to_freq_log(12, 440.0, 12) - 880.0).abs() < 1e-9);

        assert!((local_midi_note_to_freq(69) - 440.0).abs() < 1e-9);
        assert!((local_quantize(1.37, 0.25) - 1.25).abs() < 1e-12);
        assert_eq!(local_transpose(60, 12), 72);
        assert_eq!(local_transpose(120, 20), 127);
    }
}
