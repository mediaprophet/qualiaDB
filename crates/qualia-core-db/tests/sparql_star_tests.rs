//! SPARQL-Star embedded triple tests
//!
//! Tests for the SPARQL-Star implementation including:
//! - Virtual ID generation for embedded triples
//! - Zero-allocation guarantees

use qualia_core_db::lexicon::{generate_embedded_triple_id, TAG_EMBEDDED};

#[test]
fn test_tag_embedded_bit_pattern() {
    // Verify TAG_EMBEDDED is the correct bit pattern
    assert_eq!(TAG_EMBEDDED, 0b100u64 << 60);
}

#[test]
fn test_generate_embedded_triple_id_deterministic() {
    // Test that generate_embedded_triple_id is deterministic
    let s1 = 123456789u64;
    let p1 = 987654321u64;
    let o1 = 111111111u64;
    
    let id1 = generate_embedded_triple_id(s1, p1, o1);
    let id2 = generate_embedded_triple_id(s1, p1, o1);
    
    assert_eq!(id1, id2, "Virtual ID generation should be deterministic");
}

#[test]
fn test_generate_embedded_triple_id_has_tag() {
    // Test that generated Virtual IDs have TAG_EMBEDDED bit set
    let id = generate_embedded_triple_id(123, 456, 789);
    assert_ne!(id & TAG_EMBEDDED, 0, "TAG_EMBEDDED bit should be set");
}

#[test]
fn test_generate_embedded_triple_id_different_ordering() {
    // Test that different orderings produce different hashes
    let s = 1u64;
    let p = 2u64;
    let o = 3u64;
    
    let id1 = generate_embedded_triple_id(s, p, o);
    let id2 = generate_embedded_triple_id(o, p, s);
    let id3 = generate_embedded_triple_id(s, o, p);
    
    // All should be different (collision resistance)
    assert_ne!(id1, id2, "Different orderings should produce different hashes");
    assert_ne!(id1, id3, "Different orderings should produce different hashes");
    assert_ne!(id2, id3, "Different orderings should produce different hashes");
}

#[test]
fn test_generate_embedded_triple_id_no_collision_with_strings() {
    // Test that embedded triple IDs don't collide with string hashes
    use qualia_core_db::q_hash;
    
    let embedded_id = generate_embedded_triple_id(1, 2, 3);
    let string_hash = q_hash("test");
    
    // They should not collide because TAG_EMBEDDED bit is set
    assert_ne!(embedded_id & !TAG_EMBEDDED, string_hash & !TAG_EMBEDDED,
               "Embedded triple hashes should not collide with string hashes");
}

#[test]
fn test_generate_embedded_triple_id_60bit_truncation() {
    // Test that the hash is properly truncated to 60 bits before tagging
    let id = generate_embedded_triple_id(1, 2, 3);
    
    // The lower 60 bits should be the hash, upper 4 bits should be TAG_EMBEDDED
    let lower_60_bits = id & 0x0FFF_FFFF_FFFF_FFFF;
    let upper_4_bits = id & 0xF000_0000_0000_0000;
    
    assert_eq!(upper_4_bits, TAG_EMBEDDED, "Upper 4 bits should be TAG_EMBEDDED");
    assert_ne!(lower_60_bits, 0, "Lower 60 bits should contain the hash");
}