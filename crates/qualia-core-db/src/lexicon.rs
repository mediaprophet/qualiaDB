//! Lexicon Dictionary Manager
//! Maps human-readable UTF-8 URIs and strings into the deterministic 60-bit integers
//! required by the QualiaQuin data structure.

/// Generates a deterministic, collision-resistant 60-bit token from a string.
/// Uses a custom FNV-1a inspired hash restricted to 60 bits.
#[inline(always)]
pub fn generate_60bit_token(uri: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for byte in uri.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    // Truncate to 60 bits (the top 4 bits are reserved for datatype tags in the O vector)
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

/// In-memory Lexicon manager to handle reverse lookups in the future.
/// For now, ingestion purely maps forward (String -> u64) via the hash.
pub struct LexiconManager {
    // A production database would memory-map a reverse lookup file here (u64 -> String)
}

impl LexiconManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Converts a URI or literal into its 60-bit hardware representation.
    pub fn tokenize(&self, literal: &str) -> u64 {
        generate_60bit_token(literal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_60bit_truncation() {
        let uri = "https://mediaprophet.github.io/qualiaDB/user/123";
        let token = generate_60bit_token(uri);
        
        // Ensure the top 4 bits are strictly 0 (Lexicon ID datatype tag)
        assert_eq!(token >> 60, 0, "Token spilled over 60 bits");
    }

    #[test]
    fn test_determinism() {
        let uri = "qualia:guardian";
        let t1 = generate_60bit_token(uri);
        let t2 = generate_60bit_token(uri);
        assert_eq!(t1, t2, "Tokens are not deterministic");
    }
}
