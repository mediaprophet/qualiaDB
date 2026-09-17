//! Compact Binary USRI (CB-USRI) for zero-heap hot paths (OCS §13.2).
//!
//! 16-byte fixed-size binary representation of a USRI for use in
//! Tier-1 hot paths, NQuin context fields, and GPU/WASM ABI.
//!
//! Layout (OCS §13.2):
//! ```text
//! Bits [0..55]   : 56-bit FNV-1a Hash of (realm_class : universe_id : branch_id : path)
//! Bits [56..63]  : 8-bit Realm Class Index
//! Bits [64..79]  : 16-bit Hierarchy Level & Nesting Depth Index (L_-2..L_12, Depth k)
//! Bits [80..127] : 48-bit Anchor / Local Coordinate Hash
//! ```
//!
//! Reference: OCS Specification v2.2.0 §13.2.

use super::usri::RealmClass;
use crate::value::Value;

/// FNV-1a 64-bit hash.
fn fnv1a_64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Hierarchy level index (OCS §3.1, §4, §5).
///
/// L_-2 through L_12, covering 61 orders of magnitude from
/// Planck scale to cosmological horizons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HierarchyLevel {
    LNeg2, // Sub-Planck / theoretical
    LNeg1, // Planck scale
    L0,    // Quantum field
    L1,    // Nuclear subatomic
    L2,    // Atomic / orbitals
    L3,    // Macromolecular
    L4,    // Cellular / tissue
    L5,    // Celestial body / geodesy
    L6,    // Terrestrial AR / local
    L7,    // Planetary system
    L8,    // Interstellar
    L9,    // Galactic
    L10,   // Galaxy cluster
    L11,   // Supercluster
    L12,   // Cosmological horizon
}

impl HierarchyLevel {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::LNeg2 => 0,
            Self::LNeg1 => 1,
            Self::L0 => 2,
            Self::L1 => 3,
            Self::L2 => 4,
            Self::L3 => 5,
            Self::L4 => 6,
            Self::L5 => 7,
            Self::L6 => 8,
            Self::L7 => 9,
            Self::L8 => 10,
            Self::L9 => 11,
            Self::L10 => 12,
            Self::L11 => 13,
            Self::L12 => 14,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::LNeg2),
            1 => Some(Self::LNeg1),
            2 => Some(Self::L0),
            3 => Some(Self::L1),
            4 => Some(Self::L2),
            5 => Some(Self::L3),
            6 => Some(Self::L4),
            7 => Some(Self::L5),
            8 => Some(Self::L6),
            9 => Some(Self::L7),
            10 => Some(Self::L8),
            11 => Some(Self::L9),
            12 => Some(Self::L10),
            13 => Some(Self::L11),
            14 => Some(Self::L12),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::LNeg2 => "L_-2",
            Self::LNeg1 => "L_-1",
            Self::L0 => "L_0",
            Self::L1 => "L_1",
            Self::L2 => "L_2",
            Self::L3 => "L_3",
            Self::L4 => "L_4",
            Self::L5 => "L_5",
            Self::L6 => "L_6",
            Self::L7 => "L_7",
            Self::L8 => "L_8",
            Self::L9 => "L_9",
            Self::L10 => "L_10",
            Self::L11 => "L_11",
            Self::L12 => "L_12",
        }
    }
}

/// Compact Binary USRI — 16-byte fixed-size (OCS §13.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompactBinaryUsri {
    /// Bytes [0..7]: 56-bit path hash + 8-bit realm class
    pub path_and_realm: u64,
    /// Bytes [8..9]: 16-bit hierarchy level + nesting depth
    pub level_and_depth: u16,
    /// Bytes [10..15]: 48-bit anchor hash (stored as u64, upper 16 bits unused)
    pub anchor_hash: u64,
}

impl CompactBinaryUsri {
    /// Build a CB-USRI from components.
    pub fn new(
        realm_class: RealmClass,
        path_hash: u64,
        level: HierarchyLevel,
        nesting_depth: u8,
        anchor_hash: u64,
    ) -> Self {
        // Pack 56-bit path hash + 8-bit realm class into u64
        let path_56 = path_hash & 0x00FF_FFFF_FFFF_FFFF;
        let realm_8 = (realm_class.as_u8() as u64) << 56;
        let path_and_realm = path_56 | realm_8;

        // Pack 8-bit level + 8-bit depth into u16
        let level_and_depth = ((level.as_u8() as u16) << 8) | (nesting_depth as u16 & 0xFF);

        // Mask anchor to 48 bits
        let anchor = anchor_hash & 0x0000_FFFF_FFFF_FFFF;

        Self {
            path_and_realm,
            level_and_depth,
            anchor_hash: anchor,
        }
    }

    /// Extract the realm class.
    pub fn realm_class(&self) -> u8 {
        (self.path_and_realm >> 56) as u8
    }

    /// Extract the 56-bit path hash.
    pub fn path_hash(&self) -> u64 {
        self.path_and_realm & 0x00FF_FFFF_FFFF_FFFF
    }

    /// Extract the hierarchy level.
    pub fn level(&self) -> u8 {
        (self.level_and_depth >> 8) as u8
    }

    /// Extract the nesting depth.
    pub fn nesting_depth(&self) -> u8 {
        (self.level_and_depth & 0xFF) as u8
    }

    /// Extract the 48-bit anchor hash.
    pub fn anchor_hash_48(&self) -> u64 {
        self.anchor_hash & 0x0000_FFFF_FFFF_FFFF
    }

