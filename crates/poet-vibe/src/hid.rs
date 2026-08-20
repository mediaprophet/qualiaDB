//! Inbound event record ABI for HID/sensors (T41).
//!
//! Defines the structured event record that inbound HID/sensor data
//! flows through. Depth maps, EEG raw, hand skeletons must never
//! become `List<f64>` — they need a typed record with a fat-buffer
//! AssetRef.
//!
//! ## Design
//!
//! - [`InboundEvent`] is the top-level event record.
//! - [`EventKind`] classifies the event (pointer, keyboard, touch,
//!   biosignal, depth, skeleton, etc.).
//! - [`EventPayload`] carries the event data — either inline (small)
//!   or via an AssetRef (fat buffers like depth maps, EEG raw).
//! - The timestamp is `u64` nanoseconds (monotonic or Unix, depending
//!   on source).
//!
//! ## ABI rules
//!
//! - Small payloads (pointer position, key code, touch point) are
//!   inline in the record.
//! - Fat payloads (depth map, EEG raw, hand skeleton) are an AssetRef
//!   — the host holds the actual data, VibeScript sees the handle.
//! - Every event has a timestamp_ns and a source ID.
//! - Biosignal events are capability-leased and DP-filtered (T44).
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.9 T41.

use crate::span::Span;
use crate::value::{AssetRef, Value};
use std::collections::BTreeMap;

/// The kind of inbound event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Pointer move/button (mouse, trackpad, stylus).
    Pointer,
    /// Keyboard key press/release.
    Keyboard,
    /// Touch contact (start, move, end).
    Touch,
    /// Biosignal (EEG, EMG, ECG, GSR).
    Biosignal,
    /// Depth map (camera depth buffer).
    Depth,
    /// Hand/body skeleton (joint positions).
    Skeleton,
    /// IMU (accelerometer, gyroscope, magnetometer).
    Imu,
    /// Audio (raw audio samples).
    Audio,
    /// Generic sensor (temperature, humidity, pressure, etc.).
    Sensor,
    /// Assistive input (sip-and-puff, switch, Braille chord).
    Assistive,
}

impl EventKind {
    /// Get the string name of this event kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pointer => "pointer",
            Self::Keyboard => "keyboard",
            Self::Touch => "touch",
            Self::Biosignal => "biosignal",
            Self::Depth => "depth",
            Self::Skeleton => "skeleton",
            Self::Imu => "imu",
            Self::Audio => "audio",
            Self::Sensor => "sensor",
            Self::Assistive => "assistive",
        }
    }

    /// Parse from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pointer" => Some(Self::Pointer),
            "keyboard" => Some(Self::Keyboard),
            "touch" => Some(Self::Touch),
            "biosignal" => Some(Self::Biosignal),
            "depth" => Some(Self::Depth),
            "skeleton" => Some(Self::Skeleton),
            "imu" => Some(Self::Imu),
            "audio" => Some(Self::Audio),
            "sensor" => Some(Self::Sensor),
            "assistive" => Some(Self::Assistive),
            _ => None,
        }
    }
}

/// The payload of an inbound event — either inline data or a fat
/// buffer reference.
#[derive(Debug, Clone)]
pub enum EventPayload {
    /// Empty payload (e.g. a key release with no additional data).
    Empty,
    /// Inline scalar values (pointer x/y, key code, touch point).
    Inline(BTreeMap<String, Value>),
    /// Fat buffer reference (depth map, EEG raw, skeleton data).
    /// The host holds the actual data; VibeScript sees the AssetRef.
    FatBuffer(AssetRef),
}

impl EventPayload {
    /// Create an inline payload with a single key-value pair.
    pub fn inline_kv(key: &str, val: Value) -> Self {
        let mut m = BTreeMap::new();
        m.insert(key.to_string(), val);
        Self::Inline(m)
    }

    /// Create an inline payload from a slice of key-value pairs.
    pub fn inline_pairs(pairs: &[(&str, Value)]) -> Self {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v.clone());
        }
        Self::Inline(m)
    }

    /// Create a fat buffer payload.
    pub fn fat_buffer(iri: &str, hash: u64) -> Self {
        Self::FatBuffer(AssetRef {
            iri: iri.to_string(),
            hash,
        })
    }

    /// Is this payload empty?
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Is this payload inline?
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline(_))
    }

    /// Is this payload a fat buffer?
    pub fn is_fat_buffer(&self) -> bool {
        matches!(self, Self::FatBuffer(_))
    }
}

