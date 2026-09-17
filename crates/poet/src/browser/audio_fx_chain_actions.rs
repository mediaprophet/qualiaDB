//! Dual-path Tool Chest actions for remaining Host-bound `Audio.*` FX ids (wave 22).
//! No Host widen. Local sketches are CPU numeric summaries (n≤16 / 8-sample). Not GPU.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_N: u64 = 16;
const DEFAULT_SR: f64 = 44_100.0;
const DEFAULT_DELAY_SAMPLES: u64 = 4410;
const DEFAULT_TRANSPORT_ACTION: &str = "status";
const DEFAULT_ATTACK: f64 = 0.01;
const DEFAULT_INPUT: [f64; 8] = [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];

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

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

fn ctx(document: &Document) -> (Option<Element>, Vec<f64>) {
    (
        selected_container(document),
        parse_numbers(&selected_source(document).unwrap_or_default()),
    )
}
fn clamp_n(v: f64) -> u64 {
    v.round().clamp(1.0, 256.0) as u64
}
fn n_attr(el: Option<&Element>, nums: &[f64], idx: usize) -> u64 {
    numeric_attr(el, "data-n")
        .or_else(|| nums.get(idx).copied())
        .map(clamp_n)
        .unwrap_or(DEFAULT_N)
}

fn sample_rate(el: Option<&Element>) -> f64 {
    numeric_attr(el, "data-sample-rate")
        .filter(|v| *v > 0.0)
        .unwrap_or(DEFAULT_SR)
}

fn input_or_default(nums: &[f64]) -> Vec<f64> {
    if nums.is_empty() {
        DEFAULT_INPUT.to_vec()
    } else {
        nums.to_vec()
    }
}

fn peak_rms(xs: &[f64]) -> (f64, f64) {
    if xs.is_empty() {
        return (0.0, 0.0);
    }
    let (mut peak, mut ss) = (0.0_f64, 0.0_f64);
    for &x in xs {
        peak = peak.max(x.abs());
        ss += x * x;
    }
    (peak, (ss / xs.len() as f64).sqrt())
}

/// Sine sample i at phase 0. Index 0 (t=0) is 0.
fn local_sine_sample(i: usize, frequency: f64, sample_rate: f64, gain: f64) -> f64 {
    (i as f64 / sample_rate.max(1.0) * frequency * std::f64::consts::TAU).sin() * gain
}

fn local_sine_peak(n: usize, frequency: f64, sample_rate: f64, gain: f64) -> f64 {
    (0..n)
        .map(|i| local_sine_sample(i, frequency, sample_rate, gain).abs())
        .fold(0.0_f64, f64::max)
}

fn waveform_name(raw: Option<String>) -> &'static str {
    match raw.as_deref() {
        Some("square") => "square",
        Some("sawtooth") => "sawtooth",
        Some("triangle") => "triangle",
        _ => "sine",
    }
}

fn filter_name(raw: Option<String>) -> &'static str {
    match raw.as_deref() {
        Some("highpass" | "hp") => "highpass",
        Some("bandpass" | "bp") => "bandpass",
        Some("notch") => "notch",
        _ => "lowpass",
    }
}

fn transport_action(raw: Option<String>) -> &'static str {
    match raw.as_deref() {
        Some("play") => "play",
        Some("stop") => "stop",
        Some("pause") => "pause",
        Some("record") => "record",
        _ => DEFAULT_TRANSPORT_ACTION,
    }
}

fn stereo_split(nums: &[f64]) -> (Vec<f64>, Vec<f64>) {
    if nums.len() >= 2 {
        let mid = nums.len() / 2;
        (nums[..mid].to_vec(), nums[mid..].to_vec())
    } else {
        (DEFAULT_INPUT[..4].to_vec(), DEFAULT_INPUT[4..].to_vec())
    }
}

