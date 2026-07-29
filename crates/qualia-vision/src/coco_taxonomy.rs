//! MS COCO (Common Objects in Context) 80-Category Taxonomy & Semantic Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, COCO category ID lookup, and display name
//! resolution for all 80 MS COCO object detection classes.

use crate::semantic::q_hash;

/// MS COCO 80 object categories (Display Name, COCO Category ID).
pub const COCO_80_CLASSES: &[(&str, u32)] = &[
    ("person", 1),
    ("bicycle", 2),
    ("car", 3),
    ("motorcycle", 4),
    ("airplane", 5),
    ("bus", 6),
    ("train", 7),
    ("truck", 8),
    ("boat", 9),
    ("traffic light", 10),
    ("fire hydrant", 11),
    ("stop sign", 13),
    ("parking meter", 14),
    ("bench", 15),
    ("bird", 16),
    ("cat", 17),
    ("dog", 18),
    ("horse", 19),
    ("sheep", 20),
    ("cow", 21),
    ("elephant", 22),
    ("bear", 23),
    ("zebra", 24),
    ("giraffe", 25),
    ("backpack", 27),
    ("umbrella", 28),
    ("handbag", 31),
    ("tie", 32),
    ("suitcase", 33),
    ("frisbee", 34),
    ("skis", 35),
    ("snowboard", 36),
    ("sports ball", 37),
    ("kite", 38),
    ("baseball bat", 39),
    ("baseball glove", 40),
    ("skateboard", 41),
    ("surfboard", 42),
    ("tennis racket", 43),
    ("bottle", 44),
    ("wine glass", 46),
    ("cup", 47),
    ("fork", 48),
    ("knife", 49),
    ("spoon", 50),
    ("bowl", 51),
    ("banana", 52),
    ("apple", 53),
    ("sandwich", 54),
    ("orange", 55),
    ("broccoli", 56),
    ("carrot", 57),
    ("hot dog", 58),
    ("pizza", 59),
    ("donut", 60),
    ("cake", 61),
    ("chair", 62),
    ("couch", 63),
    ("potted plant", 64),
    ("bed", 65),
    ("dining table", 67),
    ("toilet", 70),
    ("tv", 72),
    ("laptop", 73),
    ("mouse", 74),
    ("remote", 75),
    ("keyboard", 76),
    ("cell phone", 77),
    ("microwave", 78),
    ("oven", 79),
    ("toaster", 80),
    ("sink", 81),
    ("refrigerator", 82),
    ("book", 84),
    ("clock", 85),
    ("vase", 86),
    ("scissors", 87),
    ("teddy bear", 88),
    ("hair drier", 89),
    ("toothbrush", 90),
];

/// Resolve an MS COCO category display name by COCO numeric ID (1..90).
pub fn lookup_coco_class_by_id(id: u32) -> Option<&'static str> {
    COCO_80_CLASSES
        .iter()
        .find(|(_, cat_id)| *cat_id == id)
        .map(|(name, _)| *name)
}

/// Resolve an MS COCO category display name by its deterministic FNV-1a class hash.
pub fn lookup_coco_class_by_hash(class_hash: u64) -> Option<&'static str> {
    COCO_80_CLASSES
        .iter()
        .find(|(name, _)| q_hash(name) == class_hash)
        .map(|(name, _)| *name)
}

/// Compute the deterministic 64-bit FNV-1a class hash for a COCO display name.
pub fn q_hash_coco_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

/// Total count of registered MS COCO object categories.
pub fn coco_category_count() -> usize {
    COCO_80_CLASSES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coco_taxonomy_lookups() {
        assert_eq!(lookup_coco_class_by_id(1), Some("person"));
        assert_eq!(lookup_coco_class_by_id(3), Some("car"));
        assert_eq!(lookup_coco_class_by_id(16), Some("bird"));
        assert_eq!(lookup_coco_class_by_id(73), Some("laptop"));

        let person_hash = q_hash("person");
        assert_eq!(lookup_coco_class_by_hash(person_hash), Some("person"));

        assert_eq!(coco_category_count(), 80);
    }
}