/// An inbound event record — the structured ABI for HID/sensor data.
///
/// Every inbound event has:
/// - A timestamp (nanoseconds, monotonic or Unix).
/// - A source ID (which device/sensor produced this event).
/// - An event kind (pointer, keyboard, touch, etc.).
/// - A payload (inline data or fat buffer reference).
/// - An optional capability lease ID (for biosignals and other
///   capability-leased sources).
#[derive(Debug, Clone)]
pub struct InboundEvent {
    /// Timestamp in nanoseconds (monotonic or Unix, depending on source).
    pub timestamp_ns: u64,
    /// Source device/sensor ID (e.g. "mouse:0", "keyboard:0", "eeg:0").
    pub source_id: String,
    /// The kind of event.
    pub kind: EventKind,
    /// The event payload (inline or fat buffer).
    pub payload: EventPayload,
    /// Optional capability lease ID (for biosignals and other leased sources).
    pub capability_lease: Option<u64>,
    /// The span where this event was triggered (if from VibeScript).
    pub span: Span,
}

impl InboundEvent {
    /// Create a new inbound event.
    pub fn new(timestamp_ns: u64, source_id: &str, kind: EventKind, payload: EventPayload) -> Self {
        Self {
            timestamp_ns,
            source_id: source_id.to_string(),
            kind,
            payload,
            capability_lease: None,
            span: Span::point(0),
        }
    }

    /// Set the capability lease ID.
    pub fn with_lease(mut self, lease_id: u64) -> Self {
        self.capability_lease = Some(lease_id);
        self
    }