/// `Audio.oscillator` — `{ waveform, frequency, sample_rate?, n?, gain? }`.
pub(super) fn run_fx_oscillator(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let waveform = waveform_name(string_attr(c.as_ref(), "data-waveform"));
    let frequency = numeric_attr(c.as_ref(), "data-frequency")
        .or_else(|| nums.first().copied())
        .unwrap_or(440.0);
    let sr = sample_rate(c.as_ref());
    let n = n_attr(c.as_ref(), &nums, 1);
    let gain = numeric_attr(c.as_ref(), "data-gain")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(1.0);
    let peak = local_sine_peak(n as usize, frequency, sr, gain);
    invoke_dual(
        document,
        label,
        "Audio.oscillator",
        format!("osc sketch {waveform} f={frequency:.2} n={n} peak={peak:.4} (CPU sine; Host on Live)"),
        json!({ "waveform": waveform, "frequency": frequency, "sample_rate": sr, "n": n, "gain": gain }),
    );
}

/// `Audio.envelope` — ADSR.
pub(super) fn run_fx_envelope(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let attack = numeric_attr(c.as_ref(), "data-attack").unwrap_or(DEFAULT_ATTACK);
    let decay = numeric_attr(c.as_ref(), "data-decay").unwrap_or(0.1);
    let sustain = numeric_attr(c.as_ref(), "data-sustain").unwrap_or(0.7);
    let release = numeric_attr(c.as_ref(), "data-release").unwrap_or(0.2);
    let sr = sample_rate(c.as_ref());
    let n = n_attr(c.as_ref(), &nums, 0);
    let note_off = numeric_attr(c.as_ref(), "data-note-off-samples")
        .map(clamp_n)
        .unwrap_or(n / 2);
    invoke_dual(
        document,
        label,
        "Audio.envelope",
        format!("ADSR sketch n={n} attack={attack:.4} decay={decay:.4} sustain={sustain:.4} (Host on Live)"),
        json!({
            "attack": attack, "decay": decay, "sustain": sustain, "release": release,
            "sample_rate": sr, "n": n, "note_off_samples": note_off,
        }),
    );
}

/// `Audio.filter` — `{ input, filter_type, cutoff, q?, sample_rate? }`.
pub(super) fn run_fx_filter(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let filter_type = filter_name(string_attr(c.as_ref(), "data-filter-type"));
    let cutoff = numeric_attr(c.as_ref(), "data-cutoff").unwrap_or(1000.0);
    let q = numeric_attr(c.as_ref(), "data-q").unwrap_or(0.707);
    let sr = sample_rate(c.as_ref());
    let dc = input.iter().sum::<f64>() / input.len().max(1) as f64;
    invoke_dual(
        document,
        label,
        "Audio.filter",
        format!("filter sketch {filter_type} cutoff={cutoff:.1} q={q:.3} dc_mean={dc:.4} n={}", input.len()),
        json!({ "input": input, "filter_type": filter_type, "cutoff": cutoff, "q": q, "sample_rate": sr }),
    );
}

/// `Audio.lfo` — `{ waveform?, frequency, sample_rate?, n?, depth? }`.
pub(super) fn run_fx_lfo(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let waveform = waveform_name(string_attr(c.as_ref(), "data-waveform"));
    let frequency = numeric_attr(c.as_ref(), "data-frequency")
        .or_else(|| nums.first().copied())
        .unwrap_or(5.0);
    let sr = sample_rate(c.as_ref());
    let n = n_attr(c.as_ref(), &nums, 1);
    let depth = numeric_attr(c.as_ref(), "data-depth")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(1.0);
    let peak = local_sine_peak(n as usize, frequency, sr, depth);
    invoke_dual(
        document,
        label,
        "Audio.lfo",
        format!("lfo sketch {waveform} f={frequency:.2} n={n} depth={depth:.4} peak={peak:.4}"),
        json!({ "waveform": waveform, "frequency": frequency, "sample_rate": sr, "n": n, "depth": depth }),
    );
}

/// `Audio.delay` — `{ input, delay_samples?, feedback?, mix? }`.
pub(super) fn run_fx_delay(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let delay_samples = numeric_attr(c.as_ref(), "data-delay-samples")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(DEFAULT_DELAY_SAMPLES);
    let feedback = numeric_attr(c.as_ref(), "data-feedback").unwrap_or(0.3);
    let mix = numeric_attr(c.as_ref(), "data-mix").unwrap_or(0.5);
    let (peak, rms) = peak_rms(&input);
    invoke_dual(
        document,
        label,
        "Audio.delay",
        format!("delay sketch delay_samples={delay_samples} fb={feedback:.3} mix={mix:.3} peak={peak:.4} rms={rms:.4}"),
        json!({ "input": input, "delay_samples": delay_samples, "feedback": feedback, "mix": mix }),
    );
}

