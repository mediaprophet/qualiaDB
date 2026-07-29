//! PASCAL VOC (Visual Object Classes) 20-Category Taxonomy & Semantic Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, class index mapping (0..19),
//! and display name resolution for all 20 PASCAL VOC object classes.

use crate::semantic::q_hash;

/// PASCAL VOC 20 object classes (Display Name, Class Index 0..19).
pub const PASCAL_VOC_20_CLASSES: &[(&str, u32)] = &[
    ("aeroplane", 0),
    ("bicycle", 1),
    ("bird", 2),
    ("boat", 3),
    ("bottle", 4),
    ("bus", 5),
    ("car", 6),
    ("cat", 7),
    ("chair", 8),
    ("cow", 9),
    ("diningtable", 10),
    ("dog", 11),
    ("horse", 12),
    ("motorbike", 13),
    ("person", 14),
    ("pottedplant", 15),
    ("sheep", 16),
    ("sofa", 17),
    ("train", 18),
    ("tvmonitor", 19),
];

/// Resolve a PASCAL VOC class display name by numeric class index (0..19).
pub fn lookup_pascal_voc_class_by_id(id: u32) -> Option<&'static str> {
    PASCAL_VOC_20_CLASSES
        .iter()
        .find(|(_, idx)| *idx == id)
        .map(|(name, _)| *name)
}

/// Resolve a PASCAL VOC class display name by its deterministic FNV-1a class hash.
pub fn lookup_pascal_voc_class_by_hash(class_hash: u64) -> Option<&'static str> {
    PASCAL_VOC_20_CLASSES
        .iter()
        .find(|(name, _)| q_hash(name) == class_hash)
        .map(|(name, _)| *name)
}

/// Compute the deterministic 64-bit FNV-1a class hash for a PASCAL VOC display name.
pub fn q_hash_pascal_voc_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

/// Total count of registered PASCAL VOC object categories.
pub fn pascal_voc_category_count() -> usize {
    PASCAL_VOC_20_CLASSES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_voc_taxonomy_lookups() {
        assert_eq!(lookup_pascal_voc_class_by_id(0), Some("aeroplane"));
        assert_eq!(lookup_pascal_voc_class_by_id(6), Some("car"));
        assert_eq!(lookup_pascal_voc_class_by_id(14), Some("person"));

        let dog_hash = q_hash("dog");
        assert_eq!(lookup_pascal_voc_class_by_hash(dog_hash), Some("dog"));

        assert_eq!(pascal_voc_category_count(), 20);
    }
}
