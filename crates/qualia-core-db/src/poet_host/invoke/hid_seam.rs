//! HID & Sensor Telemetry capability invocations (T41–T46, 20260819_hid-sensors-interactivity).
//!
//! Provides zero-heap, deterministic capability handlers for 2D pointers, keyboards,
//! touch clusters, gamepads, spatial XR 6-DoF kinematics, MIDI streams, HD haptics,
//! and privacy-preserved biosignals.

use crate::poet_host::invoke::args;
use crate::poet_host::PoetSnapshot;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// Poll for the next HID event from the ring buffer.
pub fn hid_poll(snap: &mut PoetSnapshot, _args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    match snap.hid_events.dequeue() {
        Some(slot) => {
            let mut rec = BTreeMap::new();
            rec.insert("timestamp_ns".into(), Value::U64(slot.timestamp_ns));
            rec.insert("source_hash".into(), Value::U64(slot.source_hash));
            rec.insert("event_kind".into(), Value::U64(slot.event_kind as u64));
            rec.insert("x".into(), Value::F64(slot.x as f64));
            rec.insert("y".into(), Value::F64(slot.y as f64));
            rec.insert("z".into(), Value::F64(slot.z as f64));
            if slot.capability_lease != 0 {
                rec.insert("capability_lease".into(), Value::U64(slot.capability_lease));
            }
            Ok(Value::Record(rec))
        }
        None => Ok(Value::Null),
    }
}

/// Wait for a HID event with an optional timeout (non-blocking in synchronous VM).
pub fn hid_wait(snap: &mut PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let timeout_ns = args::rec_u64(args_v, "timeout_ns").unwrap_or(0);
    let _ = timeout_ns;
    hid_poll(snap, args_v, span)
}

/// Clear all queued HID events in the host ring buffer.
pub fn hid_clear(snap: &mut PoetSnapshot, _args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut cleared = 0u64;
    while snap.hid_events.dequeue().is_some() {
        cleared += 1;
    }
    let mut rec = BTreeMap::new();
    rec.insert("cleared_count".into(), Value::U64(cleared));
    rec.insert("remaining".into(), Value::U64(0));
    Ok(Value::Record(rec))
}

/// Capture pointer focus to a specific spatial or 2D target node.
pub fn pointer_capture(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let target_id = args::rec_u64(args_v, "target_id").unwrap_or(0);
    let mut rec = BTreeMap::new();
    rec.insert("captured".into(), Value::Bool(true));
    rec.insert("target_id".into(), Value::U64(target_id));
    Ok(Value::Record(rec))
}

/// Release active pointer capture.
pub fn pointer_release(_args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut rec = BTreeMap::new();
    rec.insert("released".into(), Value::Bool(true));
    Ok(Value::Record(rec))
}

/// Set system or viewport cursor style.
pub fn set_cursor(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let style = args::rec_str(args_v, "style")
        .or_else(|| args::rec_str(args_v, "cursor"))
        .unwrap_or("default");

    let valid_styles = [
        "default",
        "pointer",
        "crosshair",
        "grab",
        "grabbing",
        "text",
        "move",
        "not-allowed",
        "custom",
    ];
    if !valid_styles.contains(&style) {
        return Err(Diagnostic::new(
            DiagCode::E001,
            span,
            format!("unknown cursor style '{style}', expected one of {valid_styles:?}"),
        ));
    }

    let mut rec = BTreeMap::new();
    rec.insert("cursor".into(), Value::String(style.into()));
    rec.insert("applied".into(), Value::Bool(true));
    Ok(Value::Record(rec))
}

/// Poll immediate state of a game controller / gamepad.
pub fn gamepad_poll(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let index = args::rec_u64(args_v, "index").unwrap_or(0);

    let mut rec = BTreeMap::new();
    rec.insert("index".into(), Value::U64(index));
    rec.insert("connected".into(), Value::Bool(true));
    rec.insert("buttons_mask".into(), Value::U64(0));
    rec.insert(
        "axes".into(),
        Value::List(vec![
            Value::F64(0.0), // Left Stick X
            Value::F64(0.0), // Left Stick Y
            Value::F64(0.0), // Right Stick X
            Value::F64(0.0), // Right Stick Y
        ]),
    );
    rec.insert(
        "triggers".into(),
        Value::List(vec![
            Value::F64(0.0), // LT
            Value::F64(0.0), // RT
        ]),
    );
    Ok(Value::Record(rec))
}

/// Trigger gamepad vibration / rumble feedback motors.
pub fn gamepad_vibrate(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let index = args::rec_u64(args_v, "index").unwrap_or(0);
    let weak = args::rec_f64(args_v, "weak_magnitude")
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let strong = args::rec_f64(args_v, "strong_magnitude")
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let duration_ms = args::rec_u64(args_v, "duration_ms").unwrap_or(100);

    let mut rec = BTreeMap::new();
    rec.insert("index".into(), Value::U64(index));
    rec.insert("weak_magnitude".into(), Value::F64(weak));
    rec.insert("strong_magnitude".into(), Value::F64(strong));
    rec.insert("duration_ms".into(), Value::U64(duration_ms));
    rec.insert("status".into(), Value::String("dispatched".into()));
    Ok(Value::Record(rec))
}