    /// Encode to 16 bytes (big-endian for deterministic ordering).
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&self.path_and_realm.to_be_bytes());
        bytes[8..10].copy_from_slice(&self.level_and_depth.to_be_bytes());
        bytes[10..16].copy_from_slice(&self.anchor_hash.to_be_bytes()[2..8]);
        bytes
    }

    /// Decode from 16 bytes.
    pub fn from_bytes(bytes: &[u8; 16]) -> Self {
        let path_and_realm = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let level_and_depth = u16::from_be_bytes(bytes[8..10].try_into().unwrap());
        let mut anchor_bytes = [0u8; 8];
        anchor_bytes[2..8].copy_from_slice(&bytes[10..16]);
        let anchor_hash = u64::from_be_bytes(anchor_bytes);
        Self {
            path_and_realm,
            level_and_depth,
            anchor_hash,
        }
    }

    /// Compute from a USRI string and level/depth/anchor info.
    pub fn from_usri(
        usri_str: &str,
        realm_class: RealmClass,
        level: HierarchyLevel,
        nesting_depth: u8,
        anchor: &str,
    ) -> Self {
        let path_hash = fnv1a_64(usri_str);
        let anchor_hash = if anchor.is_empty() {
            0
        } else {
            fnv1a_64(anchor)
        };
        Self::new(realm_class, path_hash, level, nesting_depth, anchor_hash)
    }

    /// Convert to a Value::Record for inspection.
    pub fn to_value(&self) -> Value {
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("realm_class".into(), Value::U64(self.realm_class() as u64));
        rec.insert("path_hash".into(), Value::U64(self.path_hash()));
        rec.insert("level".into(), Value::U64(self.level() as u64));
        rec.insert(
            "nesting_depth".into(),
            Value::U64(self.nesting_depth() as u64),
        );
        rec.insert("anchor_hash".into(), Value::U64(self.anchor_hash_48()));
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cb_usri_pack_unpack() {
        let cb = CompactBinaryUsri::new(
            RealmClass::Physical,
            0x1234_5678_9ABC,
            HierarchyLevel::L5,
            2,
            0xDEAD_BEEF,
        );
        assert_eq!(cb.realm_class(), RealmClass::Physical.as_u8());
        assert_eq!(cb.path_hash(), 0x1234_5678_9ABC);
        assert_eq!(cb.level(), HierarchyLevel::L5.as_u8());
        assert_eq!(cb.nesting_depth(), 2);
        assert_eq!(cb.anchor_hash_48(), 0xDEAD_BEEF);
    }

    #[test]
    fn cb_usri_bytes_round_trip() {
        let cb = CompactBinaryUsri::new(
            RealmClass::Fiction,
            0x00FF_FFFF_FFFF_FFFF,
            HierarchyLevel::L12,
            3,
            0xFFFF_FFFF_FFFF,
        );
        let bytes = cb.to_bytes();
        let recovered = CompactBinaryUsri::from_bytes(&bytes);
        assert_eq!(cb, recovered);
    }

    #[test]
    fn cb_usri_from_usri_deterministic() {
        let a = CompactBinaryUsri::from_usri(
            "urn:omni:v1:physical:observable:standard:earth:wgs84",
            RealmClass::Physical,
            HierarchyLevel::L5,
            0,
            "geo(lat=37.8,lon=-122.4)",
        );
        let b = CompactBinaryUsri::from_usri(
            "urn:omni:v1:physical:observable:standard:earth:wgs84",
            RealmClass::Physical,
            HierarchyLevel::L5,
            0,
            "geo(lat=37.8,lon=-122.4)",
        );
        assert_eq!(a, b, "same USRI should produce same CB-USRI");
    }

    #[test]
    fn cb_usri_different_realms_differ() {
        let physical = CompactBinaryUsri::from_usri(
            "path",
            RealmClass::Physical,
            HierarchyLevel::L5,
            0,
            "anchor",
        );
        let fiction = CompactBinaryUsri::from_usri(
            "path",
            RealmClass::Fiction,
            HierarchyLevel::L5,
            0,
            "anchor",
        );
        assert_ne!(physical, fiction);
    }

    #[test]
    fn cb_usri_different_levels_differ() {
        let l5 = CompactBinaryUsri::from_usri(
            "path",
            RealmClass::Physical,
            HierarchyLevel::L5,
            0,
            "anchor",
        );
        let l6 = CompactBinaryUsri::from_usri(
            "path",
            RealmClass::Physical,
            HierarchyLevel::L6,
            0,
            "anchor",
        );
        assert_ne!(l5, l6);
    }

    #[test]
    fn hierarchy_level_round_trip() {
        for level in [
            HierarchyLevel::LNeg2,
            HierarchyLevel::L0,
            HierarchyLevel::L5,
            HierarchyLevel::L12,
        ] {
            assert_eq!(HierarchyLevel::from_u8(level.as_u8()), Some(level));
        }
    }

    #[test]
    fn cb_usri_16_bytes() {
        let cb = CompactBinaryUsri::new(RealmClass::Physical, 1, HierarchyLevel::L0, 0, 1);
        assert_eq!(cb.to_bytes().len(), 16);
    }

    #[test]
    fn cb_usri_to_value() {
        let cb = CompactBinaryUsri::new(RealmClass::Physical, 42, HierarchyLevel::L5, 1, 99);
        let v = cb.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("realm_class"), Some(&Value::U64(0)));
                assert_eq!(r.get("path_hash"), Some(&Value::U64(42)));
                assert_eq!(r.get("level"), Some(&Value::U64(7))); // L5 = 7
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn cb_usri_empty_anchor() {
        let cb =
            CompactBinaryUsri::from_usri("test", RealmClass::Physical, HierarchyLevel::L0, 0, "");
        assert_eq!(cb.anchor_hash_48(), 0);
    }
}
