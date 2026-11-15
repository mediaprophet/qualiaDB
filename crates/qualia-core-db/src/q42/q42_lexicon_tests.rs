use super::*;

#[test]
fn namespaced_v4_volume_preserves_original_hash_associations_and_full_terms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lexicon.q42");
    let mut original = HashMap::new();
    original.insert(crate::q_hash("Alice"), "https://example.org/Alice".into());
    original.insert(crate::q_hash("Bob"), "https://example.org/Bob".into());
    original.insert(crate::q_hash("knows"), "http://schema.org/knows".into());
    original.insert(crate::q_hash("prefix:ex"), "ex:https://example.org/".into());
    let builder = crate::q42_volume::UnifiedVolumeBuilder::with_lex_map(&original).unwrap();
    builder.finish(&path).unwrap();
    let volume = Q42Volume::open(&path).unwrap();
    let native = volume.lex_view().unwrap();
    assert_eq!(native.lookup_hash(crate::q_hash("Alice")), None, "fixture exercises namespaced v4, not the borrowed verbatim path");
    let adapted = Q42Lexicon::from_volume(&volume).unwrap();
    assert_eq!(adapted.reverse, original);
    for (key, term) in &original { assert_eq!(adapted.resolve_term(term), Some(*key)); }
    let context = Q42Context::from_volume(&volume).unwrap();
    assert_eq!(context.resolve_semantic_term("ex").as_deref(), Some("https://example.org/"));
}
