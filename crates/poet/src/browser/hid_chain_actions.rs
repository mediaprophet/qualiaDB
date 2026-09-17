//! Dual-path Tool Chest actions for curated Host-bound `HID.*` ids (wave 23).
//!
//! Pointer, gamepad, MIDI, haptics, spatial XR, and biosignal poll. Hardware
//! is not assumed in the local sketch: sketches report Host unwrap_or defaults
//! (empty poll, identity pose, dispatched rumble) without inventing live devices.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_CURSOR: &str = "default";
const DEFAULT_MIDI_PORT: &str = "default";
const DEFAULT_HAPTIC_PATTERN: &str = "click";
const DEFAULT_BIOSIGNAL: &str = "eeg";
const DEFAULT_HAND: &str = "right";

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn u64_attr(el: Option<&Element>, name: &str) -> Option<u64> {
    numeric_attr(el, name).map(|v| v.max(0.0) as u64)
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

fn local_str(from_attr: Option<&str>, default: &str) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn local_cursor(style: Option<&str>) -> String {
    local_str(style, DEFAULT_CURSOR)
}

fn local_gamepad_index(index: Option<u64>) -> u64 {
    index.unwrap_or(0)
}

fn local_rumble(
    weak: Option<f64>,
    strong: Option<f64>,
    duration_ms: Option<u64>,
) -> (f64, f64, u64) {
    (
        weak.unwrap_or(0.5).clamp(0.0, 1.0),
        strong.unwrap_or(0.5).clamp(0.0, 1.0),
        duration_ms.unwrap_or(100),
    )
}

fn local_midi(status: Option<u64>, data1: Option<u64>, data2: Option<u64>) -> (u64, u64, u64) {
    (
        status.unwrap_or(0x90),
        data1.unwrap_or(60).min(127),
        data2.unwrap_or(127).min(127),
    )
}

fn local_haptic_pulse(
    duration_ms: Option<f64>,
    intensity: Option<f64>,
    freq_hz: Option<f64>,
) -> (f64, f64, f64) {
    (
        duration_ms.unwrap_or(50.0),
        intensity.unwrap_or(1.0).clamp(0.0, 1.0),
        freq_hz.unwrap_or(160.0),
    )
}

fn local_haptic_pattern(pattern: Option<&str>, intensity: Option<f64>) -> (String, f64) {
    (
        local_str(pattern, DEFAULT_HAPTIC_PATTERN),
        intensity.unwrap_or(1.0).clamp(0.0, 1.0),
    )
}

fn local_hand_is_left(hand: Option<&str>) -> bool {
    hand.map(|h| h.eq_ignore_ascii_case("left")).unwrap_or(false)
}

fn local_biosignal(modality: Option<&str>, epsilon: Option<f64>) -> (String, f64) {
    (
        local_str(modality, DEFAULT_BIOSIGNAL),
        epsilon.filter(|v| *v > 0.0).unwrap_or(1.0),
    )
}

/// `HID.poll` — no required args; empty ring → Null.
pub(super) fn run_poll(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "HID.poll",
        "hid poll sketch empty=true".into(),
        json!({}),
    );
}

/// `HID.wait` — `{ timeout_ns? }`.
pub(super) fn run_wait(document: &Document, label: &str) {
    let timeout_ns = u64_attr(selected_container(document).as_ref(), "data-timeout-ns").unwrap_or(0);
    invoke_dual(
        document,
        label,
        "HID.wait",
        format!("hid wait sketch timeout_ns={timeout_ns} empty=true"),
        json!({ "timeout_ns": timeout_ns }),
    );
}

/// `HID.clear` — drain host ring; sketch remaining=0.
pub(super) fn run_clear(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "HID.clear",
        "hid clear sketch cleared_count=0 remaining=0".into(),
        json!({}),
    );
}

