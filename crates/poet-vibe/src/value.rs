//! Runtime values.

use std::collections::BTreeMap;

/// Time scale for an [`Value::Instant`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimeScale {
    /// Unix epoch seconds + nanos.
    Unix,
    /// International Atomic Time.
    Tai,
    /// GPS time.
    Gps,
    /// Monotonic clock (never goes backwards; epoch arbitrary).
    Monotonic,
    /// Proper time along a worldline (from 10D manifold metric).
    Proper,
}

/// A point in time with a known scale, exact secs+nanos, and an optional
/// cryptographic seal (T1).
///
/// Replaces `time.unix() -> i64` as the primitive. The seal is a
/// signature over `(scale, secs, nanos, frame)` proving the instant was
/// asserted by a specific authority.
#[derive(Debug, Clone, PartialEq)]
pub struct Instant {
    pub scale: TimeScale,
    pub secs: i64,
    pub nanos: u32,
    /// Optional reference frame (e.g. a worldline ID or observer DID).
    pub frame: Option<String>,
    /// Optional ed25519 signature over the instant's canonical encoding.
    pub seal: Option<[u8; 64]>,
}

/// An exact duration: secs + nanos. No float subtraction (T2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Duration {
    pub secs: i64,
    pub nanos: u32,
}

/// A physical quantity: a value with a required unit IRI (T3).
///
/// Mixing `kPa` and `Pa` without conversion is a type error. Dimensionless
/// quantities have an explicit empty unit IRI.
#[derive(Debug, Clone, PartialEq)]
pub struct Quantity {
    pub value: f64,
    /// Unit IRI (e.g. `"qudt:KiloPascal"`). Empty string = dimensionless.
    pub unit: String,
}

/// A reference frame: origin + basis vectors (T4).
///
/// Local by default. A morphism, not a naked mat4.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Origin as [x, y, z, t] (or fewer dimensions as needed).
    pub origin: Vec<f64>,
    /// Basis vectors as rows of a matrix. Empty = identity.
    pub basis: Vec<Vec<f64>>,
    /// Optional parent frame IRI (for nested frames).
    pub parent: Option<String>,
}

/// A pose: position + orientation within a frame (T4).
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    pub position: Vec<f64>,
    /// Orientation as a quaternion [w, x, y, z] or rotation matrix.
    pub orientation: Vec<f64>,
    /// The frame this pose is relative to.
    pub frame: Option<String>,
}

/// A transform: translation + rotation + scale (T4).
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub translate: Vec<f64>,
    pub rotate: Vec<f64>,
    pub scale: Vec<f64>,
    pub skew: Vec<f64>,
}

/// An opaque handle to a sampled field (T5). Scripts do not see the grid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldRef {
    /// The field's IRI identifier.
    pub iri: String,
    /// A content hash of the field data (for provenance).
    pub hash: u64,
}

/// An opaque handle to a material signature (T5).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialRef {
    /// The material's IRI identifier.
    pub iri: String,
    /// A content hash of the material definition.
    pub hash: u64,
}

/// A worldline: a continuant through Instant × Pose (T6).
///
/// Kills UUID-as-identity. A worldline is the identity of a thing that
/// persists through time, anchored to its spatiotemporal trajectory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorldLine {
    /// Stable IRI for this worldline.
    pub iri: String,
    /// The principal or observer DID that asserted this worldline.
    pub asserted_by: String,
    /// Creation instant (Unix secs).
    pub created_at: i64,
}

/// A user-defined enum value (T9). Carries the enum name, variant name,
/// and optional payload values. Unit variants have an empty payload.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    pub enum_name: String,
    pub variant_name: String,
    pub payload: Vec<Value>,
}

impl EnumValue {
    pub fn unit(enum_name: &str, variant_name: &str) -> Self {
        Self {
            enum_name: enum_name.to_string(),
            variant_name: variant_name.to_string(),
            payload: Vec::new(),
        }
    }

    pub fn with_payload(enum_name: &str, variant_name: &str, payload: Vec<Value>) -> Self {
        Self {
            enum_name: enum_name.to_string(),
            variant_name: variant_name.to_string(),
            payload,
        }
    }
}