    /// Set the span.
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// Convert to a VibeScript Record value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("timestamp_ns".into(), Value::U64(self.timestamp_ns));
        rec.insert("source_id".into(), Value::String(self.source_id.clone()));
        rec.insert("kind".into(), Value::String(self.kind.as_str().to_string()));
        rec.insert(
            "payload".into(),
            match &self.payload {
                EventPayload::Empty => Value::Null,
                EventPayload::Inline(m) => Value::Record(m.clone()),
                EventPayload::FatBuffer(ar) => Value::AssetRef(ar.clone()),
            },
        );
        if let Some(lease) = self.capability_lease {
            rec.insert("capability_lease".into(), Value::U64(lease));
        }
        Value::Record(rec)
    }

    /// Create a pointer move event.
    pub fn pointer_move(timestamp_ns: u64, source_id: &str, x: f64, y: f64) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Pointer,
            EventPayload::inline_pairs(&[
                ("x", Value::F64(x)),
                ("y", Value::F64(y)),
                ("button", Value::I64(0)), // no button
                ("state", Value::String("move".into())),
            ]),
        )
    }

    /// Create a pointer button event.
    pub fn pointer_button(
        timestamp_ns: u64,
        source_id: &str,
        x: f64,
        y: f64,
        button: i64,
        pressed: bool,
    ) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Pointer,
            EventPayload::inline_pairs(&[
                ("x", Value::F64(x)),
                ("y", Value::F64(y)),
                ("button", Value::I64(button)),
                (
                    "state",
                    Value::String(if pressed { "down" } else { "up" }.into()),
                ),
            ]),
        )
    }

    /// Create a keyboard event.
    pub fn keyboard(timestamp_ns: u64, source_id: &str, key_code: i64, pressed: bool) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Keyboard,
            EventPayload::inline_pairs(&[
                ("key_code", Value::I64(key_code)),
                (
                    "state",
                    Value::String(if pressed { "down" } else { "up" }.into()),
                ),
            ]),
        )
    }

    /// Create a touch event.
    pub fn touch(
        timestamp_ns: u64,
        source_id: &str,
        touch_id: i64,
        x: f64,
        y: f64,
        phase: &str,
    ) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Touch,
            EventPayload::inline_pairs(&[
                ("touch_id", Value::I64(touch_id)),
                ("x", Value::F64(x)),
                ("y", Value::F64(y)),
                ("phase", Value::String(phase.into())),
            ]),
        )
    }

    /// Create a biosignal event (with capability lease).
    pub fn biosignal(
        timestamp_ns: u64,
        source_id: &str,
        iri: &str,
        hash: u64,
        lease_id: u64,
    ) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Biosignal,
            EventPayload::fat_buffer(iri, hash),
        )
        .with_lease(lease_id)
    }

    /// Create a depth map event (fat buffer).
    pub fn depth(timestamp_ns: u64, source_id: &str, iri: &str, hash: u64) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Depth,
            EventPayload::fat_buffer(iri, hash),
        )
    }

    // ── T43: Assistive I/O constructors ──────────────────────────────

    /// Create a sip-and-puff event (assistive input).
    /// `strength` is 0.0–1.0 (no puff → hard puff).
    /// `duration_ms` is the duration of the sip/puff in milliseconds.
    pub fn sip_and_puff(
        timestamp_ns: u64,
        source_id: &str,
        strength: f64,
        duration_ms: u64,
    ) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Assistive,
            EventPayload::inline_pairs(&[
                ("device", Value::String("sip_puff".into())),
                ("strength", Value::F64(strength.clamp(0.0, 1.0))),
                ("duration_ms", Value::U64(duration_ms)),
            ]),
        )
    }

    /// Create a switch event (assistive input — single switch, binary).
    pub fn switch_event(timestamp_ns: u64, source_id: &str, switch_id: i64, pressed: bool) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Assistive,
            EventPayload::inline_pairs(&[
                ("device", Value::String("switch".into())),
                ("switch_id", Value::I64(switch_id)),
                (
                    "state",
                    Value::String(if pressed { "down" } else { "up" }.into()),
                ),
            ]),
        )
    }

    /// Create a Braille chord event (assistive input).
    /// `dots` is a bitmask of the 8 Braille dots (bits 0–7).
    pub fn braille_chord(timestamp_ns: u64, source_id: &str, dots: u8) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Assistive,
            EventPayload::inline_pairs(&[
                ("device", Value::String("braille".into())),
                ("dots", Value::I64(dots as i64)),
            ]),
        )
    }

    /// Create an eye-gaze event (assistive input — eye tracking).
    pub fn eye_gaze(timestamp_ns: u64, source_id: &str, x: f64, y: f64, fixation_ms: u64) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Assistive,
            EventPayload::inline_pairs(&[
                ("device", Value::String("eye_gaze".into())),
                ("x", Value::F64(x)),
                ("y", Value::F64(y)),
                ("fixation_ms", Value::U64(fixation_ms)),
            ]),
        )
    }
}

/// Outbound cue kinds for assistive output (T43/T45).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueKind {
    /// Haptic feedback (vibration, force feedback).
    Haptic,
    /// Audio cue (tone, earcon, speech).
    Audio,
    /// Visual cue (flash, border highlight).
    Visual,
    /// Accessibility cue (screen reader announcement, Braille output).
    Accessibility,
}

impl CueKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Haptic => "Haptic",
            Self::Audio => "Audio",
            Self::Visual => "Visual",
            Self::Accessibility => "Accessibility",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Haptic" => Some(Self::Haptic),
            "Audio" => Some(Self::Audio),
            "Visual" => Some(Self::Visual),
            "Accessibility" => Some(Self::Accessibility),
            _ => None,
        }
    }
}

/// An outbound cue — haptic, audio, visual, or accessibility output.
#[derive(Debug, Clone)]
pub struct OutboundCue {
    /// The cue kind.
    pub kind: CueKind,
    /// The cue name (e.g. "Haptic.buzz", "Audio.earcon.success",
    /// "Accessibility.announce", "Visual.flash").
    pub name: String,
    /// The cue payload (inline parameters).
    pub payload: BTreeMap<String, Value>,
}

impl OutboundCue {
    /// Create a haptic buzz cue.
    pub fn haptic_buzz(duration_ms: u64, strength: f64) -> Self {
        let mut payload = BTreeMap::new();
        payload.insert("duration_ms".into(), Value::U64(duration_ms));
        payload.insert("strength".into(), Value::F64(strength.clamp(0.0, 1.0)));
        Self {
            kind: CueKind::Haptic,
            name: "Haptic.buzz".into(),
            payload,
        }
    }