/// `Audio.reverb` — `{ input, room_size?, damping?, mix?, sample_rate? }`.
pub(super) fn run_fx_reverb(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let room_size = numeric_attr(c.as_ref(), "data-room-size").unwrap_or(0.5);
    let damping = numeric_attr(c.as_ref(), "data-damping").unwrap_or(0.3);
    let mix = numeric_attr(c.as_ref(), "data-mix").unwrap_or(0.3);
    let (peak, rms) = peak_rms(&input);
    invoke_dual(
        document,
        label,
        "Audio.reverb",
        format!("reverb sketch room={room_size:.3} damp={damping:.3} mix={mix:.3} peak={peak:.4} rms={rms:.4}"),
        json!({
            "input": input, "room_size": room_size, "damping": damping, "mix": mix,
            "sample_rate": sample_rate(c.as_ref()),
        }),
    );
}

/// `Audio.compressor` — `{ input, threshold?, ratio?, attack?, release?, sample_rate? }`.
pub(super) fn run_fx_compressor(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let threshold = numeric_attr(c.as_ref(), "data-threshold").unwrap_or(-20.0);
    let ratio = numeric_attr(c.as_ref(), "data-ratio").unwrap_or(4.0);
    let attack = numeric_attr(c.as_ref(), "data-attack").unwrap_or(0.003);
    let release = numeric_attr(c.as_ref(), "data-release").unwrap_or(0.1);
    let (peak, rms) = peak_rms(&input);
    invoke_dual(
        document,
        label,
        "Audio.compressor",
        format!("comp sketch thr={threshold:.1}dB ratio={ratio:.2} peak={peak:.4} rms={rms:.4}"),
        json!({
            "input": input, "threshold": threshold, "ratio": ratio, "attack": attack,
            "release": release, "sample_rate": sample_rate(c.as_ref()),
        }),
    );
}

/// `Audio.eq` — `{ input, low_gain?, mid_gain?, high_gain?, sample_rate? }`.
pub(super) fn run_fx_eq(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let low_gain = numeric_attr(c.as_ref(), "data-low-gain").unwrap_or(0.0);
    let mid_gain = numeric_attr(c.as_ref(), "data-mid-gain").unwrap_or(0.0);
    let high_gain = numeric_attr(c.as_ref(), "data-high-gain").unwrap_or(0.0);
    let (peak, rms) = peak_rms(&input);
    invoke_dual(
        document,
        label,
        "Audio.eq",
        format!("eq sketch low={low_gain:.1} mid={mid_gain:.1} high={high_gain:.1} peak={peak:.4} rms={rms:.4}"),
        json!({
            "input": input, "low_gain": low_gain, "mid_gain": mid_gain, "high_gain": high_gain,
            "sample_rate": sample_rate(c.as_ref()),
        }),
    );
}

/// `Audio.transport` — `{ action, tempo?, sample_rate? }`.
pub(super) fn run_fx_transport(document: &Document, label: &str) {
    let (c, _nums) = ctx(document);
    let action = transport_action(string_attr(c.as_ref(), "data-action"));
    let tempo = numeric_attr(c.as_ref(), "data-tempo").unwrap_or(120.0);
    let sr = sample_rate(c.as_ref());
    let mut args = json!({ "action": action, "tempo": tempo, "sample_rate": sr });
    if let (Some(start), Some(end)) = (
        numeric_attr(c.as_ref(), "data-loop-start").map(|v| v.max(0.0) as u64),
        numeric_attr(c.as_ref(), "data-loop-end").map(|v| v.max(0.0) as u64),
    ) {
        args["loop_start"] = json!(start);
        args["loop_end"] = json!(end);
    }
    invoke_dual(
        document,
        label,
        "Audio.transport",
        format!("transport sketch action={action} tempo={tempo:.1} sr={sr:.0}"),
        args,
    );
}