/// An opaque 48-byte handle to a Super-Quin / NQuin (T7).
///
/// Scripts do not see the raw `subject`/`predicate`/`object`/`context`/
/// `metadata`/`parity` fields. The handle is a content-addressed reference
/// that the host resolves. This replaces `Value::Quin { s, p, o, c }` as
/// the preferred representation — `Quin` remains for backward compatibility
/// but new code should use `QuinRef`.
///
/// The 6 × `u64` fields are the full NQuin payload (subject, predicate,
/// object, context, metadata, parity) but they are opaque to scripts —
/// the `as_quin_ref()` accessor returns the handle, not the fields.
/// Host code can access the raw fields via `raw_fields()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuinRef {
    /// Opaque payload — 6 × u64 = 48 bytes. Scripts must not access this
    /// directly. The host resolves the handle to graph data.
    payload: [u64; 6],
}

impl QuinRef {
    /// Create a QuinRef from raw NQuin fields (host-side only).
    pub fn from_raw(subject: u64, predicate: u64, object: u64, context: u64, metadata: u64, parity: u64) -> Self {
        Self {
            payload: [subject, predicate, object, context, metadata, parity],
        }
    }

    /// Create a QuinRef from a `Value::Quin` (which has 4 fields; metadata
    /// and parity are computed).
    pub fn from_quin(subject: u64, predicate: u64, object: u64, context: u64) -> Self {
        let metadata = 0u64;
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        Self::from_raw(subject, predicate, object, context, metadata, parity)
    }

    /// Access the raw 6 × u64 payload (host-side only). Scripts never
    /// see this — it's for the host to resolve the handle to graph data.
    pub fn raw_fields(&self) -> [u64; 6] {
        self.payload
    }

    /// The content hash of this QuinRef — used for deduplication and
    /// provenance tracking. Position-dependent (FNV-1a style).
    pub fn content_hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for &v in &self.payload {
            h ^= v;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Iri(String),
    Blank(String),
    Prefixed(String, String),
    Var(String),
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Triple(Box<Value>, Box<Value>, Box<Value>),
    Reified {
        s: Box<Value>,
        p: Box<Value>,
        o: Box<Value>,
        r: Box<Value>,
    },
    Quin {
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
    },
    Receipt,
    Ok(Box<Value>),
    Err(Box<Value>),
    // ── T1–T6: Type lattice extensions ────────────────────────────────
    /// A point in time with scale, exact secs+nanos, and optional seal (T1).
    Instant(Instant),
    /// An exact duration: secs + nanos (T2).
    Duration(Duration),
    /// A physical quantity with a required unit IRI (T3).
    Quantity(Quantity),
    /// A reference frame: origin + basis (T4).
    Frame(Frame),
    /// A pose within a frame (T4).
    Pose(Pose),
    /// A transform: translate/rotate/scale/skew (T4).
    Transform(Transform),
    /// An opaque handle to a sampled field (T5).
    FieldRef(FieldRef),
    /// An opaque handle to a material signature (T5).
    MaterialRef(MaterialRef),
    /// A worldline: continuant through Instant × Pose (T6).
    WorldLine(WorldLine),
    /// An opaque 48-byte handle to a Super-Quin (T7). Scripts do not see
    /// the raw s/p/o/c/metadata/parity fields.
    QuinRef(QuinRef),
    /// A user-defined enum value (T9).
    Enum(EnumValue),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::I64(0) | Value::U64(0) => false,
            _ => true,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            Value::F64(n) => Some(*n),
            Value::Quantity(q) => Some(q.value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(n) => Some(*n),
            Value::U64(n) if *n <= i64::MAX as u64 => Some(*n as i64),
            Value::F64(n) if n.fract() == 0.0 => Some(*n as i64),
            _ => None,
        }
    }

    /// Extract an Instant if this value is one.
    pub fn as_instant(&self) -> Option<&Instant> {
        match self {
            Value::Instant(i) => Some(i),
            _ => None,
        }
    }

    /// Extract a Duration if this value is one.
    pub fn as_duration(&self) -> Option<&Duration> {
        match self {
            Value::Duration(d) => Some(d),
            _ => None,
        }
    }

    /// Extract a Quantity if this value is one.
    pub fn as_quantity(&self) -> Option<&Quantity> {
        match self {
            Value::Quantity(q) => Some(q),
            _ => None,
        }
    }

    /// Extract a Frame if this value is one.
    pub fn as_frame(&self) -> Option<&Frame> {
        match self {
            Value::Frame(f) => Some(f),
            _ => None,
        }
    }

    /// Extract a Pose if this value is one.
    pub fn as_pose(&self) -> Option<&Pose> {
        match self {
            Value::Pose(p) => Some(p),
            _ => None,
        }
    }

    /// Extract a Transform if this value is one.
    pub fn as_transform(&self) -> Option<&Transform> {
        match self {
            Value::Transform(t) => Some(t),
            _ => None,
        }
    }

