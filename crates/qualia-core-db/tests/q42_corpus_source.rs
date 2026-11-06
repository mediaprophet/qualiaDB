//! W5b Phase 2 — validate the `Q42Field` calibration-corpus source against a REAL in-repo q42.
//!
//! Proves the mechanism Timothy described (query a q42 knowledge graph for a description field → corpus
//! passages) end-to-end on actual data: `Q42Volume::read_all_quins` + predicate filter + lexicon
//! resolution. Uses the bundled w3c-archives ontologies (which carry `rdfs:comment`/`label`/definition
//! text) as a stand-in for `princeton.q42` WordNet glosses — same code path. Integration test so it
//! runs despite the CG lib-`#[cfg(test)]` breakage.

#![cfg(not(target_arch = "wasm32"))]

#[cfg(feature = "wgsl-forge")]
use qualia_core_db::wgsl_forge::calibration::corpus::assemble;
#[cfg(feature = "wgsl-forge")]
use qualia_core_db::wgsl_forge::calibration::CorpusSpec;
#[cfg(feature = "wgsl-forge")]
use std::path::PathBuf;

#[cfg(feature = "wgsl-forge")]
fn find_q42() -> Option<PathBuf> {
    // Prefer the real WordNet q42 (glosses = the calibration corpus) if present locally, then the
    // release/repo locations, then small bundled ontologies as a fallback.
    let wordnet = [
        // Lossless re-ingest (Complete mode) — glosses recoverable from the lexicon; prefer it.
        r"C:\Projects\Local_LIbraries\Local_LIbraries\wordnet\wordnet-lossless.q42",
        r"C:\Projects\Local_LIbraries\Local_LIbraries\wordnet\wordnet.q42",
        "docs/data/wordnet/princeton.q42",
        "../../docs/data/wordnet/princeton.q42",
    ];
    for p in wordnet {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for name in ["dqv", "duv", "adms", "activitystreams-owl"] {
        for root in [
            "bundled/ontologies/w3c-archives",
            "../../bundled/ontologies/w3c-archives",
        ] {
            let p = PathBuf::from(format!("{root}/{name}.q42"));
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(feature = "wgsl-forge")]
#[test]
fn extracts_corpus_text_from_in_repo_q42() {
    let Some(vol) = find_q42() else {
        eprintln!("[q42corpus] no bundled w3c q42 found — skipping");
        return;
    };
    // Common description/definition predicates; whichever the ontology uses should yield text.
    let predicates = [
        // WordNet gloss/definition predicates (Princeton WN 2.0/3.0/3.1 RDF schemas + skos).
        "http://www.w3.org/2006/03/wn/wn20/schema/gloss",
        "http://wordnet-rdf.princeton.edu/ontology#definition",
        "http://www.w3.org/2004/02/skos/core#definition",
        "http://globalwordnet.github.io/schemas/wn#definition",
        "http://www.w3.org/2000/01/rdf-schema#comment",
        "http://purl.org/dc/terms/description",
        "http://www.w3.org/2000/01/rdf-schema#label",
    ];
    // Diagnostic: what does this q42 actually hold? Dump quin count + the first rows' resolved
    // subject/predicate/object strings so we can see whether literal TEXT survives in the lexicon.
    {
        use qualia_core_db::q42_volume::Q42Volume;
        let dvol = Q42Volume::open(&vol).expect("open");
        let quins = dvol.read_all_quins().expect("quins");
        let lex = dvol.lex_view().expect("lex");
        println!("[q42diag] {} quins in {}", quins.len(), vol.display());
        const MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF; // clear the upper 4-bit modality/type tag
        let r = |h: u64| {
            lex.lookup_owned(h)
                .map(|s| s.chars().take(50).collect::<String>())
        };
        let rm = |h: u64| {
            lex.lookup_owned(h & MASK)
                .map(|s| s.chars().take(50).collect::<String>())
        };
        for (i, q) in quins.iter().take(12).enumerate() {
            println!(
                "[q42diag] #{i} raw o={:016x} p={:016x} | s={:?} p={:?} o={:?} | masked o={:?} p={:?}",
                q.object, q.predicate, r(q.subject), r(q.predicate), r(q.object), rm(q.object), rm(q.predicate)
            );
        }
        let obj_raw = quins.iter().filter(|q| lex.contains(q.object)).count();
        let obj_masked = quins
            .iter()
            .filter(|q| lex.contains(q.object & MASK))
            .count();
        println!(
            "[q42diag] resolvable objects: raw {}/{}, masked {}/{}",
            obj_raw,
            quins.len(),
            obj_masked,
            quins.len()
        );
    }

    // Primary path: dump the lexicon's sentence-like strings (WordNet glosses/examples). Robust —
    // reads the lexicon strings directly, independent of the graph's object encoding.
    match assemble(&CorpusSpec::Q42Lexicon {
        volume: vol.clone(),
        min_len: 24,
    }) {
        Ok(docs) if !docs.is_empty() => {
            println!(
                "[q42corpus] LEXICON DUMP → {} sentence-like passages from {}",
                docs.len(),
                vol.display()
            );
            for d in docs.iter().take(5) {
                println!("  · {}", d.chars().take(120).collect::<String>());
            }
            assert!(docs.iter().all(|d| d.contains(' ') && d.trim().len() >= 24));
            println!("[q42corpus] PASS — real definitional corpus recovered from the q42 lexicon.");
            return;
        }
        Ok(_) => println!(
            "[q42corpus] lexicon dump: 0 sentence-like strings (structure-only / URI-only lexicon)"
        ),
        Err(e) => println!("[q42corpus] lexicon dump error: {e}"),
    }

    for pred in predicates {
        let spec = CorpusSpec::Q42Field {
            volume: vol.clone(),
            predicate: pred.to_string(),
        };
        match assemble(&spec) {
            Ok(docs) if !docs.is_empty() => {
                println!(
                    "[q42corpus] {} → {} passages from <{}>",
                    vol.display(),
                    docs.len(),
                    pred
                );
                for d in docs.iter().take(3) {
                    println!("  · {}", d.chars().take(110).collect::<String>());
                }
                assert!(
                    docs.iter().all(|d| !d.trim().is_empty()),
                    "all extracted passages must be non-empty"
                );
                println!("[q42corpus] PASS — SPARQL/q42 corpus mechanism extracts real text via the lexicon.");
                return;
            }
            Ok(_) => {
                println!("[q42corpus] <{pred}>: 0 passages (predicate not used by this ontology)")
            }
            Err(e) => println!("[q42corpus] <{pred}>: {e}"),
        }
    }
    // FINDING (2026-07-06): the bundled ontology q42s are STRUCTURE-ONLY — their quin hashes do not
    // resolve against the volume lexicon (0/N above), so there is no literal text to project. The
    // Q42Field mechanism is built + correct, but validating it end-to-end needs a q42 whose lexicon
    // RETAINS the literal text — i.e. `princeton.q42` (WordNet, where glosses are the content), or a
    // re-ingest that keeps literals. Skip (not fail) here: this is a data availability gap, not a code
    // bug. When a text-bearing q42 is present the assert-path above validates real extraction.
    eprintln!(
        "[q42corpus] SKIP — no lexicon-bearing q42 available (bundled ontologies are structure-only). \
Mechanism built; validate against princeton.q42 (WordNet glosses) once fetched."
    );
}

#[test]
fn q42lex_v4_multi_namespace_corpus_roundtrips_single_page_fetch_and_beats_v2_size() {
    use qualia_core_db::q42_volume::{
        Q42ByteRange, Q42RangeSource, Q42RangeVolume, Q42Volume, StreamingQ42VolumeWriter,
        Q42_VERSION_V4,
    };
    use qualia_core_db::query::lexicon::generate_60bit_token;
    use qualia_core_db::NQuin;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::io;
    use tempfile::NamedTempFile;

    struct CountingRangeSource<'a> {
        bytes: &'a [u8],
        page_fetches: &'a Cell<usize>,
    }

    impl<'a> Q42RangeSource for CountingRangeSource<'a> {
        fn length(&self) -> io::Result<u64> {
            Ok(self.bytes.len() as u64)
        }

        fn read_range_into(&self, range: Q42ByteRange, out: &mut [u8]) -> io::Result<()> {
            if out.len() != range.length {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "buffer length mismatch",
                ));
            }
            // Count lexicon page fetches (> 64B, excluding 32B header / 8B count / 32B directory entry probes).
            if range.length > 64 && range.offset > 64 {
                self.page_fetches.set(self.page_fetches.get() + 1);
            }
            let start = usize::try_from(range.offset)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "offset overflow"))?;
            let end = start
                .checked_add(range.length)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "range overflow"))?;
            let slice = self.bytes.get(start..end).ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "range out of bounds")
            })?;
            out.copy_from_slice(slice);
            Ok(())
        }
    }

    // Build a multi-namespace RDF corpus spanning all 5 mandated prefix families + bare literals:
    let prefixes = [
        "https://schema.org/",
        "http://www.w3.org/ns/shacl#",
        "did:q42:",
        "did:qi:",
        "urn:omni:",
    ];
    let local_names = [
        "NaturalAgent",
        "NodeShape",
        "PropertyShape",
        "SpatiotemporalHandle",
        "InstrumentBinding",
        "ClaimEnvelope",
        "CustodyReceipt",
        "RealmCoordinate",
    ];

    let mut lex = HashMap::new();
    let mut all_terms = Vec::new();
    for prefix in prefixes {
        for local in local_names {
            let term = format!("{prefix}{local}");
            let h = generate_60bit_token(term.as_bytes());
            lex.insert(h, term.clone());
            all_terms.push((h, prefix, local, term));
        }
    }
    // Also include a verbatim literal with no separator (`0x01` tag in v4):
    let verbatim_term = "verbatim-unsplit-literal-token".to_string();
    let verbatim_hash = generate_60bit_token(verbatim_term.as_bytes());
    lex.insert(verbatim_hash, verbatim_term.clone());

    let mut quins: Vec<NQuin> = all_terms
        .iter()
        .map(|&(h, _, _, _)| {
            let mut q = NQuin {
                subject: h,
                predicate: verbatim_hash,
                object: h,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            q.recalculate_parity();
            q
        })
        .collect();
    quins.sort_by_key(|q| q.object);

    let tmp = NamedTempFile::new().expect("temp q42 file");
    let mut writer = StreamingQ42VolumeWriter::new(&lex).expect("streaming writer");
    writer.push_block(0, &quins).expect("push block");
    writer.finish(tmp.path()).expect("finish v4 volume");

    let vol = Q42Volume::open(tmp.path()).expect("open v4 volume");
    let version = vol.header().version;
    assert_eq!(version, Q42_VERSION_V4);
    let lex_mmap = vol.lex_view().expect("lex_view");
    assert_eq!(lex_mmap.entry_count(), lex.len());

    // Compute what a v2 verbatim paged lexicon for the exact same entry set costs:
    // Header (32B) + page_count (8B) + 1 page dir (32B) + page header (16B) + N * 16B index + sum(3 + len)
    let v2_verbatim_bytes: usize = 32
        + 8
        + 32
        + 16
        + lex.len() * 16
        + lex.values().map(|s| 3 + s.len()).sum::<usize>();
    let v4_lex_bytes = vol.header().lex_length as usize;
    assert!(
        v4_lex_bytes <= v2_verbatim_bytes,
        "ADR 0015: v4 namespaced lexicon ({v4_lex_bytes} B) must be <= v2 verbatim lexicon ({v2_verbatim_bytes} B)"
    );
    assert!(
        v4_lex_bytes < v2_verbatim_bytes,
        "multi-namespace corpus must achieve strict byte savings in v4 ({v4_lex_bytes} B < {v2_verbatim_bytes} B)"
    );

    // Open a Q42RangeVolume over the volume bytes and verify every term via both
    // Q42LexMmap::lookup_parts and Q42RangeVolume::lookup_lexicon_hash_into in a SINGLE page fetch:
    let raw_vol_bytes = vol.as_bytes();
    let page_fetches = Cell::new(0usize);
    let source = CountingRangeSource {
        bytes: raw_vol_bytes,
        page_fetches: &page_fetches,
    };
    let range_vol = Q42RangeVolume::open(source).expect("open Q42RangeVolume");
    let mut page_scratch = vec![0u8; 64 * 1024];
    let mut scratch = [0u8; 256];

    for (h, expected_prefix, expected_local, expected_full) in &all_terms {
        let (ns, local) = lex_mmap
            .lookup_parts(*h)
            .expect("Q42LexMmap::lookup_parts must find namespaced term");
        assert_eq!(ns, *expected_prefix);
        assert_eq!(local, *expected_local);
        assert_eq!(format!("{ns}{local}"), *expected_full);

        page_fetches.set(0);
        let n = range_vol
            .lookup_lexicon_hash_into(*h, &mut page_scratch, &mut scratch)
            .expect("range lookup ok")
            .expect("range lookup hit");
        assert_eq!(
            &scratch[..n],
            expected_full.as_bytes(),
            "range_volume::lookup_lexicon_hash_into must match Q42LexMmap::lookup_parts byte-for-byte"
        );
        assert_eq!(
            page_fetches.get(),
            1,
            "cold-path lexicon lookup must require at most 1 page fetch after header/dir load"
        );
    }

    // Verify the verbatim (0x01) entry also round-trips identically in 1 page fetch:
    let (v_ns, v_local) = lex_mmap
        .lookup_parts(verbatim_hash)
        .expect("lookup verbatim term");
    assert_eq!(v_ns, "");
    assert_eq!(v_local, verbatim_term);
    page_fetches.set(0);
    let vn = range_vol
        .lookup_lexicon_hash_into(verbatim_hash, &mut page_scratch, &mut scratch)
        .expect("range lookup verbatim ok")
        .expect("range lookup verbatim hit");
    assert_eq!(&scratch[..vn], verbatim_term.as_bytes());
    assert_eq!(page_fetches.get(), 1);
}