    /// Create an audio earcon cue.
    pub fn audio_earcon(earcon_id: &str) -> Self {
        let mut payload = BTreeMap::new();
        payload.insert("earcon_id".into(), Value::String(earcon_id.into()));
        Self {
            kind: CueKind::Audio,
            name: "Audio.earcon".into(),
            payload,
        }
    }

    /// Create a screen reader announcement cue.
    pub fn accessibility_announce(message: &str) -> Self {
        let mut payload = BTreeMap::new();
        payload.insert("message".into(), Value::String(message.into()));
        Self {
            kind: CueKind::Accessibility,
            name: "Accessibility.announce".into(),
            payload,
        }
    }

    /// Create a visual flash cue.
    pub fn visual_flash(color: &str, duration_ms: u64) -> Self {
        let mut payload = BTreeMap::new();
        payload.insert("color".into(), Value::String(color.into()));
        payload.insert("duration_ms".into(), Value::U64(duration_ms));
        Self {
            kind: CueKind::Visual,
            name: "Visual.flash".into(),
            payload,
        }
    }

    /// Convert to a VibeScript cue.post argument value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("kind".into(), Value::String(self.kind.as_str().into()));
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("payload".into(), Value::Record(self.payload.clone()));
        Value::Record(rec)
    }

    /// Get the cue ID string for cue.post (e.g. "Haptic.buzz").
    pub fn cue_id(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_as_str() {
        assert_eq!(EventKind::Pointer.as_str(), "pointer");
        assert_eq!(EventKind::Keyboard.as_str(), "keyboard");
        assert_eq!(EventKind::Touch.as_str(), "touch");
        assert_eq!(EventKind::Biosignal.as_str(), "biosignal");
        assert_eq!(EventKind::Depth.as_str(), "depth");
    }

    #[test]
    fn event_kind_from_str() {
        assert_eq!(EventKind::from_str("pointer"), Some(EventKind::Pointer));
        assert_eq!(EventKind::from_str("keyboard"), Some(EventKind::Keyboard));
        assert_eq!(EventKind::from_str("touch"), Some(EventKind::Touch));
        assert_eq!(EventKind::from_str("biosignal"), Some(EventKind::Biosignal));
        assert_eq!(EventKind::from_str("depth"), Some(EventKind::Depth));
        assert_eq!(EventKind::from_str("unknown"), None);
    }

    #[test]
    fn event_kind_round_trip() {
        for kind in [
            EventKind::Pointer,
            EventKind::Keyboard,
            EventKind::Touch,
            EventKind::Biosignal,
            EventKind::Depth,
            EventKind::Skeleton,
            EventKind::Imu,
            EventKind::Audio,
            EventKind::Sensor,
            EventKind::Assistive,
        ] {
            let s = kind.as_str();
            assert_eq!(EventKind::from_str(s), Some(kind));
        }
    }

    #[test]
    fn payload_empty() {
        let p = EventPayload::Empty;
        assert!(p.is_empty());
        assert!(!p.is_inline());
        assert!(!p.is_fat_buffer());
    }

    #[test]
    fn payload_inline() {
        let p = EventPayload::inline_kv("x", Value::F64(1.0));
        assert!(p.is_inline());
        assert!(!p.is_empty());
        assert!(!p.is_fat_buffer());
    }

    #[test]
    fn payload_fat_buffer() {
        let p = EventPayload::fat_buffer("asset:eeg:0", 123);
        assert!(p.is_fat_buffer());
        assert!(!p.is_empty());
        assert!(!p.is_inline());
    }

    #[test]
    fn inbound_event_pointer_move() {
        let e = InboundEvent::pointer_move(1000, "mouse:0", 10.5, 20.5);
        assert_eq!(e.timestamp_ns, 1000);
        assert_eq!(e.source_id, "mouse:0");
        assert_eq!(e.kind, EventKind::Pointer);
        assert!(e.payload.is_inline());
        assert!(e.capability_lease.is_none());
    }

    #[test]
    fn inbound_event_pointer_button() {
        let e = InboundEvent::pointer_button(2000, "mouse:0", 10.0, 20.0, 1, true);
        assert_eq!(e.kind, EventKind::Pointer);
        assert!(e.payload.is_inline());
    }

    #[test]
    fn inbound_event_keyboard() {
        let e = InboundEvent::keyboard(3000, "keyboard:0", 65, true);
        assert_eq!(e.kind, EventKind::Keyboard);
        assert_eq!(e.source_id, "keyboard:0");
        assert!(e.payload.is_inline());
    }

    #[test]
    fn inbound_event_touch() {
        let e = InboundEvent::touch(4000, "touch:0", 0, 100.0, 200.0, "start");
        assert_eq!(e.kind, EventKind::Touch);
        assert!(e.payload.is_inline());
    }

    #[test]
    fn inbound_event_biosignal_has_lease() {
        let e = InboundEvent::biosignal(5000, "eeg:0", "asset:eeg:raw", 999, 42);
        assert_eq!(e.kind, EventKind::Biosignal);
        assert!(e.payload.is_fat_buffer());
        assert_eq!(e.capability_lease, Some(42));
    }

    #[test]
    fn inbound_event_depth_is_fat_buffer() {
        let e = InboundEvent::depth(6000, "depth:0", "asset:depth:0", 777);
        assert_eq!(e.kind, EventKind::Depth);
        assert!(e.payload.is_fat_buffer());
        assert!(e.capability_lease.is_none());
    }

    #[test]
    fn inbound_event_to_value() {
        let e = InboundEvent::pointer_move(1000, "mouse:0", 10.5, 20.5);
        let v = e.to_value();
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("timestamp_ns").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            1000
        );
        assert_eq!(
            match rec.get("source_id").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "mouse:0"
        );
        assert_eq!(
            match rec.get("kind").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "pointer"
        );
    }

    #[test]
    fn inbound_event_to_value_with_lease() {
        let e = InboundEvent::biosignal(5000, "eeg:0", "asset:eeg:raw", 999, 42);
        let v = e.to_value();
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("capability_lease").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            42
        );
    }

    #[test]
    fn inbound_event_to_value_fat_buffer() {
        let e = InboundEvent::depth(6000, "depth:0", "asset:depth:0", 777);
        let v = e.to_value();
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let payload = rec.get("payload").unwrap();
        assert!(payload.as_asset_ref().is_some());
    }

    #[test]
    fn inbound_event_with_lease_builder() {
        let e = InboundEvent::keyboard(1000, "kb:0", 65, true).with_lease(99);
        assert_eq!(e.capability_lease, Some(99));
    }

    #[test]
    fn inbound_event_with_span_builder() {
        let e = InboundEvent::keyboard(1000, "kb:0", 65, true).with_span(Span::new(10, 20));
        assert_eq!(e.span.start, 10);
        assert_eq!(e.span.end, 20);
    }

    #[test]
    fn payload_inline_pairs() {
        let p = EventPayload::inline_pairs(&[
            ("a", Value::I64(1)),
            ("b", Value::F64(2.0)),
            ("c", Value::String("hi".into())),
        ]);
        assert!(p.is_inline());
        if let EventPayload::Inline(m) = p {
            assert_eq!(m.len(), 3);
        }
    }

    // ── T43: Assistive I/O tests ─────────────────────────────────────

    #[test]
    fn t43_sip_and_puff_event() {
        let e = InboundEvent::sip_and_puff(1000, "sip_puff:0", 0.7, 200);
        assert_eq!(e.kind, EventKind::Assistive);
        assert!(e.payload.is_inline());
        if let EventPayload::Inline(m) = &e.payload {
            assert_eq!(
                match m.get("device").unwrap() {
                    Value::String(s) => s.as_str(),
                    _ => panic!("expected String"),
                },
                "sip_puff"
            );
            assert_eq!(
                match m.get("strength").unwrap() {
                    Value::F64(f) => *f,
                    _ => panic!("expected F64"),
                },
                0.7
            );
        }
    }

    #[test]
    fn t43_sip_and_puff_clamps_strength() {
        let e = InboundEvent::sip_and_puff(1000, "sip_puff:0", 1.5, 100);
        if let EventPayload::Inline(m) = &e.payload {
            assert_eq!(
                match m.get("strength").unwrap() {
                    Value::F64(f) => *f,
                    _ => panic!("expected F64"),
                },
                1.0
            ); // clamped
        }
    }

    #[test]
    fn t43_switch_event() {
        let e = InboundEvent::switch_event(2000, "switch:0", 1, true);
        assert_eq!(e.kind, EventKind::Assistive);
        if let EventPayload::Inline(m) = &e.payload {
            assert_eq!(
                match m.get("switch_id").unwrap() {
                    Value::I64(n) => *n,
                    _ => panic!("expected I64"),
                },
                1
            );
            assert_eq!(
                match m.get("state").unwrap() {
                    Value::String(s) => s.as_str(),
                    _ => panic!("expected String"),
                },
                "down"
            );
        }
    }

    #[test]
    fn t43_braille_chord_event() {
        let e = InboundEvent::braille_chord(3000, "braille:0", 0b00010111);
        assert_eq!(e.kind, EventKind::Assistive);
        if let EventPayload::Inline(m) = &e.payload {
            assert_eq!(
                match m.get("dots").unwrap() {
                    Value::I64(n) => *n,
                    _ => panic!("expected I64"),
                },
                0b00010111
            );
        }
    }

    #[test]
    fn t43_eye_gaze_event() {
        let e = InboundEvent::eye_gaze(4000, "eye:0", 150.0, 200.0, 500);
        assert_eq!(e.kind, EventKind::Assistive);
        if let EventPayload::Inline(m) = &e.payload {
            assert_eq!(
                match m.get("device").unwrap() {
                    Value::String(s) => s.as_str(),
                    _ => panic!("expected String"),
                },
                "eye_gaze"
            );
            assert_eq!(
                match m.get("fixation_ms").unwrap() {
                    Value::U64(n) => *n,
                    _ => panic!("expected U64"),
                },
                500
            );
        }
    }

    #[test]
    fn t43_cue_kind_round_trip() {
        for kind in [
            CueKind::Haptic,
            CueKind::Audio,
            CueKind::Visual,
            CueKind::Accessibility,
        ] {
            let s = kind.as_str();
            assert_eq!(CueKind::from_str(s), Some(kind));
        }
    }

    #[test]
    fn t43_haptic_buzz_cue() {
        let cue = OutboundCue::haptic_buzz(100, 0.5);
        assert_eq!(cue.kind, CueKind::Haptic);
        assert_eq!(cue.cue_id(), "Haptic.buzz");
        assert_eq!(cue.payload.len(), 2);
    }

    #[test]
    fn t43_audio_earcon_cue() {
        let cue = OutboundCue::audio_earcon("success");
        assert_eq!(cue.kind, CueKind::Audio);
        assert_eq!(cue.cue_id(), "Audio.earcon");
    }

    #[test]
    fn t43_accessibility_announce_cue() {
        let cue = OutboundCue::accessibility_announce("Form submitted");
        assert_eq!(cue.kind, CueKind::Accessibility);
        assert_eq!(cue.cue_id(), "Accessibility.announce");
        if let Value::String(s) = cue.payload.get("message").unwrap() {
            assert_eq!(s, "Form submitted");
        }
    }

    #[test]
    fn t43_visual_flash_cue() {
        let cue = OutboundCue::visual_flash("red", 200);
        assert_eq!(cue.kind, CueKind::Visual);
        assert_eq!(cue.cue_id(), "Visual.flash");
    }

    #[test]
    fn t43_cue_to_value() {
        let cue = OutboundCue::haptic_buzz(100, 0.5);
        let v = cue.to_value();
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("kind").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "Haptic"
        );
        assert_eq!(
            match rec.get("name").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "Haptic.buzz"
        );
    }

    #[test]
    fn t43_haptic_buzz_clamps_strength() {
        let cue = OutboundCue::haptic_buzz(100, 2.0);
        if let Value::F64(f) = cue.payload.get("strength").unwrap() {
            assert_eq!(*f, 1.0); // clamped
        }
    }
}
