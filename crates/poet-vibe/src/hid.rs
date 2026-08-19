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
    pub fn new(
        timestamp_ns: u64,
        source_id: &str,
        kind: EventKind,
        payload: EventPayload,
    ) -> Self {
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
    pub fn pointer_button(timestamp_ns: u64, source_id: &str, x: f64, y: f64, button: i64, pressed: bool) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Pointer,
            EventPayload::inline_pairs(&[
                ("x", Value::F64(x)),
                ("y", Value::F64(y)),
                ("button", Value::I64(button)),
                ("state", Value::String(if pressed { "down" } else { "up" }.into())),
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
                ("state", Value::String(if pressed { "down" } else { "up" }.into())),
            ]),
        )
    }

    /// Create a touch event.
    pub fn touch(timestamp_ns: u64, source_id: &str, touch_id: i64, x: f64, y: f64, phase: &str) -> Self {
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
    pub fn biosignal(timestamp_ns: u64, source_id: &str, iri: &str, hash: u64, lease_id: u64) -> Self {
        Self::new(
            timestamp_ns,
            source_id,
            EventKind::Biosignal,
            EventPayload::fat_buffer(iri, hash),
        ).with_lease(lease_id)
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
            EventKind::Pointer, EventKind::Keyboard, EventKind::Touch,
            EventKind::Biosignal, EventKind::Depth, EventKind::Skeleton,
            EventKind::Imu, EventKind::Audio, EventKind::Sensor,
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
}
