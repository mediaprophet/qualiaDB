//! NQuin cosmic opcodes and 48-byte metadata layout (OCS §14.2).
//!
//! The OCS defines five opcodes in the `0x50–0x55` range, packed into
//! bits `[0..7]` of the `predicate` field of an `NQuin`:
//!
//! | Opcode              | Value | Meaning                                    |
//! |---------------------|-------|--------------------------------------------|
//! | `OP_COSMIC_POSE`    | 0x50  | Entity has a spacetime pose in a realm     |
//! | `OP_REALM_TRANSIT`  | 0x51  | Entity transits between realms/frames      |
//! | `OP_OBSERVER_FIBER` | 0x53  | Observer fiber attachment to an entity     |
//! | `OP_MICRO_POSE`     | 0x54  | Microscopic-scale pose (sub-atomic)        |
//! | `OP_ELEMENT_COLLAPSE` | 0x55 | Granular element collapse onto physical    |
//!
//! These are in the `0x50+` range, above the LTL range (`0x40–0x44`)
//! and do not conflict with any existing modality.
//!
//! ## 48-byte NQuin Layout for Cosmic Operations (OCS §14.2)
//!
//! ```text
//! Field      Bits      Content
//! ─────────────────────────────────────────────────────────────────
//! subject    [63]      MSB flag (1 = did:q42 entity/observer pointer)
//!            [0..62]   q_hash(entity_or_observer_did)
//!
//! predicate  [63]      0
//!            [8..62]   q_hash("cosmic:hasSpacetimePose") << 8
//!            [0..7]    Opcode (0x50–0x55)
//!
//! object     [63]      MSB=1 (did:q42 topological pointer to Pose/Anchor)
//!            [0..62]   q_hash(pose_or_record_iri)
//!
//! context    [56..63]  Sensitivity Class (PUBLIC=0, RESTRICTED=1, CLASSIFIED=2)
//!            [0..55]   q_hash(realm_usri) (56-bit scoped realm hash)
//!
//! metadata   [61..62]  PermissiveRoutingLane (01=Commons, 10=Bilateral Isolated)
//!            [32..60]  Lamport Logical Clock / Epoch Timestamp (29 bits)
//!            [0..31]   Realm Class & Scale Level Index
//!                       bits [0..7]:  realm class index
//!                       bits [8..15]: hierarchy level (L_-2..L_12)
//!                       bits [16..31]: nesting depth + reserved
//!
//! parity     [0..63]   XOR fold: subject ^ predicate ^ object ^ context
//! ```
//!
//! Reference: OCS Specification v2.2.0 §14.2.

use super::cb_usri::{CompactBinaryUsri, HierarchyLevel};
use super::usri::RealmClass;

/// FNV-1a 64-bit hash (matching q_hash used throughout QualiaDB).
fn fnv1a_64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ── Opcode constants (OCS §14.2) ───────────────────────────────────

/// Entity has a spacetime pose in a realm.
pub const OP_COSMIC_POSE: u8 = 0x50;

/// Entity transits between realms or reference frames.
pub const OP_REALM_TRANSIT: u8 = 0x51;

/// Observer fiber attachment to an entity.
pub const OP_OBSERVER_FIBER: u8 = 0x53;

/// Microscopic-scale pose (sub-atomic / quantum realm).
pub const OP_MICRO_POSE: u8 = 0x54;

/// Granular element collapse onto physical spacetime.
pub const OP_ELEMENT_COLLAPSE: u8 = 0x55;

/// The canonical predicate IRI for cosmic pose operations.
pub const COSMIC_POSE_IRI: &str = "cosmic:hasSpacetimePose";

/// Sensitivity class constants (OCS §14.2, matching NQuin context field).
pub const SENSITIVITY_PUBLIC: u8 = 0;
pub const SENSITIVITY_RESTRICTED: u8 = 1;
pub const SENSITIVITY_CLASSIFIED: u8 = 2;

/// PermissiveRoutingLane constants (OCS §14.2, matching NQuin metadata field).
pub const LANE_PASSTHROUGH: u8 = 0b00;
pub const LANE_COMMONS: u8 = 0b01;
pub const LANE_BILATERAL: u8 = 0b10;
pub const LANE_SPATIAL: u8 = 0b11;