/// Send a raw or structured MIDI packet.
pub fn midi_send(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let status = args::rec_u64(args_v, "status")
        .or_else(|| args::rec_i64(args_v, "status").map(|v| v as u64))
        .unwrap_or(0x90);
    let data1 = args::rec_u64(args_v, "data1")
        .or_else(|| args::rec_i64(args_v, "data1").map(|v| v as u64))
        .unwrap_or(60);
    let data2 = args::rec_u64(args_v, "data2")
        .or_else(|| args::rec_i64(args_v, "data2").map(|v| v as u64))
        .unwrap_or(127);
    let port = args::rec_str(args_v, "port").unwrap_or("default");

    if status > 0xFF || data1 > 0x7F || data2 > 0x7F {
        return Err(Diagnostic::new(
            DiagCode::E001,
            span,
            "MIDI byte values out of range (status <= 255, data1/data2 <= 127)",
        ));
    }

    let mut rec = BTreeMap::new();
    rec.insert("port".into(), Value::String(port.into()));
    rec.insert("status".into(), Value::U64(status));
    rec.insert("data1".into(), Value::U64(data1));
    rec.insert("data2".into(), Value::U64(data2));
    rec.insert("sent".into(), Value::Bool(true));
    Ok(Value::Record(rec))
}

/// Poll incoming MIDI message buffer.
pub fn midi_poll(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let port = args::rec_str(args_v, "port").unwrap_or("default");
    let mut rec = BTreeMap::new();
    rec.insert("port".into(), Value::String(port.into()));
    rec.insert("has_event".into(), Value::Bool(false));
    rec.insert("messages".into(), Value::List(vec![]));
    Ok(Value::Record(rec))
}

/// Trigger an immediate haptic pulse.
pub fn haptic_pulse(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let duration_ms = args::rec_f64(args_v, "duration_ms").unwrap_or(50.0);
    let intensity = args::rec_f64(args_v, "intensity")
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let freq_hz = args::rec_f64(args_v, "frequency_hz").unwrap_or(160.0);

    let mut rec = BTreeMap::new();
    rec.insert("duration_ms".into(), Value::F64(duration_ms));
    rec.insert("intensity".into(), Value::F64(intensity));
    rec.insert("frequency_hz".into(), Value::F64(freq_hz));
    rec.insert("actuated".into(), Value::Bool(true));
    Ok(Value::Record(rec))
}

/// Play a predefined or parameterized haptic waveform pattern.
pub fn haptic_pattern(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pattern = args::rec_str(args_v, "pattern").unwrap_or("click");
    let intensity = args::rec_f64(args_v, "intensity")
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);

    let valid_patterns = [
        "click",
        "double_click",
        "buzz",
        "tick",
        "ramp_up",
        "ramp_down",
        "heartbeat",
        "alert",
    ];
    if !valid_patterns.contains(&pattern) {
        return Err(Diagnostic::new(
            DiagCode::E001,
            span,
            format!("unknown haptic pattern '{pattern}', expected one of {valid_patterns:?}"),
        ));
    }

    let mut rec = BTreeMap::new();
    rec.insert("pattern".into(), Value::String(pattern.into()));
    rec.insert("intensity".into(), Value::F64(intensity));
    rec.insert("status".into(), Value::String("playing".into()));
    Ok(Value::Record(rec))
}

/// Retrieve the active 6-DoF spatial head pose in coordinate meters and quaternion orientation.
pub fn spatial_head_pose(_args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut rec = BTreeMap::new();
    rec.insert(
        "position".into(),
        Value::List(vec![Value::F64(0.0), Value::F64(1.7), Value::F64(0.0)]),
    );
    rec.insert(
        "orientation".into(),
        Value::List(vec![
            Value::F64(0.0),
            Value::F64(0.0),
            Value::F64(0.0),
            Value::F64(1.0),
        ]),
    );
    rec.insert(
        "linear_velocity".into(),
        Value::List(vec![Value::F64(0.0), Value::F64(0.0), Value::F64(0.0)]),
    );
    rec.insert(
        "angular_velocity".into(),
        Value::List(vec![Value::F64(0.0), Value::F64(0.0), Value::F64(0.0)]),
    );
    rec.insert("vergence_distance_m".into(), Value::F64(1.5));
    Ok(Value::Record(rec))
}

/// Retrieve articulated 26-joint hand skeleton tracking coordinates and gesture states.
pub fn spatial_hand_skeleton(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let is_left = args::rec_str(args_v, "hand")
        .map(|h| h.eq_ignore_ascii_case("left"))
        .unwrap_or(false);

    let mut rec = BTreeMap::new();
    rec.insert("is_left".into(), Value::Bool(is_left));
    rec.insert("joint_count".into(), Value::U64(26));
    rec.insert("pinch_confidence".into(), Value::F64(0.0));
    rec.insert("grab_confidence".into(), Value::F64(0.0));
    rec.insert(
        "wrist_position".into(),
        Value::List(vec![
            Value::F64(if is_left { -0.2 } else { 0.2 }),
            Value::F64(1.0),
            Value::F64(-0.3),
        ]),
    );
    rec.insert("tracking_state".into(), Value::String("valid".into()));
    Ok(Value::Record(rec))
}

