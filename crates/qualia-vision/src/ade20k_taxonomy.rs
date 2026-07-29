//! MIT ADE20K 150-Category Scene Parsing Taxonomy & Semantic Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, class ID lookup (1..150),
//! and display name resolution for all 150 ADE20K scene parsing classes.

use crate::semantic::q_hash;

/// MIT ADE20K 150 scene parsing classes (Display Name, Class ID 1..150).
pub const ADE20K_150_CLASSES: &[(&str, u32)] = &[
    ("wall", 1),
    ("building", 2),
    ("sky", 3),
    ("floor", 4),
    ("tree", 5),
    ("ceiling", 6),
    ("road", 7),
    ("bed", 8),
    ("windowpane", 9),
    ("grass", 10),
    ("cabinet", 11),
    ("sidewalk", 12),
    ("person", 13),
    ("earth", 14),
    ("door", 15),
    ("table", 16),
    ("mountain", 17),
    ("plant", 18),
    ("curtain", 19),
    ("chair", 20),
    ("car", 21),
    ("water", 22),
    ("painting", 23),
    ("sofa", 24),
    ("shelf", 25),
    ("house", 26),
    ("sea", 27),
    ("mirror", 28),
    ("rug", 29),
    ("field", 30),
    ("armchair", 31),
    ("seat", 32),
    ("fence", 33),
    ("desk", 34),
    ("rock", 35),
    ("wardrobe", 36),
    ("lamp", 37),
    ("bathtub", 38),
    ("railing", 39),
    ("cushion", 40),
    ("base", 41),
    ("box", 42),
    ("column", 43),
    ("signboard", 44),
    ("chest of drawers", 45),
    ("counter", 46),
    ("sand", 47),
    ("sink", 48),
    ("skyscraper", 49),
    ("fireplace", 50),
    ("refrigerator", 51),
    ("grandstand", 52),
    ("path", 53),
    ("stairs", 54),
    ("runway", 55),
    ("case", 56),
    ("pool table", 57),
    ("pillow", 58),
    ("screen door", 59),
    ("stairway", 60),
    ("river", 61),
    ("bridge", 62),
    ("bookcase", 63),
    ("blind", 64),
    ("coffee table", 65),
    ("toilet", 66),
    ("flower", 67),
    ("book", 68),
    ("hill", 69),
    ("bench", 70),
    ("countertop", 71),
    ("stove", 72),
    ("palm", 73),
    ("kitchen island", 74),
    ("computer", 75),
    ("swivel chair", 76),
    ("boat", 77),
    ("bar", 78),
    ("arcade machine", 79),
    ("hovel", 80),
    ("bus", 81),
    ("towel", 82),
    ("light", 83),
    ("truck", 84),
    ("tower", 85),
    ("chandelier", 86),
    ("awning", 87),
    ("street lamp", 88),
    ("booth", 89),
    ("television receiver", 90),
    ("airplane", 91),
    ("dirt track", 92),
    ("apparel", 93),
    ("pole", 94),
    ("land", 95),
    ("bannister", 96),
    ("escalator", 97),
    ("ottoman", 98),
    ("bottle", 99),
    ("buffet", 100),
    ("poster", 101),
    ("stage", 102),
    ("van", 103),
    ("ship", 104),
    ("fountain", 105),
    ("conveyer belt", 106),
    ("canopy", 107),
    ("washer", 108),
    ("plaything", 109),
    ("swimming pool", 110),
    ("stool", 111),
    ("barrel", 112),
    ("basket", 113),
    ("waterfall", 114),
    ("tent", 115),
    ("bag", 116),
    ("minibike", 117),
    ("cradle", 118),
    ("oven", 119),
    ("ball", 120),
    ("food", 121),
    ("step", 122),
    ("tank", 123),
    ("trade name", 124),
    ("microwave", 125),
    ("pot", 126),
    ("animal", 127),
    ("bicycle", 128),
    ("lake", 129),
    ("dishwasher", 130),
    ("screen", 131),
    ("blanket", 132),
    ("sculpture", 133),
    ("hood", 134),
    ("sconce", 135),
    ("vase", 136),
    ("traffic light", 137),
    ("tray", 138),
    ("trash can", 139),
    ("fan", 140),
    ("pier", 141),
    ("crt screen", 142),
    ("plate", 143),
    ("monitor", 144),
    ("bulletin board", 145),
    ("shower", 146),
    ("radiator", 147),
    ("glass", 148),
    ("clock", 149),
    ("flag", 150),
];

/// Resolve an ADE20K scene parsing display name by numeric class ID (1..150).
pub fn lookup_ade20k_class_by_id(id: u32) -> Option<&'static str> {
    ADE20K_150_CLASSES
        .iter()
        .find(|(_, idx)| *idx == id)
        .map(|(name, _)| *name)
}

/// Resolve an ADE20K scene parsing display name by its deterministic FNV-1a class hash.
pub fn lookup_ade20k_class_by_hash(class_hash: u64) -> Option<&'static str> {
    ADE20K_150_CLASSES
        .iter()
        .find(|(name, _)| q_hash(name) == class_hash)
        .map(|(name, _)| *name)
}

/// Compute the deterministic 64-bit FNV-1a class hash for an ADE20K display name.
pub fn q_hash_ade20k_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

/// Total count of registered ADE20K scene parsing categories.
pub fn ade20k_category_count() -> usize {
    ADE20K_150_CLASSES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ade20k_taxonomy_lookups() {
        assert_eq!(lookup_ade20k_class_by_id(1), Some("wall"));
        assert_eq!(lookup_ade20k_class_by_id(2), Some("building"));
        assert_eq!(lookup_ade20k_class_by_id(3), Some("sky"));
        assert_eq!(lookup_ade20k_class_by_id(150), Some("flag"));

        let tree_hash = q_hash("tree");
        assert_eq!(lookup_ade20k_class_by_hash(tree_hash), Some("tree"));

        assert_eq!(ade20k_category_count(), 150);
    }
}
