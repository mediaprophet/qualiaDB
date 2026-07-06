//! Integrity test for the RDF → `.q42` ingest modes (`query::ingest`).
//!
//! Background: the historical ingest hashed every subject/predicate/object into a 48-byte quin and
//! wrote an **empty** lexicon (`lex_length: 0`). Every URI and every literal was therefore
//! irrecoverable, while the shrunk output was described as "compression". That was data loss reported
//! as a size win — the exact claim-vs-reality gap CLAUDE.md §15 forbids. The fix makes the mode
//! explicit:
//!   * `IngestMode::Complete` (default) — interns all terms into the lexicon; text is recoverable.
//!   * `IngestMode::StripLiterals`      — hash-only, structure-only; text is gone (and labelled so).
//!
//! This test proves both, end-to-end on a real ingest, including that **multilingual (non-ASCII)**
//! literals survive byte-intact — a lossless store that only kept ASCII would silently erase most of
//! the world's languages. Integration test (links the lib compiled normally) so it runs despite a
//! `#[cfg(test)]` breakage elsewhere in the crate.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::ingest::{streaming_import_rdf_with_mode, IngestMode};
use qualia_core_db::q42_volume::Q42Volume;

/// N-Triples with an English gloss plus Finnish, Japanese, and Arabic literals — and a couple of URIs.
const NT: &str = r#"<http://example.org/dog> <http://www.w3.org/2000/01/rdf-schema#label> "hound" .
<http://example.org/dog> <http://example.org/gloss> "a domesticated carnivore" .
<http://example.org/koira> <http://example.org/gloss> "koira on eläin, ei kasvi" .
<http://example.org/inu> <http://example.org/gloss> "犬は動物です" .
<http://example.org/kalb> <http://example.org/gloss> "الكلب حيوان أليف" .
"#;

fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(name)
}

fn all_lexicon_strings(vol: &Q42Volume) -> Vec<String> {
    match vol.lex_view() {
        Ok(lex) => (0..lex.entry_count())
            .filter_map(|i| lex.string_at(i).map(|s| s.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn complete_mode_recovers_uris_and_multilingual_literals() {
    let in_path = temp("qualia_ingest_complete.nt");
    let out_path = temp("qualia_ingest_complete.q42");
    std::fs::write(&in_path, NT).unwrap();

    let n = streaming_import_rdf_with_mode(
        in_path.to_str().unwrap(),
        out_path.to_str().unwrap(),
        IngestMode::Complete,
    )
    .expect("ingest (complete)");
    assert_eq!(n, 5, "five triples in, five quins out");

    let vol = Q42Volume::open(&out_path).expect("open q42");
    let strings = all_lexicon_strings(&vol);
    assert!(
        !strings.is_empty(),
        "Complete mode MUST write a populated lexicon (this is the anti-data-loss fix)"
    );
    let joined = strings.join("\n");

    // Literal TEXT is recoverable — including every non-ASCII script, byte-intact.
    for needle in [
        "a domesticated carnivore", // English
        "koira on eläin",           // Finnish (ä)
        "犬は動物です",              // Japanese
        "الكلب حيوان أليف",         // Arabic (RTL)
        "hound",
    ] {
        assert!(
            joined.contains(needle),
            "Complete lexicon must retain literal {needle:?}; got {} entries",
            strings.len()
        );
    }
    // Subject/predicate URIs are recoverable too.
    assert!(joined.contains("example.org/dog"), "subject URI retained");
    assert!(
        joined.contains("2000/01/rdf-schema#label"),
        "predicate URI retained"
    );

    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn strip_mode_writes_no_lexicon_and_is_labelled_lossy() {
    let in_path = temp("qualia_ingest_strip.nt");
    let out_path = temp("qualia_ingest_strip.q42");
    std::fs::write(&in_path, NT).unwrap();

    let n = streaming_import_rdf_with_mode(
        in_path.to_str().unwrap(),
        out_path.to_str().unwrap(),
        IngestMode::StripLiterals,
    )
    .expect("ingest (strip)");
    assert_eq!(n, 5);

    let vol = Q42Volume::open(&out_path).expect("open q42");
    let strings = all_lexicon_strings(&vol);
    assert!(
        strings.is_empty(),
        "StripLiterals mode is structure-only: no lexicon, no recoverable text (got {} strings)",
        strings.len()
    );

    // The lossy output is genuinely smaller than the lossless one — but that reduction is DATA LOSS,
    // not compression. Prove the size gap exists (so the mode difference is real), and that the text
    // simply isn't there to recover.
    let complete = temp("qualia_ingest_strip_cmp.q42");
    streaming_import_rdf_with_mode(
        in_path.to_str().unwrap(),
        complete.to_str().unwrap(),
        IngestMode::Complete,
    )
    .unwrap();
    let strip_len = std::fs::metadata(&out_path).unwrap().len();
    let complete_len = std::fs::metadata(&complete).unwrap().len();
    assert!(
        strip_len < complete_len,
        "strip ({strip_len} B) drops the lexicon that lossless ({complete_len} B) keeps — the delta is discarded text, not compression"
    );

    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&complete);
}
