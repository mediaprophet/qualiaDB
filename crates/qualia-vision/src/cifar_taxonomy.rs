//! CIFAR-10 & CIFAR-100 Taxonomy & Semantic Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, class ID lookup, supercategory mapping,
//! and display name resolution for CIFAR-10 (10 classes) and CIFAR-100 (100 fine classes / 20 superclasses).

use crate::semantic::q_hash;

/// CIFAR-10 object classes (Display Name, Class Index 0..9).
pub const CIFAR_10_CLASSES: &[(&str, u32)] = &[
    ("airplane", 0),
    ("automobile", 1),
    ("bird", 2),
    ("cat", 3),
    ("deer", 4),
    ("dog", 5),
    ("frog", 6),
    ("horse", 7),
    ("ship", 8),
    ("truck", 9),
];

/// CIFAR-100 fine classes (Fine Display Name, Class Index 0..99, Superclass Name).
pub const CIFAR_100_CLASSES: &[(&str, u32, &str)] = &[
    // 0: Aquatic mammals
    ("beaver", 0, "aquatic_mammals"),
    ("dolphin", 1, "aquatic_mammals"),
    ("otter", 2, "aquatic_mammals"),
    ("seal", 3, "aquatic_mammals"),
    ("whale", 4, "aquatic_mammals"),
    // 1: Fish
    ("aquarium_fish", 5, "fish"),
    ("flatfish", 6, "fish"),
    ("ray", 7, "fish"),
    ("shark", 8, "fish"),
    ("trout", 9, "fish"),
    // 2: Flowers
    ("orchid", 10, "flowers"),
    ("poppy", 11, "flowers"),
    ("rose", 12, "flowers"),
    ("sunflower", 13, "flowers"),
    ("tulip", 14, "flowers"),
    // 3: Food containers
    ("bottle", 15, "food_containers"),
    ("bowl", 16, "food_containers"),
    ("can", 17, "food_containers"),
    ("cup", 18, "food_containers"),
    ("plate", 19, "food_containers"),
    // 4: Fruit and vegetables
    ("apple", 20, "fruit_and_vegetables"),
    ("mushroom", 21, "fruit_and_vegetables"),
    ("orange", 22, "fruit_and_vegetables"),
    ("pear", 23, "fruit_and_vegetables"),
    ("sweet_pepper", 24, "fruit_and_vegetables"),
    // 5: Household electrical devices
    ("clock", 25, "household_electrical_devices"),
    ("keyboard", 26, "household_electrical_devices"),
    ("lamp", 27, "household_electrical_devices"),
    ("telephone", 28, "household_electrical_devices"),
    ("television", 29, "household_electrical_devices"),
    // 6: Household furniture
    ("bed", 30, "household_furniture"),
    ("chair", 31, "household_furniture"),
    ("couch", 32, "household_furniture"),
    ("table", 33, "household_furniture"),
    ("wardrobe", 34, "household_furniture"),
    // 7: Insects
    ("bee", 35, "insects"),
    ("beetle", 36, "insects"),
    ("butterfly", 37, "insects"),
    ("caterpillar", 38, "insects"),
    ("cockroach", 39, "insects"),
    // 8: Large carnivores
    ("bear", 40, "large_carnivores"),
    ("leopard", 41, "large_carnivores"),
    ("lion", 42, "large_carnivores"),
    ("tiger", 43, "large_carnivores"),
    ("wolf", 44, "large_carnivores"),
    // 9: Large man-made outdoor things
    ("bridge", 45, "large_man_made_outdoor_things"),
    ("castle", 46, "large_man_made_outdoor_things"),
    ("house", 47, "large_man_made_outdoor_things"),
    ("road", 48, "large_man_made_outdoor_things"),
    ("skyscraper", 49, "large_man_made_outdoor_things"),
    // 10: Large natural outdoor scenes
    ("cloud", 50, "large_natural_outdoor_scenes"),
    ("forest", 51, "large_natural_outdoor_scenes"),
    ("mountain", 52, "large_natural_outdoor_scenes"),
    ("plain", 53, "large_natural_outdoor_scenes"),
    ("sea", 54, "large_natural_outdoor_scenes"),
    // 11: Large omnivores and herbivores
    ("camel", 55, "large_omnivores_and_herbivores"),
    ("cattle", 56, "large_omnivores_and_herbivores"),
    ("chimpanzee", 57, "large_omnivores_and_herbivores"),
    ("elephant", 58, "large_omnivores_and_herbivores"),
    ("kangaroo", 59, "large_omnivores_and_herbivores"),
    // 12: Medium-sized mammals
    ("fox", 60, "medium_sized_mammals"),
    ("porcupine", 61, "medium_sized_mammals"),
    ("possum", 62, "medium_sized_mammals"),
    ("raccoon", 63, "medium_sized_mammals"),
    ("skunk", 64, "medium_sized_mammals"),
    // 13: Non-insect invertebrates
    ("crab", 65, "non_insect_invertebrates"),
    ("lobster", 66, "non_insect_invertebrates"),
    ("snail", 67, "non_insect_invertebrates"),
    ("spider", 68, "non_insect_invertebrates"),
    ("worm", 69, "non_insect_invertebrates"),
    // 14: People
    ("baby", 70, "people"),
    ("boy", 71, "people"),
    ("girl", 72, "people"),
    ("man", 73, "people"),
    ("woman", 74, "people"),
    // 15: Reptiles
    ("crocodile", 75, "reptiles"),
    ("dinosaur", 76, "reptiles"),
    ("lizard", 77, "reptiles"),
    ("snake", 78, "reptiles"),
    ("turtle", 79, "reptiles"),
    // 16: Small mammals
    ("hamster", 80, "small_mammals"),
    ("mouse", 81, "small_mammals"),
    ("rabbit", 82, "small_mammals"),
    ("shrew", 83, "small_mammals"),
    ("squirrel", 84, "small_mammals"),
    // 17: Trees
    ("maple_tree", 85, "trees"),
    ("oak_tree", 86, "trees"),
    ("palm_tree", 87, "trees"),
    ("pine_tree", 88, "trees"),
    ("willow_tree", 89, "trees"),
    // 18: Vehicles 1
    ("bicycle", 90, "vehicles_1"),
    ("bus", 91, "vehicles_1"),
    ("motorcycle", 92, "vehicles_1"),
    ("pickup_truck", 93, "vehicles_1"),
    ("train", 94, "vehicles_1"),
    // 19: Vehicles 2
    ("lawn_mower", 95, "vehicles_2"),
    ("rocket", 96, "vehicles_2"),
    ("streetcar", 97, "vehicles_2"),
    ("tank", 98, "vehicles_2"),
    ("tractor", 99, "vehicles_2"),
];