    /// Extract a FieldRef if this value is one.
    pub fn as_field_ref(&self) -> Option<&FieldRef> {
        match self {
            Value::FieldRef(f) => Some(f),
            _ => None,
        }
    }

    /// Extract a MaterialRef if this value is one.
    pub fn as_material_ref(&self) -> Option<&MaterialRef> {
        match self {
            Value::MaterialRef(m) => Some(m),
            _ => None,
        }
    }

    /// Extract a WorldLine if this value is one.
    pub fn as_worldline(&self) -> Option<&WorldLine> {
        match self {
            Value::WorldLine(w) => Some(w),
            _ => None,
        }
    }

    /// Extract a QuinRef if this value is one (T7).
    pub fn as_quin_ref(&self) -> Option<&QuinRef> {
        match self {
            Value::QuinRef(q) => Some(q),
            _ => None,
        }
    }

    /// Extract an EnumValue if this value is one (T9).
    pub fn as_enum(&self) -> Option<&EnumValue> {
        match self {
            Value::Enum(e) => Some(e),
            _ => None,
        }
    }
}

// ── Convenience constructors ───────────────────────────────────────────────

impl Instant {
    pub fn unix(secs: i64, nanos: u32) -> Self {
        Self {
            scale: TimeScale::Unix,
            secs,
            nanos,
            frame: None,
            seal: None,
        }
    }

    pub fn monotonic(nanos: u64) -> Self {
        Self {
            scale: TimeScale::Monotonic,
            secs: (nanos / 1_000_000_000) as i64,
            nanos: (nanos % 1_000_000_000) as u32,
            frame: None,
            seal: None,
        }
    }
}

impl Duration {
    pub fn from_secs(secs: i64) -> Self {
        Self { secs, nanos: 0 }
    }

    pub fn from_nanos(nanos: i64) -> Self {
        Self {
            secs: nanos / 1_000_000_000,
            nanos: (nanos % 1_000_000_000) as u32,
        }
    }

    /// Total nanoseconds (may overflow for very large durations).
    pub fn total_nanos(&self) -> i128 {
        self.secs as i128 * 1_000_000_000 + self.nanos as i128
    }
}

impl Quantity {
    pub fn new(value: f64, unit: &str) -> Self {
        Self {
            value,
            unit: unit.to_string(),
        }
    }