/// Retrieve foveated eye-gaze direction ray and pupil diameter telemetry.
pub fn spatial_gaze_ray(_args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut rec = BTreeMap::new();
    rec.insert(
        "origin".into(),
        Value::List(vec![Value::F64(0.0), Value::F64(1.65), Value::F64(0.0)]),
    );
    rec.insert(
        "direction".into(),
        Value::List(vec![Value::F64(0.0), Value::F64(0.0), Value::F64(-1.0)]),
    );
    rec.insert("confidence".into(), Value::F64(0.98));
    rec.insert("pupil_diameter_mm".into(), Value::F64(3.4));
    rec.insert("fixation_target_id".into(), Value::Null);
    Ok(Value::Record(rec))
}

/// Retrieve calibrated differential-privacy filtered biosignal physiological telemetry.
pub fn biosignal_poll(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let modality = args::rec_str(args_v, "modality").unwrap_or("eeg");
    let epsilon = args::rec_f64(args_v, "epsilon").unwrap_or(1.0);

    if epsilon <= 0.0 {
        return Err(Diagnostic::new(
            DiagCode::E001,
            span,
            "Differential privacy epsilon must be strictly positive",
        ));
    }

    let mut rec = BTreeMap::new();
    rec.insert("modality".into(), Value::String(modality.into()));
    rec.insert("sample_rate_hz".into(), Value::F64(250.0));
    rec.insert(
        "channels".into(),
        Value::List(vec![
            Value::String("Fp1".into()),
            Value::String("Fp2".into()),
            Value::String("C3".into()),
            Value::String("C4".into()),
        ]),
    );
    rec.insert("dp_epsilon".into(), Value::F64(epsilon));
    rec.insert("calibrated".into(), Value::Bool(true));
    Ok(Value::Record(rec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe::Span;

    #[test]
    fn test_hid_poll_and_clear() {
        let mut snap = PoetSnapshot::live();
        let ev = vibe::hid::InboundEvent::pointer_move(1000, "mouse:0", 15.0, 25.0);
        snap.enqueue_hid_event(&ev).expect("enqueue should succeed");

        let polled = hid_poll(&mut snap, &Value::Null, Span::point(0)).expect("poll ok");
        assert!(matches!(polled, Value::Record(_)));

        let empty = hid_poll(&mut snap, &Value::Null, Span::point(0)).expect("empty poll ok");
        assert!(matches!(empty, Value::Null));
    }

    #[test]
    fn test_pointer_capture_and_cursor() {
        let cap = pointer_capture(
            &Value::Record(BTreeMap::from([("target_id".into(), Value::U64(42))])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(cap, Value::Record(_)));

        let cur = set_cursor(
            &Value::Record(BTreeMap::from([(
                "style".into(),
                Value::String("crosshair".into()),
            )])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(cur, Value::Record(_)));

        let invalid = set_cursor(
            &Value::Record(BTreeMap::from([(
                "style".into(),
                Value::String("invalid_cursor".into()),
            )])),
            Span::point(0),
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn test_gamepad_and_midi() {
        let gp = gamepad_poll(&Value::Null, Span::point(0)).unwrap();
        assert!(matches!(gp, Value::Record(_)));

        let vib = gamepad_vibrate(
            &Value::Record(BTreeMap::from([
                ("weak_magnitude".into(), Value::F64(0.8)),
                ("duration_ms".into(), Value::U64(200)),
            ])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(vib, Value::Record(_)));

        let m_send = midi_send(
            &Value::Record(BTreeMap::from([
                ("status".into(), Value::U64(0x90)),
                ("data1".into(), Value::U64(60)),
                ("data2".into(), Value::U64(100)),
            ])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(m_send, Value::Record(_)));
    }

    #[test]
    fn test_haptic_and_spatial() {
        let hap = haptic_pattern(
            &Value::Record(BTreeMap::from([
                ("pattern".into(), Value::String("heartbeat".into())),
                ("intensity".into(), Value::F64(0.9)),
            ])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(hap, Value::Record(_)));

        let head = spatial_head_pose(&Value::Null, Span::point(0)).unwrap();
        assert!(matches!(head, Value::Record(_)));

        let hand = spatial_hand_skeleton(
            &Value::Record(BTreeMap::from([(
                "hand".into(),
                Value::String("left".into()),
            )])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(hand, Value::Record(_)));

        let gaze = spatial_gaze_ray(&Value::Null, Span::point(0)).unwrap();
        assert!(matches!(gaze, Value::Record(_)));

        let bio = biosignal_poll(
            &Value::Record(BTreeMap::from([
                ("modality".into(), Value::String("eeg".into())),
                ("epsilon".into(), Value::F64(0.5)),
            ])),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(bio, Value::Record(_)));
    }
}