/// Resolve a CIFAR-10 class display name by numeric class index (0..9).
pub fn lookup_cifar10_class_by_id(id: u32) -> Option<&'static str> {
    CIFAR_10_CLASSES
        .iter()
        .find(|(_, idx)| *idx == id)
        .map(|(name, _)| *name)
}

/// Resolve a CIFAR-100 fine class display name and superclass by numeric fine class index (0..99).
pub fn lookup_cifar100_class_by_id(id: u32) -> Option<(&'static str, &'static str)> {
    CIFAR_100_CLASSES
        .iter()
        .find(|(_, idx, _)| *idx == id)
        .map(|(name, _, superclass)| (*name, *superclass))
}

/// Resolve a CIFAR-10 or CIFAR-100 display name by its deterministic FNV-1a class hash.
pub fn lookup_cifar_class_by_hash(class_hash: u64) -> Option<&'static str> {
    if let Some((name, _)) = CIFAR_10_CLASSES
        .iter()
        .find(|(name, _)| q_hash(name) == class_hash)
    {
        return Some(*name);
    }
    if let Some((name, _, _)) = CIFAR_100_CLASSES
        .iter()
        .find(|(name, _, _)| q_hash(name) == class_hash)
    {
        return Some(*name);
    }
    None
}

/// Compute the deterministic 64-bit FNV-1a class hash for a CIFAR display name.
pub fn q_hash_cifar_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cifar_taxonomy_lookups() {
        assert_eq!(lookup_cifar10_class_by_id(0), Some("airplane"));
        assert_eq!(lookup_cifar10_class_by_id(3), Some("cat"));
        assert_eq!(lookup_cifar10_class_by_id(5), Some("dog"));

        let (fine, superclass) = lookup_cifar100_class_by_id(73).unwrap();
        assert_eq!(fine, "man");
        assert_eq!(superclass, "people");

        let (dolphin, super_d) = lookup_cifar100_class_by_id(1).unwrap();
        assert_eq!(dolphin, "dolphin");
        assert_eq!(super_d, "aquatic_mammals");

        let frog_hash = q_hash("frog");
        assert_eq!(lookup_cifar_class_by_hash(frog_hash), Some("frog"));
    }
}
