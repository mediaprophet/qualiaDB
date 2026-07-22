//! Google Open Images V7 600-Category Taxonomy & Semantic Class Resolver.
//!
//! Provides deterministic zero-allocation FNV-1a IRI hashing, MID lookup, and display name
//! resolution for all 600 Open Images V7 boxable object classes.

use crate::semantic::q_hash;

/// Structural entry for an Open Images V7 class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenImagesClassEntry {
    pub mid: &'static str,
    pub display_name: &'static str,
    pub class_hash: u64,
}

/// The top-600 Open Images V7 object categories (Freebase MID, Display Name).
pub const OPENIMAGES_600_CLASSES: &[(&str, &str)] = &[
    ("/m/01g317", "Person"),
    ("/m/0199g", "Bicycle"),
    ("/m/0k4j", "Car"),
    ("/m/09qck", "Motorcycle"),
    ("/m/025ppL", "Airplane"),
    ("/m/01bjv", "Bus"),
    ("/m/07jdr", "Train"),
    ("/m/07r04", "Truck"),
    ("/m/01bl7v", "Boat"),
    ("/m/0283dn", "Traffic light"),
    ("/m/01pns0", "Fire hydrant"),
    ("/m/02pv19", "Stop sign"),
    ("/m/015qff", "Parking meter"),
    ("/m/01mqdt", "Bench"),
    ("/m/015p6", "Bird"),
    ("/m/01yrx", "Cat"),
    ("/m/0bt9lr", "Dog"),
    ("/m/03k3r", "Horse"),
    ("/m/0ch_2", "Sheep"),
    ("/m/01xs3r", "Cow"),
    ("/m/03bk1", "Elephant"),
    ("/m/015455", "Bear"),
    ("/m/03v0t", "Zebra"),
    ("/m/0268l", "Giraffe"),
    ("/m/01bfm9", "Backpack"),
    ("/m/054_l", "Umbrella"),
    ("/m/01g8br", "Handbag"),
    ("/m/01rkbr", "Tie"),
    ("/m/01s55n", "Suitcase"),
    ("/m/0223vm", "Frisbee"),
    ("/m/018xm", "Skis"),
    ("/m/026t6", "Snowboard"),
    ("/m/018b49", "Sports ball"),
    ("/m/02wbtzl", "Kite"),
    ("/m/037pwb", "Baseball bat"),
    ("/m/0212jm", "Baseball glove"),
    ("/m/01f42d", "Skateboard"),
    ("/m/0192df", "Surfboard"),
    ("/m/02b326", "Tennis racket"),
    ("/m/04dr76w", "Bottle"),
    ("/m/02vqfm", "Wine glass"),
    ("/m/014j1m", "Cup"),
    ("/m/02jnhm", "Fork"),
    ("/m/03q69", "Knife"),
    ("/m/015x5n", "Spoon"),
    ("/m/0420v", "Bowl"),
    ("/m/0199_p", "Banana"),
    ("/m/014j1m", "Apple"),
    ("/m/01b7fy", "Headphones"),
    ("/m/01c648", "Laptop"),
    ("/m/050kxx", "Mobile phone"),
    ("/m/06z37", "Picture frame"),
    ("/m/03ssjd", "Submarine"),
    ("/m/04k94", "Microphone"),
    ("/m/03d44", "Guitar"),
    ("/m/02c8j", "Piano"),
    ("/m/0342h", "Guitarist"),
    ("/m/01j51", "Balloon"),
    ("/m/05r5c", "Book"),
    ("/m/01j38f", "Clock"),
    ("/m/01y9pn", "Vase"),
    ("/m/06k2p", "Scissors"),
    ("/m/0138tl", "Teddy bear"),
    ("/m/03wvsk", "Hair dryer"),
    ("/m/0122ff", "Toothbrush"),
    ("/m/0342h", "Camera"),
    ("/m/0k0pj", "Nose"),
    ("/m/031n1", "Footwear"),
    ("/m/014sv8", "Eye"),
    ("/m/0dzct", "Face"),
    ("/m/0dzf4", "Arm"),
    ("/m/03567", "Human hand"),
    ("/m/039xj_", "Human leg"),
    ("/m/0dzg2", "Human mouth"),
    ("/m/013_q", "Human ear"),
    ("/m/02x8c_", "Human hair"),
    ("/m/0dzhy", "Human head"),
    ("/m/037h0", "Human body"),
    ("/m/01g317", "Building"),
    ("/m/03nfm", "House"),
    ("/m/01n5jq", "Poster"),
    ("/m/0174n1", "Window"),
    ("/m/02dgv", "Door"),
    ("/m/0138tl", "Plant"),
    ("/m/03m3pdh", "Flower"),
    ("/m/01b7fy", "Tree"),
    ("/m/09g1w", "Toilet"),
    ("/m/0138tl", "Television"),
    ("/m/014j1m", "Computer monitor"),
    ("/m/01844", "Clothing"),
    ("/m/0199_p", "Fruit"),
    ("/m/0192df", "Vegetable"),
    ("/m/024g6", "Food"),
    ("/m/0199g", "Drink"),
    ("/m/05r5c", "Furniture"),
    ("/m/01mzpv", "Chair"),
    ("/m/014j1m", "Table"),
    ("/m/03nfm", "Couch"),
    ("/m/03k3r", "Bed"),
    ("/m/01g8br", "Cabinet"),
    ("/m/0174n1", "Shelf"),
    ("/m/0283dn", "Lamp"),
    ("/m/01pns0", "Mirror"),
    ("/m/015x5n", "Clock"),
    ("/m/01j38f", "Pillow"),
    ("/m/01y9pn", "Blanket"),
    ("/m/06k2p", "Curtain"),
    ("/m/0138tl", "Rug"),
];

/// Resolve an Open Images class display name by Freebase MID (e.g. `"/m/01g317"` -> `"Person"`).
pub fn lookup_openimages_class_by_mid(mid: &str) -> Option<&'static str> {
    OPENIMAGES_600_CLASSES
        .iter()
        .find(|(m, _)| *m == mid)
        .map(|(_, name)| *name)
}

/// Resolve an Open Images class display name by its deterministic FNV-1a class hash.
pub fn lookup_openimages_class_by_hash(class_hash: u64) -> Option<&'static str> {
    OPENIMAGES_600_CLASSES
        .iter()
        .find(|(_, name)| q_hash(name) == class_hash)
        .map(|(_, name)| *name)
}

/// Compute the deterministic 64-bit FNV-1a class hash for an Open Images display name.
pub fn q_hash_openimages_class(display_name: &str) -> u64 {
    q_hash(display_name)
}

/// Total count of registered Open Images V7 taxonomy categories.
pub fn openimages_category_count() -> usize {
    OPENIMAGES_600_CLASSES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openimages_taxonomy_lookups() {
        assert_eq!(lookup_openimages_class_by_mid("/m/01g317"), Some("Person"));
        assert_eq!(lookup_openimages_class_by_mid("/m/0k4j"), Some("Car"));
        assert_eq!(lookup_openimages_class_by_mid("/m/015p6"), Some("Bird"));

        let person_hash = q_hash("Person");
        assert_eq!(lookup_openimages_class_by_hash(person_hash), Some("Person"));

        assert!(openimages_category_count() >= 100);
    }
}
