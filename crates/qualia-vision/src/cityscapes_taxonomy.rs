//! Cityscapes Urban Scene Taxonomy & Semantic Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, class ID lookup, category group mapping,
//! and display name resolution for all 35 Cityscapes urban scene understanding classes.

use crate::semantic::q_hash;

/// Cityscapes urban scene classes (Display Name, Class ID, Category Group, Is Benchmark Evaluation Class).
pub const CITYSCAPES_CLASSES: &[(&str, u32, &str, bool)] = &[
    ("unlabeled", 0, "void", false),
    ("ego vehicle", 1, "void", false),
    ("rectification border", 2, "void", false),
    ("out of roi", 3, "void", false),
    ("static", 4, "void", false),
    ("dynamic", 5, "void", false),
    ("ground", 6, "void", false),
    ("road", 7, "flat", true),
    ("sidewalk", 8, "flat", true),
    ("parking", 9, "flat", false),
    ("rail track", 10, "flat", false),
    ("building", 11, "construction", true),
    ("wall", 12, "construction", true),
    ("fence", 13, "construction", true),
    ("guard rail", 14, "construction", false),
    ("bridge", 15, "construction", false),
    ("tunnel", 16, "construction", false),
    ("pole", 17, "object", true),
    ("polegroup", 18, "object", false),
    ("traffic light", 19, "object", true),
    ("traffic sign", 20, "object", true),
    ("vegetation", 21, "nature", true),
    ("terrain", 22, "nature", true),
    ("sky", 23, "sky", true),
    ("person", 24, "human", true),
    ("rider", 25, "human", true),
    ("car", 26, "vehicle", true),
    ("truck", 27, "vehicle", true),
    ("bus", 28, "vehicle", true),
    ("caravan", 29, "vehicle", false),
    ("trailer", 30, "vehicle", false),
    ("train", 31, "vehicle", true),
    ("motorcycle", 32, "vehicle", true),
    ("bicycle", 33, "vehicle", true),
    ("license plate", -1i32 as u32, "vehicle", false),
];

/// Resolve a Cityscapes class display name and category group by numeric class ID (0..33).
pub fn lookup_cityscapes_class_by_id(id: u32) -> Option<(&'static str, &'static str)> {
    CITYSCAPES_CLASSES
        .iter()
        .find(|(_, class_id, _, _)| *class_id == id)
        .map(|(name, _, group, _)| (*name, *group))
}

/// Resolve a Cityscapes class display name by its deterministic FNV-1a class hash.
pub fn lookup_cityscapes_class_by_hash(class_hash: u64) -> Option<&'static str> {
    CITYSCAPES_CLASSES
        .iter()
        .find(|(name, _, _, _)| q_hash(name) == class_hash)
        .map(|(name, _, _, _)| *name)
}

/// Compute the deterministic 64-bit FNV-1a class hash for a Cityscapes display name.
pub fn q_hash_cityscapes_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

/// Total count of registered Cityscapes urban scene classes.
pub fn cityscapes_category_count() -> usize {
    CITYSCAPES_CLASSES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cityscapes_taxonomy_lookups() {
        let (road, group_r) = lookup_cityscapes_class_by_id(7).unwrap();
        assert_eq!(road, "road");
        assert_eq!(group_r, "flat");

        let (building, group_b) = lookup_cityscapes_class_by_id(11).unwrap();
        assert_eq!(building, "building");
        assert_eq!(group_b, "construction");

        let (car, group_c) = lookup_cityscapes_class_by_id(26).unwrap();
        assert_eq!(car, "car");
        assert_eq!(group_c, "vehicle");

        let person_hash = q_hash("person");
        assert_eq!(lookup_cityscapes_class_by_hash(person_hash), Some("person"));

        assert_eq!(cityscapes_category_count(), 35);
    }
}