    pub fn dimensionless(value: f64) -> Self {
        Self {
            value,
            unit: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_unix_constructor() {
        let i = Instant::unix(1000, 500_000);
        assert_eq!(i.scale, TimeScale::Unix);
        assert_eq!(i.secs, 1000);
        assert_eq!(i.nanos, 500_000);
        assert!(i.seal.is_none());
    }

    #[test]
    fn instant_monotonic_constructor() {
        let i = Instant::monotonic(1_500_000_000);
        assert_eq!(i.scale, TimeScale::Monotonic);
        assert_eq!(i.secs, 1);
        assert_eq!(i.nanos, 500_000_000);
    }

    #[test]
    fn duration_from_secs() {
        let d = Duration::from_secs(60);
        assert_eq!(d.secs, 60);
        assert_eq!(d.nanos, 0);
    }

    #[test]
    fn duration_from_nanos() {
        let d = Duration::from_nanos(1_500_000_000);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn duration_total_nanos() {
        let d = Duration { secs: 2, nanos: 500_000_000 };
        assert_eq!(d.total_nanos(), 2_500_000_000);
    }

    #[test]
    fn quantity_dimensionless() {
        let q = Quantity::dimensionless(42.0);
        assert_eq!(q.value, 42.0);
        assert_eq!(q.unit, "");
    }

    #[test]
    fn quantity_with_unit() {
        let q = Quantity::new(101.325, "qudt:KiloPascal");
        assert_eq!(q.value, 101.325);
        assert_eq!(q.unit, "qudt:KiloPascal");
    }

    #[test]
    fn value_instant_extract() {
        let v = Value::Instant(Instant::unix(100, 0));
        let i = v.as_instant().unwrap();
        assert_eq!(i.secs, 100);
    }

    #[test]
    fn value_duration_extract() {
        let v = Value::Duration(Duration::from_secs(30));
        let d = v.as_duration().unwrap();
        assert_eq!(d.secs, 30);
    }

    #[test]
    fn value_quantity_extract() {
        let v = Value::Quantity(Quantity::new(1.0, "qudt:Meter"));
        let q = v.as_quantity().unwrap();
        assert_eq!(q.unit, "qudt:Meter");
    }

    #[test]
    fn value_quantity_as_f64() {
        let v = Value::Quantity(Quantity::new(42.5, "qudt:KiloPascal"));
        assert_eq!(v.as_f64(), Some(42.5));
    }

    #[test]
    fn value_frame_extract() {
        let f = Frame {
            origin: vec![0.0, 0.0, 0.0],
            basis: vec![],
            parent: None,
        };
        let v = Value::Frame(f);
        assert!(v.as_frame().is_some());
    }

    #[test]
    fn value_pose_extract() {
        let p = Pose {
            position: vec![1.0, 2.0, 3.0],
            orientation: vec![1.0, 0.0, 0.0, 0.0],
            frame: None,
        };
        let v = Value::Pose(p);
        assert!(v.as_pose().is_some());
    }

    #[test]
    fn value_transform_extract() {
        let t = Transform {
            translate: vec![1.0, 0.0],
            rotate: vec![0.0],
            scale: vec![1.0, 1.0],
            skew: vec![0.0, 0.0],
        };
        let v = Value::Transform(t);
        assert!(v.as_transform().is_some());
    }

    #[test]
    fn value_field_ref_extract() {
        let f = FieldRef {
            iri: "field:ambient_pressure".to_string(),
            hash: 0xDEAD_BEEF,
        };
        let v = Value::FieldRef(f);
        let fr = v.as_field_ref().unwrap();
        assert_eq!(fr.iri, "field:ambient_pressure");
    }

    #[test]
    fn value_material_ref_extract() {
        let m = MaterialRef {
            iri: "material:sucrose".to_string(),
            hash: 0xCAFEBABE,
        };
        let v = Value::MaterialRef(m);
        let mr = v.as_material_ref().unwrap();
        assert_eq!(mr.iri, "material:sucrose");
    }

    #[test]
    fn value_worldline_extract() {
        let w = WorldLine {
            iri: "worldline:observer-1".to_string(),
            asserted_by: "did:qualia:root:alice".to_string(),
            created_at: 1000,
        };
        let v = Value::WorldLine(w);
        let wl = v.as_worldline().unwrap();
        assert_eq!(wl.iri, "worldline:observer-1");
    }

    #[test]
    fn value_quantity_truthy() {
        // A quantity with non-zero value is truthy.
        let v = Value::Quantity(Quantity::new(0.0, "qudt:Meter"));
        // Zero quantity — is_truthy returns true for non-I64/U64/Null/Bool(false).
        assert!(v.is_truthy());
    }

    // ── T7: QuinRef ──────────────────────────────────────────────────────

    #[test]
    fn quin_ref_from_raw() {
        let q = QuinRef::from_raw(1, 2, 3, 4, 5, 6);
        let fields = q.raw_fields();
        assert_eq!(fields, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn quin_ref_from_quin_computes_parity() {
        let q = QuinRef::from_quin(10, 20, 30, 40);
        let fields = q.raw_fields();
        // metadata = 0, parity = 10 ^ 20 ^ 30 ^ 40 ^ 0 = 10 ^ 20 ^ 30 ^ 40
        assert_eq!(fields[0], 10);
        assert_eq!(fields[1], 20);
        assert_eq!(fields[2], 30);
        assert_eq!(fields[3], 40);
        assert_eq!(fields[4], 0); // metadata
        assert_eq!(fields[5], 10 ^ 20 ^ 30 ^ 40); // parity
    }

    #[test]
    fn quin_ref_content_hash_deterministic() {
        let q1 = QuinRef::from_raw(1, 2, 3, 4, 5, 6);
        let q2 = QuinRef::from_raw(1, 2, 3, 4, 5, 6);
        assert_eq!(q1.content_hash(), q2.content_hash());
    }

    #[test]
    fn quin_ref_different_payloads_different_hash() {
        let q1 = QuinRef::from_raw(1, 2, 3, 4, 5, 6);
        let q2 = QuinRef::from_raw(6, 5, 4, 3, 2, 1);
        assert_ne!(q1.content_hash(), q2.content_hash());
    }

    #[test]
    fn value_quin_ref_extract() {
        let q = QuinRef::from_raw(100, 200, 300, 400, 0, 0);
        let v = Value::QuinRef(q);
        let extracted = v.as_quin_ref().unwrap();
        assert_eq!(extracted.raw_fields(), [100, 200, 300, 400, 0, 0]);
    }

    #[test]
    fn value_quin_ref_is_copy() {
        let q = QuinRef::from_raw(1, 2, 3, 4, 5, 6);
        let v1 = Value::QuinRef(q);
        let v2 = v1.clone();
        // QuinRef is Copy — both should be equal.
        assert_eq!(v1, v2);
    }
}