/// `Audio.waveform_meter` — `{ input, buckets? }`.
pub(super) fn run_fx_waveform_meter(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let buckets = numeric_attr(c.as_ref(), "data-buckets").map(clamp_n).unwrap_or(8);
    let (peak, rms) = peak_rms(&input);
    invoke_dual(
        document,
        label,
        "Audio.waveform_meter",
        format!("waveform meter sketch n={} buckets={buckets} peak={peak:.4} rms={rms:.4}", input.len()),
        json!({ "input": input, "buckets": buckets }),
    );
}

/// `Audio.phase_meter` — `{ left, right }`. One list splits at half.
pub(super) fn run_fx_phase_meter(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let left_a = string_attr(c.as_ref(), "data-left").map(|s| parse_numbers(&s));
    let right_a = string_attr(c.as_ref(), "data-right").map(|s| parse_numbers(&s));
    let (left, right) = match (left_a, right_a) {
        (Some(l), Some(r)) if !l.is_empty() && !r.is_empty() => (l, r),
        _ => stereo_split(&nums),
    };
    let n = left.len().min(right.len());
    let (mut num, mut dl, mut dr) = (0.0, 0.0, 0.0);
    for i in 0..n {
        num += left[i] * right[i];
        dl += left[i] * left[i];
        dr += right[i] * right[i];
    }
    let den = (dl * dr).sqrt();
    let corr = if den < 1e-18 { 0.0 } else { (num / den).clamp(-1.0, 1.0) };
    invoke_dual(
        document,
        label,
        "Audio.phase_meter",
        format!("phase meter sketch n={n} corr={corr:.4} (dot-product sketch; Host on Live)"),
        json!({ "left": left, "right": right }),
    );
}

/// `Audio.loudness_meter` — `{ input, sample_rate? }`.
pub(super) fn run_fx_loudness_meter(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let input = input_or_default(&nums);
    let (peak, rms) = peak_rms(&input);
    invoke_dual(
        document,
        label,
        "Audio.loudness_meter",
        format!("loudness sketch n={} peak={peak:.4} rms={rms:.4} (LUFS is Host; not GPU)", input.len()),
        json!({ "input": input, "sample_rate": sample_rate(c.as_ref()) }),
    );
}

/// `Audio.spectrum` — `{ raster, frame_count, bin_count, sample_rate, hop_size }`.
pub(super) fn run_fx_spectrum(document: &Document, label: &str) {
    let (c, nums) = ctx(document);
    let frame_count = numeric_attr(c.as_ref(), "data-frame-count").map(clamp_n).unwrap_or(1);
    let bin_count = numeric_attr(c.as_ref(), "data-bin-count").map(clamp_n).unwrap_or(8);
    let hop_size = numeric_attr(c.as_ref(), "data-hop-size")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(8);
    let sr = sample_rate(c.as_ref()).round().max(1.0) as u64;
    let need = (frame_count as usize).saturating_mul(bin_count as usize).max(1);
    let mut raster = nums;
    raster.truncate(need);
    while raster.len() < need {
        raster.push(0.0);
    }
    let energy: f64 = raster.iter().map(|x| x * x).sum();
    invoke_dual(
        document,
        label,
        "Audio.spectrum",
        format!("spectrum sketch frames={frame_count} bins={bin_count} hop={hop_size} energy={energy:.4}"),
        json!({
            "raster": raster, "frame_count": frame_count, "bin_count": bin_count,
            "sample_rate": sr, "hop_size": hop_size,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_audio_wave22_fx_sketches() {
        assert_eq!(parse_numbers("1, 2; 3 | 4"), vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!((clamp_n(0.0), clamp_n(16.0), clamp_n(999.0)), (1, 16, 256));
        assert_eq!(DEFAULT_N, 16);
        assert!((local_sine_sample(0, 440.0, DEFAULT_SR, 1.0)).abs() < 1e-12);
        assert!(DEFAULT_DELAY_SAMPLES > 0 && DEFAULT_DELAY_SAMPLES == 4410);
        assert_eq!(DEFAULT_TRANSPORT_ACTION, "status");
        assert!(DEFAULT_ATTACK > 0.0);
        let dc = DEFAULT_INPUT.iter().sum::<f64>() / DEFAULT_INPUT.len() as f64;
        assert!((dc - 0.5).abs() < 1e-12);
        assert_eq!(stereo_split(&[1.0, 2.0, 3.0, 4.0]), (vec![1.0, 2.0], vec![3.0, 4.0]));
    }
}
