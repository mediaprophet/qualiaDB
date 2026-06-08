//! Lexicon Dictionary Manager
//! Maps multi-modal semantic concepts (Text, Audio, Visual) into the deterministic 60-bit integers
//! required by the QualiaQuin data structure.

/// Represents the pluralistic forms that a semantic concept can take.
/// We explicitly reject the assumption that knowledge is exclusively bound to Unicode strings.
pub enum SemanticModality<'a> {
    Text(&'a str),
    AudioHash(&'a [u8]),        // For mother tongues / oral traditions
    CeremonialVisual(&'a [u8]), // For heraldry / visual concepts
    PhoneticSchema(&'a [u8]),   // For non-western phonetics
}

/// Generates a deterministic, collision-resistant 60-bit token from a raw byte stream.
/// Uses a custom FNV-1a inspired hash restricted to 60 bits.
#[inline(always)]
pub fn generate_60bit_token(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    // Truncate to 60 bits (the top 4 bits are reserved for datatype tags in the O vector)
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

/// In-memory Lexicon manager to handle reverse lookups in the future.
/// For now, ingestion purely maps forward (Bytes -> u64) via the hash.
pub struct LexiconManager {
    // A production database would memory-map a reverse lookup file here (u64 -> Modality)
}

impl LexiconManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Converts a multi-modal semantic concept into its 60-bit hardware representation.
    pub fn tokenize_modal(&self, modality: &SemanticModality) -> u64 {
        match modality {
            SemanticModality::Text(text) => generate_60bit_token(text.as_bytes()),
            SemanticModality::AudioHash(bytes) => generate_60bit_token(bytes),
            SemanticModality::CeremonialVisual(bytes) => generate_60bit_token(bytes),
            SemanticModality::PhoneticSchema(bytes) => generate_60bit_token(bytes),
        }
    }

    /// Legacy support for text strings
    pub fn tokenize(&self, literal: &str) -> u64 {
        self.tokenize_modal(&SemanticModality::Text(literal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_60bit_truncation() {
        let uri = "https://mediaprophet.github.io/qualiaDB/user/123";
        let token = generate_60bit_token(uri.as_bytes());

        // Ensure the top 4 bits are strictly 0 (Lexicon ID datatype tag)
        assert_eq!(token >> 60, 0, "Token spilled over 60 bits");
    }

    #[test]
    fn test_determinism() {
        let uri = "qualia:guardian";
        let t1 = generate_60bit_token(uri.as_bytes());
        let t2 = generate_60bit_token(uri.as_bytes());
        assert_eq!(t1, t2, "Tokens are not deterministic");
    }

    #[test]
    fn test_linguistic_plurality() {
        let lexicon = LexiconManager::new();

        // Simulating a written concept
        let written = SemanticModality::Text("peace_infrastructure");
        let t1 = lexicon.tokenize_modal(&written);

        // Simulating the exact same concept represented as a cryptographic audio hash of a spoken prayer
        let audio_hash = vec![0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        let oral = SemanticModality::AudioHash(&audio_hash);
        let t2 = lexicon.tokenize_modal(&oral);

        // Simulating a ceremonial SVG file representation
        let svg_bytes = b"<svg>Heraldry</svg>";
        let visual = SemanticModality::CeremonialVisual(svg_bytes);
        let t3 = lexicon.tokenize_modal(&visual);

        // Prove that the database treats all modalities as valid 60-bit structural Quins
        assert!(t1 > 0 && t1 <= 0x0FFF_FFFF_FFFF_FFFF);
        assert!(t2 > 0 && t2 <= 0x0FFF_FFFF_FFFF_FFFF);
        assert!(t3 > 0 && t3 <= 0x0FFF_FFFF_FFFF_FFFF);

        // Prove that changing the audio hash creates a unique identifier
        let altered_audio = vec![0x1a, 0x2b, 0x3c, 0x4d, 0x5f];
        let altered_oral = SemanticModality::AudioHash(&altered_audio);
        let t4 = lexicon.tokenize_modal(&altered_oral);
        assert_ne!(t2, t4, "Collision in multi-modal hashing");
    }
}