/// A 48-byte cosmic NQuin built from OCS components.
///
/// This is a `repr(C)` flat struct matching the `NQuin` ABI in
/// `qualia-core-db`. It can be transmuted to `NQuin` via `bytemuck`
/// when crossing the crate boundary.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CosmicNQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

impl CosmicNQuin {
    /// Build a cosmic pose NQuin (OCS §14.2, OP_COSMIC_POSE).
    ///
    /// Encodes: "entity `subject_did` has spacetime pose `pose_iri`
    /// in realm `realm_usri` at level `level`".
    pub fn cosmic_pose(
        subject_did: &str,
        pose_iri: &str,
        realm_usri: &str,
        level: HierarchyLevel,
        sensitivity: u8,
        lamport_clock: u32,
    ) -> Self {
        Self::build(
            OP_COSMIC_POSE,
            subject_did,
            pose_iri,
            realm_usri,
            level,
            0, // nesting depth
            sensitivity,
            LANE_COMMONS,
            lamport_clock,
        )
    }

    /// Build a realm transit NQuin (OCS §14.2, OP_REALM_TRANSIT).
    pub fn realm_transit(
        subject_did: &str,
        target_realm_usri: &str,
        source_realm_usri: &str,
        level: HierarchyLevel,
        sensitivity: u8,
        lamport_clock: u32,
    ) -> Self {
        Self::build(
            OP_REALM_TRANSIT,
            subject_did,
            target_realm_usri,
            source_realm_usri,
            level,
            0,
            sensitivity,
            LANE_BILATERAL,
            lamport_clock,
        )
    }

    /// Build an observer fiber NQuin (OCS §14.2, OP_OBSERVER_FIBER).
    pub fn observer_fiber(
        observer_did: &str,
        perceived_frame_usi: &str,
        realm_usri: &str,
        level: HierarchyLevel,
        sensitivity: u8,
        lamport_clock: u32,
    ) -> Self {
        Self::build(
            OP_OBSERVER_FIBER,
            observer_did,
            perceived_frame_usi,
            realm_usri,
            level,
            0,
            sensitivity,
            LANE_BILATERAL,
            lamport_clock,
        )
    }

    /// Build a micro-pose NQuin (OCS §14.2, OP_MICRO_POSE).
    pub fn micro_pose(
        subject_did: &str,
        pose_iri: &str,
        realm_usri: &str,
        level: HierarchyLevel,
        sensitivity: u8,
        lamport_clock: u32,
    ) -> Self {
        Self::build(
            OP_MICRO_POSE,
            subject_did,
            pose_iri,
            realm_usri,
            level,
            0,
            sensitivity,
            LANE_COMMONS,
            lamport_clock,
        )
    }

    /// Build an element collapse NQuin (OCS §14.2, OP_ELEMENT_COLLAPSE).
    pub fn element_collapse(
        narrative_entity_iri: &str,
        physical_anchor_iri: &str,
        realm_usri: &str,
        level: HierarchyLevel,
        sensitivity: u8,
        lamport_clock: u32,
    ) -> Self {
        Self::build(
            OP_ELEMENT_COLLAPSE,
            narrative_entity_iri,
            physical_anchor_iri,
            realm_usri,
            level,
            0,
            sensitivity,
            LANE_COMMONS,
            lamport_clock,
        )
    }

