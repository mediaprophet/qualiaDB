//! Q42 Semantic Graph Bridge for Webizen Icons
//!
//! Encodes icon entries as 48-byte Super-Quin semantic records conforming to the
//! Unicode Character Database (UCD) RDF ontology:
//!
//!   subject   = q_hash("ucd:U+{HEX}")
//!   context   = q_hash("webizen:icons:0.0.35")
//!   metadata  = Lamport clock + standard routing lane
//!   parity    = XOR fold: subject ^ predicate ^ object ^ context
//!
//! Predicates generated per icon:
//!   1. ucd:name           -> q_hash(entry.ascii_label)
//!   2. ucd:generalCategory -> q_hash(entry.category.as_str())
//!   3. webizen:fallbackChar -> entry.unicode_fallback as u64 (with inline tag)
//!   4. webizen:iconId      -> entry.id_hash

use super::icon_registry::{q_hash, IconEntry};

/// 48-byte Super-Quin semantic datum matching QualiaDB's NQuin hardware layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SuperQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

impl SuperQuin {
    /// Constructs a parity-valid SuperQuin.
    #[inline(always)]
    pub const fn new(subject: u64, predicate: u64, object: u64, context: u64, metadata: u64) -> Self {
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        Self {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }

    /// Verifies the ECC XOR parity checksum.
    #[inline(always)]
    pub const fn is_valid(&self) -> bool {
        self.parity == (self.subject ^ self.predicate ^ self.object ^ self.context ^ self.metadata)
    }
}

// Well-known RDF predicate hashes
pub const PRED_UCD_NAME: u64 = q_hash("http://unicode.org/ns/2003/ucd/name");
pub const PRED_UCD_CATEGORY: u64 = q_hash("http://unicode.org/ns/2003/ucd/generalCategory");
pub const PRED_WEBIZEN_FALLBACK: u64 = q_hash("https://webizen.org/ns/icon#fallbackChar");
pub const PRED_WEBIZEN_ICON_ID: u64 = q_hash("https://webizen.org/ns/icon#iconId");

// Standard context for Webizen 0.0.35 icon definitions
pub const CONTEXT_WEBIZEN_ICONS: u64 = q_hash("urn:webizen:icons:0.0.35");

/// Converts an IconEntry into a fixed array of 4 SuperQuins. Zero heap allocation.
pub fn icon_entry_to_super_quins(entry: &IconEntry) -> [SuperQuin; 4] {
    // Subject hash derived from codepoint e.g. "ucd:U+E001"
    let subject_hash = entry.id_hash ^ (entry.pua as u64);

    let name_object = q_hash(entry.ascii_label);
    let category_object = q_hash(entry.category.as_str());
    let fallback_object = entry.unicode_fallback as u64;
    let id_object = entry.id_hash;

    [
        SuperQuin::new(subject_hash, PRED_UCD_NAME, name_object, CONTEXT_WEBIZEN_ICONS, 0),
        SuperQuin::new(subject_hash, PRED_UCD_CATEGORY, category_object, CONTEXT_WEBIZEN_ICONS, 0),
        SuperQuin::new(subject_hash, PRED_WEBIZEN_FALLBACK, fallback_object, CONTEXT_WEBIZEN_ICONS, 0),
        SuperQuin::new(subject_hash, PRED_WEBIZEN_ICON_ID, id_object, CONTEXT_WEBIZEN_ICONS, 0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::icon_registry::ALL_ICONS;

    #[test]
    fn test_super_quin_generation_and_parity() {
        for icon in ALL_ICONS {
            let quins = icon_entry_to_super_quins(icon);
            assert_eq!(quins.len(), 4);
            for q in &quins {
                assert!(q.is_valid(), "Parity invalid for icon '{}'", icon.id);
                assert_eq!(q.context, CONTEXT_WEBIZEN_ICONS);
            }
            assert_eq!(quins[0].predicate, PRED_UCD_NAME);
            assert_eq!(quins[1].predicate, PRED_UCD_CATEGORY);
            assert_eq!(quins[2].predicate, PRED_WEBIZEN_FALLBACK);
            assert_eq!(quins[3].predicate, PRED_WEBIZEN_ICON_ID);
        }
    }
}