/// `HID.pointer_capture` — `{ target_id? }`.
pub(super) fn run_pointer_capture(document: &Document, label: &str) {
    let target_id = u64_attr(selected_container(document).as_ref(), "data-target-id").unwrap_or(0);
    invoke_dual(
        document,
        label,
        "HID.pointer_capture",
        format!("hid pointer_capture sketch captured=true target_id={target_id}"),
        json!({ "target_id": target_id }),
    );
}

/// `HID.pointer_release`.
pub(super) fn run_pointer_release(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "HID.pointer_release",
        "hid pointer_release sketch released=true".into(),
        json!({}),
    );
}

/// `HID.set_cursor` — `{ style? | cursor? }`.
pub(super) fn run_set_cursor(document: &Document, label: &str) {
    let container = selected_container(document);
    let style = local_cursor(
        string_attr(container.as_ref(), "data-cursor")
            .or_else(|| string_attr(container.as_ref(), "data-style"))
            .as_deref(),
    );
    invoke_dual(
        document,
        label,
        "HID.set_cursor",
        format!("hid set_cursor sketch cursor={style} applied=true"),
        json!({ "style": style }),
    );
}

/// `HID.gamepad_poll` — `{ index? }`.
pub(super) fn run_gamepad_poll(document: &Document, label: &str) {
    let index = local_gamepad_index(u64_attr(selected_container(document).as_ref(), "data-index"));
    invoke_dual(
        document,
        label,
        "HID.gamepad_poll",
        format!("hid gamepad_poll sketch index={index} connected=true buttons_mask=0"),
        json!({ "index": index }),
    );
}

/// `HID.gamepad_vibrate` — `{ index?, weak_magnitude?, strong_magnitude?, duration_ms? }`.
pub(super) fn run_gamepad_vibrate(document: &Document, label: &str) {
    let container = selected_container(document);
    let index = local_gamepad_index(u64_attr(container.as_ref(), "data-index"));
    let (weak, strong, duration_ms) = local_rumble(
        numeric_attr(container.as_ref(), "data-weak-magnitude"),
        numeric_attr(container.as_ref(), "data-strong-magnitude"),
        u64_attr(container.as_ref(), "data-duration-ms"),
    );
    invoke_dual(
        document,
        label,
        "HID.gamepad_vibrate",
        format!("hid gamepad_vibrate sketch index={index} weak={weak} strong={strong} duration_ms={duration_ms}"),
        json!({
            "index": index,
            "weak_magnitude": weak,
            "strong_magnitude": strong,
            "duration_ms": duration_ms
        }),
    );
}

/// `HID.midi_send` — `{ status?, data1?, data2?, port? }`.
pub(super) fn run_midi_send(document: &Document, label: &str) {
    let container = selected_container(document);
    let (status, data1, data2) = local_midi(
        u64_attr(container.as_ref(), "data-status"),
        u64_attr(container.as_ref(), "data-data1"),
        u64_attr(container.as_ref(), "data-data2"),
    );
    let port = local_str(
        string_attr(container.as_ref(), "data-port").as_deref(),
        DEFAULT_MIDI_PORT,
    );
    invoke_dual(
        document,
        label,
        "HID.midi_send",
        format!("hid midi_send sketch port={port} status={status} data1={data1} data2={data2}"),
        json!({ "status": status, "data1": data1, "data2": data2, "port": port }),
    );
}

/// `HID.midi_poll` — `{ port? }`.
pub(super) fn run_midi_poll(document: &Document, label: &str) {
    let port = local_str(
        string_attr(selected_container(document).as_ref(), "data-port").as_deref(),
        DEFAULT_MIDI_PORT,
    );
    invoke_dual(
        document,
        label,
        "HID.midi_poll",
        format!("hid midi_poll sketch port={port} has_event=false"),
        json!({ "port": port }),
    );
}

