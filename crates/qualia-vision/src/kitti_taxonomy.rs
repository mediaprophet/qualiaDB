//! KITTI 3D Autonomous Driving Taxonomy & Semantic Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, class index mapping (0..7),
//! and display name resolution for all 8 KITTI 3D object detection classes.

use crate::semantic::q_hash;

/// KITTI 3D object classes (Display Name, Class Index 0..7).
pub const KITTI_8_CLASSES: &[(&str, u32)] = &[
    ("Car", 0),
    ("Van", 1),
    ("Truck", 2),
    ("Pedestrian", 3),
    ("Person_sitting", 4),
    ("Cyclist", 5),
    ("Tram", 6),
    ("Misc", 7),
];

/// Resolve a KITTI class display name by numeric class index (0..7).
pub fn lookup_kitti_class_by_id(id: u32) -> Option<&'static str> {
    KITTI_8_CLASSES
        .iter()
        .find(|(_, idx)| *idx == id)
        .map(|(name, _)| *name)
}

/// Resolve a KITTI class display name by its deterministic FNV-1a class hash.
pub fn lookup_kitti_class_by_hash(class_hash: u64) -> Option<&'static str> {
    KITTI_8_CLASSES
        .iter()
        .find(|(name, _)| q_hash(name) == class_hash)
        .map(|(name, _)| *name)
}

/// Compute the deterministic 64-bit FNV-1a class hash for a KITTI display name.
pub fn q_hash_kitti_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

/// Total count of registered KITTI 3D object categories.
pub fn kitti_category_count() -> usize {
    KITTI_8_CLASSES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kitti_taxonomy_lookups() {
        assert_eq!(lookup_kitti_class_by_id(0), Some("Car"));
        assert_eq!(lookup_kitti_class_by_id(3), Some("Pedestrian"));
        assert_eq!(lookup_kitti_class_by_id(5), Some("Cyclist"));

        let car_hash = q_hash("Car");
        assert_eq!(lookup_kitti_class_by_hash(car_hash), Some("Car"));

        assert_eq!(kitti_category_count(), 8);
    }
}