    /// Core builder for all cosmic NQuin types (OCS §14.2).
    #[allow(clippy::too_many_arguments)]
    fn build(
        opcode: u8,
        subject_did: &str,
        object_iri: &str,
        realm_usri: &str,
        level: HierarchyLevel,
        nesting_depth: u8,
        sensitivity: u8,
        routing_lane: u8,
        lamport_clock: u32,
    ) -> Self {
        // Subject: MSB=1 (did:q42 pointer), bits [0..62] = q_hash(did)
        let subject_hash = fnv1a_64(subject_did) & 0x7FFF_FFFF_FFFF_FFFF;
        let subject = subject_hash | (1u64 << 63); // MSB=1 for did:q42 pointer

        // Predicate: [63]=0, [8..62] = q_hash(cosmic:hasSpacetimePose) << 8, [0..7] = opcode
        let pose_hash = fnv1a_64(COSMIC_POSE_IRI) & 0x003F_FFFF_FFFF_FFFF; // 54-bit hash
        let predicate = (pose_hash << 8) | (opcode as u64);

        // Object: MSB=1 (did:q42 topological pointer), bits [0..62] = q_hash(iri)
        let object_hash = fnv1a_64(object_iri) & 0x7FFF_FFFF_FFFF_FFFF;
        let object = object_hash | (1u64 << 63); // MSB=1 for did:q42 pointer

        // Context: [56..63] = sensitivity class, [0..55] = q_hash(realm_usri) (56-bit)
        let realm_hash = fnv1a_64(realm_usri) & 0x00FF_FFFF_FFFF_FFFF; // 56-bit
        let context = ((sensitivity as u64) << 56) | realm_hash;

        // Metadata: [61..62] = routing lane, [32..60] = Lamport clock (29 bits),
        //            [0..31] = realm class + scale level + nesting depth
        let lane_bits = ((routing_lane as u64) & 0b11) << 61;
        let clock_bits = ((lamport_clock as u64) & 0x1FFF_FFFF) << 32;
        // Lower 32 bits: [0..7] = realm class (derived from CB-USRI), [8..15] = level, [16..23] = depth
        let realm_class = derive_realm_class_from_usri(realm_usri);
        let lower32 = (realm_class as u64)
            | ((level.as_u8() as u64) << 8)
            | ((nesting_depth as u64) << 16);
        let metadata = lane_bits | clock_bits | lower32;

        // Parity: XOR fold
        let parity = subject ^ predicate ^ object ^ context;

        Self {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }

    /// Extract the opcode from the predicate field.
    pub fn opcode(&self) -> u8 {
        (self.predicate & 0xFF) as u8
    }

    /// Extract the subject DID hash (without MSB).
    pub fn subject_hash(&self) -> u64 {
        self.subject & 0x7FFF_FFFF_FFFF_FFFF
    }

    /// Extract the object IRI hash (without MSB).
    pub fn object_hash(&self) -> u64 {
        self.object & 0x7FFF_FFFF_FFFF_FFFF
    }

    /// Extract the sensitivity class from the context field.
    pub fn sensitivity(&self) -> u8 {
        (self.context >> 56) as u8
    }

    /// Extract the realm hash from the context field (56-bit).
    pub fn realm_hash(&self) -> u64 {
        self.context & 0x00FF_FFFF_FFFF_FFFF
    }

    /// Extract the routing lane from the metadata field.
    pub fn routing_lane(&self) -> u8 {
        ((self.metadata >> 61) & 0b11) as u8
    }

    /// Extract the Lamport logical clock from the metadata field (29 bits).
    pub fn lamport_clock(&self) -> u32 {
        ((self.metadata >> 32) & 0x1FFF_FFFF) as u32
    }

    /// Extract the realm class index from the metadata lower 32 bits.
    pub fn realm_class_index(&self) -> u8 {
        (self.metadata & 0xFF) as u8
    }

    /// Extract the hierarchy level from the metadata.
    pub fn hierarchy_level(&self) -> Option<HierarchyLevel> {
        HierarchyLevel::from_u8(((self.metadata >> 8) & 0xFF) as u8)
    }

    /// Extract the nesting depth from the metadata.
    pub fn nesting_depth(&self) -> u8 {
        ((self.metadata >> 16) & 0xFF) as u8
    }

    /// Verify the parity field (XOR fold check).
    pub fn verify_parity(&self) -> bool {
        self.parity == (self.subject ^ self.predicate ^ self.object ^ self.context)
    }

    /// Encode to 48 bytes.
    pub fn to_bytes(&self) -> [u8; 48] {
        bytemuck::cast(*self)
    }

    /// Decode from 48 bytes.
    pub fn from_bytes(bytes: &[u8; 48]) -> Self {
        *bytemuck::cast_ref(bytes)
    }

    /// Size in bytes (always 48 for NQuin ABI).
    pub const fn size_bytes() -> usize {
        48
    }
}

/// Derive a realm class index from a USRI string.
fn derive_realm_class_from_usri(usri: &str) -> u8 {
    // Try to parse the USRI and extract the realm class
    if let Ok(parsed) = super::usri::Usri::parse(usri) {
        parsed.realm_class.as_u8()
    } else {
        // Default to Physical (0) for unparseable strings
        RealmClass::Physical.as_u8()
    }
}

/// Build a CosmicNQuin from a CompactBinaryUsri (OCS §13.2 + §14.2).
///
/// This bridges the 16-byte CB-USRI (zero-heap hot path identifier)
/// to the 48-byte NQuin (semantic graph record).
pub fn from_cb_usri(
    cb: &CompactBinaryUsri,
    subject_did: &str,
    object_iri: &str,
    opcode: u8,
    lamport_clock: u32,
) -> CosmicNQuin {
    let level = HierarchyLevel::from_u8(cb.level()).unwrap_or(HierarchyLevel::L5);
    let nesting_depth = cb.nesting_depth();
    let realm_class_idx = cb.realm_class();

    // Subject: MSB=1, hash of DID
    let subject = (fnv1a_64(subject_did) & 0x7FFF_FFFF_FFFF_FFFF) | (1u64 << 63);

    // Predicate: cosmic pose hash + opcode
    let pose_hash = fnv1a_64(COSMIC_POSE_IRI) & 0x003F_FFFF_FFFF_FFFF;
    let predicate = (pose_hash << 8) | (opcode as u64);

    // Object: MSB=1, hash of IRI
    let object = (fnv1a_64(object_iri) & 0x7FFF_FFFF_FFFF_FFFF) | (1u64 << 63);

    // Context: sensitivity=PUBLIC, realm hash from CB-USRI path hash (56-bit)
    let context = (SENSITIVITY_PUBLIC as u64) << 56 | (cb.path_hash() & 0x00FF_FFFF_FFFF_FFFF);

    // Metadata: lane=Commons, clock, lower32 = realm_class | level | depth
    let metadata = ((LANE_COMMONS as u64) << 61)
        | ((lamport_clock as u64 & 0x1FFF_FFFF) << 32)
        | (realm_class_idx as u64)
        | ((level.as_u8() as u64) << 8)
        | ((nesting_depth as u64) << 16);

    let parity = subject ^ predicate ^ object ^ context;

    CosmicNQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes_in_correct_range() {
        // OCS §14.2: opcodes 0x50-0x55
        assert_eq!(OP_COSMIC_POSE, 0x50);
        assert_eq!(OP_REALM_TRANSIT, 0x51);
        assert_eq!(OP_OBSERVER_FIBER, 0x53);
        assert_eq!(OP_MICRO_POSE, 0x54);
        assert_eq!(OP_ELEMENT_COLLAPSE, 0x55);
    }

    #[test]
    fn opcodes_above_ltl_range() {
        // AGENTS.md: LTL owns 0x40-0x44, cosmic starts at 0x50
        assert!(OP_COSMIC_POSE > 0x44);
        assert!(OP_ELEMENT_COLLAPSE > 0x44);
    }

    #[test]
    fn cosmic_pose_builds_correctly() {
        let q = CosmicNQuin::cosmic_pose(
            "did:q42:person:alice",
            "urn:omni:v1:physical:observable:standard:earth:wgs84#pose",
            "urn:omni:v1:physical:observable:standard:earth:wgs84",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            12345,
        );
        assert_eq!(q.opcode(), OP_COSMIC_POSE);
        assert!(q.subject & (1u64 << 63) != 0, "subject MSB should be set");
        assert!(q.object & (1u64 << 63) != 0, "object MSB should be set");
        assert_eq!(q.sensitivity(), SENSITIVITY_PUBLIC);
        assert_eq!(q.lamport_clock(), 12345);
        assert_eq!(q.hierarchy_level(), Some(HierarchyLevel::L5));
        assert!(q.verify_parity());
    }

    #[test]
    fn realm_transit_uses_bilateral_lane() {
        let q = CosmicNQuin::realm_transit(
            "did:q42:person:alice",
            "urn:omni:v1:fiction:star-trek:prime",
            "urn:omni:v1:physical:observable:standard:earth",
            HierarchyLevel::L5,
            SENSITIVITY_RESTRICTED,
            100,
        );
        assert_eq!(q.opcode(), OP_REALM_TRANSIT);
        assert_eq!(q.routing_lane(), LANE_BILATERAL);
        assert_eq!(q.sensitivity(), SENSITIVITY_RESTRICTED);
    }

    #[test]
    fn observer_fiber_builds() {
        let q = CosmicNQuin::observer_fiber(
            "did:q42:person:bob",
            "urn:omni:v1:phenomenology:bob:perceived",
            "urn:omni:v1:phenomenology:bob",
            HierarchyLevel::L4,
            SENSITIVITY_CLASSIFIED,
            999,
        );
        assert_eq!(q.opcode(), OP_OBSERVER_FIBER);
        assert_eq!(q.sensitivity(), SENSITIVITY_CLASSIFIED);
        assert_eq!(q.routing_lane(), LANE_BILATERAL);
    }

    #[test]
    fn micro_pose_uses_correct_level() {
        let q = CosmicNQuin::micro_pose(
            "did:q42:atom:hydrogen-1",
            "urn:omni:v1:physical:observable:standard:hydrogen",
            "urn:omni:v1:physical:observable:standard:hydrogen",
            HierarchyLevel::L2,
            SENSITIVITY_PUBLIC,
            1,
        );
        assert_eq!(q.opcode(), OP_MICRO_POSE);
        assert_eq!(q.hierarchy_level(), Some(HierarchyLevel::L2));
    }

    #[test]
    fn element_collapse_builds() {
        let q = CosmicNQuin::element_collapse(
            "urn:omni:v1:narrative:homer:iliad:troy",
            "urn:omni:v1:physical:observable:standard:earth:hisarlik",
            "urn:omni:v1:narrative:homer:iliad",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            42,
        );
        assert_eq!(q.opcode(), OP_ELEMENT_COLLAPSE);
        assert_eq!(q.lamport_clock(), 42);
    }

    #[test]
    fn parity_is_verified() {
        let q = CosmicNQuin::cosmic_pose(
            "did:q42:person:test",
            "pose",
            "urn:omni:v1:physical:observable:standard:earth",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            1,
        );
        assert!(q.verify_parity());
    }

    #[test]
    fn parity_detects_corruption() {
        let mut q = CosmicNQuin::cosmic_pose(
            "did:q42:person:test",
            "pose",
            "urn:omni:v1:physical:observable:standard:earth",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            1,
        );
        q.subject ^= 1; // Corrupt one bit
        assert!(!q.verify_parity());
    }

    #[test]
    fn bytes_round_trip() {
        let q = CosmicNQuin::cosmic_pose(
            "did:q42:person:alice",
            "pose",
            "urn:omni:v1:physical:observable:standard:earth",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            100,
        );
        let bytes = q.to_bytes();
        assert_eq!(bytes.len(), 48);
        let recovered = CosmicNQuin::from_bytes(&bytes);
        assert_eq!(q, recovered);
    }

    #[test]
    fn size_is_48_bytes() {
        assert_eq!(CosmicNQuin::size_bytes(), 48);
        assert_eq!(std::mem::size_of::<CosmicNQuin>(), 48);
    }

    #[test]
    fn from_cb_usri_builds() {
        let cb = CompactBinaryUsri::from_usri(
            "urn:omni:v1:physical:observable:standard:earth:wgs84",
            RealmClass::Physical,
            HierarchyLevel::L5,
            0,
            "geo(lat=37.8,lon=-122.4)",
        );
        let q = from_cb_usri(&cb, "did:q42:person:alice", "pose-iri", OP_COSMIC_POSE, 500);
        assert_eq!(q.opcode(), OP_COSMIC_POSE);
        assert_eq!(q.hierarchy_level(), Some(HierarchyLevel::L5));
        assert_eq!(q.realm_class_index(), RealmClass::Physical.as_u8());
        assert!(q.verify_parity());
    }

    #[test]
    fn realm_class_derived_from_usri() {
        let q = CosmicNQuin::cosmic_pose(
            "did:q42:person:test",
            "pose",
            "urn:omni:v1:fiction:star-trek:prime:earth",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            1,
        );
        assert_eq!(q.realm_class_index(), RealmClass::Fiction.as_u8());
    }

    #[test]
    fn nesting_depth_extracted() {
        let q = CosmicNQuin::cosmic_pose(
            "did:q42:person:test",
            "pose",
            "urn:omni:v1:physical:observable:standard:earth",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            1,
        );
        // Default nesting depth is 0
        assert_eq!(q.nesting_depth(), 0);
    }

    #[test]
    fn lamport_clock_wraps_29_bits() {
        let q = CosmicNQuin::cosmic_pose(
            "did:q42:person:test",
            "pose",
            "urn:omni:v1:physical:observable:standard:earth",
            HierarchyLevel::L5,
            SENSITIVITY_PUBLIC,
            0x1FFF_FFFF, // max 29-bit value
        );
        assert_eq!(q.lamport_clock(), 0x1FFF_FFFF);
    }
}