/// `HID.haptic_pulse` — `{ duration_ms?, intensity?, frequency_hz? }`.
pub(super) fn run_haptic_pulse(document: &Document, label: &str) {
    let container = selected_container(document);
    let (duration_ms, intensity, frequency_hz) = local_haptic_pulse(
        numeric_attr(container.as_ref(), "data-duration-ms"),
        numeric_attr(container.as_ref(), "data-intensity"),
        numeric_attr(container.as_ref(), "data-frequency-hz"),
    );
    invoke_dual(
        document,
        label,
        "HID.haptic_pulse",
        format!("hid haptic_pulse sketch duration_ms={duration_ms} intensity={intensity} frequency_hz={frequency_hz}"),
        json!({
            "duration_ms": duration_ms,
            "intensity": intensity,
            "frequency_hz": frequency_hz
        }),
    );
}

/// `HID.haptic_pattern` — `{ pattern?, intensity? }`.
pub(super) fn run_haptic_pattern(document: &Document, label: &str) {
    let container = selected_container(document);
    let (pattern, intensity) = local_haptic_pattern(
        string_attr(container.as_ref(), "data-pattern").as_deref(),
        numeric_attr(container.as_ref(), "data-intensity"),
    );
    invoke_dual(
        document,
        label,
        "HID.haptic_pattern",
        format!("hid haptic_pattern sketch pattern={pattern} intensity={intensity}"),
        json!({ "pattern": pattern, "intensity": intensity }),
    );
}

/// `HID.spatial_head_pose`.
pub(super) fn run_spatial_head_pose(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "HID.spatial_head_pose",
        "hid spatial_head_pose sketch position=[0,1.7,0] orientation=[0,0,0,1]".into(),
        json!({}),
    );
}

/// `HID.spatial_hand_skeleton` — `{ hand? }`.
pub(super) fn run_spatial_hand_skeleton(document: &Document, label: &str) {
    let hand = local_str(
        string_attr(selected_container(document).as_ref(), "data-hand").as_deref(),
        DEFAULT_HAND,
    );
    let is_left = local_hand_is_left(Some(&hand));
    invoke_dual(
        document,
        label,
        "HID.spatial_hand_skeleton",
        format!("hid spatial_hand_skeleton sketch is_left={is_left} joint_count=26"),
        json!({ "hand": hand }),
    );
}

/// `HID.spatial_gaze_ray`.
pub(super) fn run_spatial_gaze_ray(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "HID.spatial_gaze_ray",
        "hid spatial_gaze_ray sketch origin=[0,1.65,0] direction=[0,0,-1]".into(),
        json!({}),
    );
}

/// `HID.biosignal_poll` — `{ modality?, epsilon? }`.
pub(super) fn run_biosignal_poll(document: &Document, label: &str) {
    let container = selected_container(document);
    let (modality, epsilon) = local_biosignal(
        string_attr(container.as_ref(), "data-modality").as_deref(),
        numeric_attr(container.as_ref(), "data-epsilon"),
    );
    invoke_dual(
        document,
        label,
        "HID.biosignal_poll",
        format!("hid biosignal_poll sketch modality={modality} epsilon={epsilon} calibrated=true"),
        json!({ "modality": modality, "epsilon": epsilon }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_hid_wave23_defaults() {
        assert_eq!(local_cursor(None), "default");
        assert_eq!(local_gamepad_index(None), 0);
        let (w, s, d) = local_rumble(None, None, None);
        assert!((w - 0.5).abs() < 1e-12);
        assert!((s - 0.5).abs() < 1e-12);
        assert_eq!(d, 100);
        assert_eq!(local_midi(None, None, None), (0x90, 60, 127));
        let (dur, intensity, freq) = local_haptic_pulse(None, None, None);
        assert!((dur - 50.0).abs() < 1e-12);
        assert!((intensity - 1.0).abs() < 1e-12);
        assert!((freq - 160.0).abs() < 1e-12);
        let (pattern, p_int) = local_haptic_pattern(None, None);
        assert_eq!(pattern, "click");
        assert!((p_int - 1.0).abs() < 1e-12);
        assert!(!local_hand_is_left(None));
        assert!(local_hand_is_left(Some("left")));
        let (modl, eps) = local_biosignal(None, None);
        assert_eq!(modl, "eeg");
        assert!((eps - 1.0).abs() < 1e-12);
    }
}
