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
            .filter_map(|i| {
                let (ns, local) = lex.string_parts_at(i)?;
                let mut s = String::with_capacity(ns.len() + local.len());
                s.push_str(ns);
                s.push_str(local);
                Some(s)
            })
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
        "犬は動物です",             // Japanese
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

    // The GRAPH itself must be readable via the canonical reader — not just the lexicon. The old
    // ingest wrote headerless blocks that `read_all_quins` mis-parsed (objects came back as 0). Prove
    // every triple round-trips and its object hash resolves back to the stored literal.
    const MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF; // clears the 4-bit modality tag → the stored object field
    let quins = vol.read_all_quins().expect("read_all_quins");
    assert_eq!(
        quins.len(),
        5,
        "canonical reader must see all 5 triples (got {})",
        quins.len()
    );
    let lex = vol.lex_view().expect("lex");
    let resolvable = quins
        .iter()
        .filter(|q| lex.contains(q.object) || lex.contains(q.object & MASK))
        .count();
    assert_eq!(
        resolvable, 5,
        "every triple's object hash must resolve to its stored string (got {resolvable}/5)"
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

#[test]
fn pcat_100kb_owl_annotation_prose_roundtrips_without_bidx_pollution_and_fails_closed_on_1byte_corruption(
) {
    use qualia_core_db::q42_volume::{
        Q42InspectReport, StreamingQ42VolumeWriter, FLAG_PAYLOAD_CATALOGUE,
    };
    use qualia_core_db::query::lexicon::generate_60bit_token;
    use qualia_core_db::{q_hash, NQuin};
    use std::collections::HashMap;

    let out_path = temp("qualia_pcat_100kb_owl.q42");
    let corrupt_path = temp("qualia_pcat_100kb_owl_corrupt.q42");

    // Build a >100 KB multilingual OWL annotation prose document.
    let base_paragraph = "OWL 2 Annotation Assertion (ADR 0014): NaturalAgent sovereignty and human-rights provenance must never be collapsed into instrument identifiers (DID, VC, wallet, sensor). Suomi: koira on eläin. 日本語: 識別子はアイデンティティではない。 العربية: المعرفات ليست الهوية.\n";
    let mut owl_prose = String::with_capacity(110_000);
    while owl_prose.len() < 102_400 {
        owl_prose.push_str(base_paragraph);
    }
    assert!(
        owl_prose.len() >= 102_400,
        "expected >= 100 KiB OWL prose payload"
    );

    let subj = generate_60bit_token(b"https://qualia.world/ontology/idf#NaturalAgent");
    let pred = generate_60bit_token(b"http://www.w3.org/2000/01/rdf-schema#comment");
    let pcat_ref_obj = generate_60bit_token(b"urn:qualia:pcat:owl-annotation-natural-agent");
    let raw_prose_hash = generate_60bit_token(owl_prose.as_bytes());
    let raw_prose_qhash = q_hash(&owl_prose);

    let mut lex = HashMap::new();
    lex.insert(
        subj,
        "https://qualia.world/ontology/idf#NaturalAgent".to_string(),
    );
    lex.insert(
        pred,
        "http://www.w3.org/2000/01/rdf-schema#comment".to_string(),
    );
    lex.insert(
        pcat_ref_obj,
        "urn:qualia:pcat:owl-annotation-natural-agent".to_string(),
    );

    let mut writer = StreamingQ42VolumeWriter::new(&lex).expect("streaming writer");

    // Verify media MIME types and binary weight/manifold extensions are rejected per ADR 0014:
    for (bad_key, bad_mime) in [
        ("avatar.png", "image/png"),
        ("voice.wav", "audio/wav"),
        ("clip.mp4", "video/mp4"),
        ("weights.p64", "application/octet-stream"),
        ("manifold.10d", "application/octet-stream"),
    ] {
        assert!(
            writer
                .add_payload_catalogue_utf8(bad_key, bad_mime, None, "forbidden_media", "stub")
                .is_err(),
            "ADR 0014 must reject forbidden media/container ({bad_key}, {bad_mime})"
        );
    }

    let entry = writer
        .add_payload_catalogue_utf8(
            "urn:qualia:pcat:owl-annotation-natural-agent",
            "text/markdown",
            Some("mul"),
            "owl_annotation_prose",
            &owl_prose,
        )
        .expect("add 100 KB OWL prose to PCAT");
    assert_eq!(entry.byte_length, owl_prose.len() as u64);

    let mut quin = NQuin {
        subject: subj,
        predicate: pred,
        object: pcat_ref_obj,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    quin.recalculate_parity();
    writer.push_block(0, &[quin]).expect("push block");
    writer.finish(&out_path).expect("finish volume");

    let vol = Q42Volume::open(&out_path).expect("open volume");
    assert_ne!(
        vol.header().flags & FLAG_PAYLOAD_CATALOGUE,
        0,
        "FLAG_PAYLOAD_CATALOGUE (0x0080) must be set"
    );
    assert!(
        vol.header().payload_catalogue_range().is_some(),
        "header._reserved[48..64] must record PCAT (offset, length)"
    );

    // Ensure the 100 KB prose body is NOT polluting BIDX object index:
    assert!(
        vol.bidx_blocks_for_hash(raw_prose_hash).is_empty()
            && vol.bidx_blocks_for_hash(raw_prose_qhash).is_empty(),
        "oversized 100 KB OWL prose body must live in PCAT, not BIDX"
    );

    // Verify Q42InspectReport reports PCAT presence, entry count, and byte length:
    let inspect = Q42InspectReport::from_path(&out_path).expect("inspect report");
    assert!(inspect.has_payload_catalogue);
    assert_eq!(inspect.payload_catalogue_entries, Some(1));
    assert!(inspect.payload_catalogue_bytes > owl_prose.len() as u64);
    assert!(inspect
        .sections
        .iter()
        .any(|s| s.name == "payload-catalogue"));

    // Verify byte-exact UTF-8 round-trip via Q42Volume::read_payload_catalogue:
    let pcat = vol
        .read_payload_catalogue()
        .expect("read_payload_catalogue");
    assert_eq!(pcat.len(), 1);
    assert_eq!(
        pcat[0].payload_key,
        "urn:qualia:pcat:owl-annotation-natural-agent"
    );
    assert_eq!(pcat[0].mime_type, "text/markdown");
    assert_eq!(pcat[0].language.as_deref(), Some("mul"));
    assert_eq!(pcat[0].role, "owl_annotation_prose");
    assert_eq!(pcat[0].byte_length, owl_prose.len() as u64);
    assert_eq!(pcat[0].utf8_body, owl_prose);

    // Flip 1 byte inside the PCAT section and verify CRC-32C fails closed:
    let mut raw_bytes = std::fs::read(&out_path).expect("read volume bytes");
    let (pcat_offset, pcat_length) = vol.header().payload_catalogue_range().unwrap();
    let target_idx = (pcat_offset + pcat_length - 17) as usize;
    raw_bytes[target_idx] ^= 0x01;
    std::fs::write(&corrupt_path, &raw_bytes).expect("write corrupt volume");

    let corrupt_vol = Q42Volume::open(&corrupt_path).expect("open corrupt volume");
    let err = corrupt_vol
        .read_payload_catalogue()
        .expect_err("1-byte corruption in PCAT must fail closed on CRC-32C check");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        err.to_string().contains("CRC-32C"),
        "expected CRC-32C error message, got: {err}"
    );

    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&corrupt_path);
}

#[test]
fn sum_of_three_cubes_42_exact_256bit_crt_cascade_and_execution_profile() {
    use qualia_core_db::q42::execution_profile::{
        admit_for_policy_decision, ExactnessProfile, SolverVerificationOutcome,
    };
    use qualia_core_db::specialized_libs::symbolic_algebra::{
        decode_int_const_quin, from_quins, has_sum_of_three_cubes_mod9_obstruction,
        push_int_const_node, reduce_sum_of_three_cubes_divisor_discriminant,
        sum_of_three_cubes_expr, sum_of_three_cubes_quins,
        verify_divisor_discriminant_cascade_bigint, verify_sum_of_three_cubes_exact,
        verify_sum_of_three_cubes_rns_4prime,
    };

    // 1. Execution profile gate (ADR 0003 / Q42-08):
    assert!(admit_for_policy_decision(&ExactnessProfile::Exact).is_ok());
    assert!(admit_for_policy_decision(&ExactnessProfile::DeterministicApproximate {
        abs_bound: 1e-12,
        rel_bound: 1e-12,
    })
    .is_err());
    assert!(admit_for_policy_decision(&ExactnessProfile::InteractiveApproximate).is_err());
    let outcomes = [
        SolverVerificationOutcome::WitnessChecked,
        SolverVerificationOutcome::UnsatisfiableWithinDomain,
        SolverVerificationOutcome::UnknownWithinBudget,
        SolverVerificationOutcome::GlobalImpossibilityProved,
    ];
    assert_eq!(outcomes.len(), 4);

    // 2. Booker & Sutherland (2019) k = 42 solution:
    let x: i64 = -80_538_738_812_075_974;
    let y: i64 = 80_435_758_145_817_515;
    let z: i64 = 12_602_123_297_335_631;
    let k: i64 = 42;

    assert_eq!(verify_sum_of_three_cubes_exact(x, y, z, k), Ok(true));
    assert_eq!(verify_sum_of_three_cubes_exact(x, y, z + 1, k), Ok(false));

    // Naive f64 loses >35 bits of precision on 10^51 cubes and cannot distinguish z from z+1:
    let f64_sum = (x as f64).powi(3) + (y as f64).powi(3) + (z as f64).powi(3);
    let f64_sum_perturbed = (x as f64).powi(3) + (y as f64).powi(3) + ((z + 1) as f64).powi(3);
    assert_eq!(f64_sum, f64_sum_perturbed);
    assert_ne!(f64_sum, 42.0);

    // 3. Mod-9 Hasse-Minkowski local obstruction:
    assert!(!has_sum_of_three_cubes_mod9_obstruction(42));
    assert!(has_sum_of_three_cubes_mod9_obstruction(4));
    assert!(has_sum_of_three_cubes_mod9_obstruction(5));
    assert!(has_sum_of_three_cubes_mod9_obstruction(13));
    assert!(has_sum_of_three_cubes_mod9_obstruction(-4));
    assert!(has_sum_of_three_cubes_mod9_obstruction(-5));
    assert_eq!(verify_sum_of_three_cubes_exact(x, y, z, 4), Ok(false));

    // 4. Divisor/discriminant reduction cascade (both i128 and arbitrary-width BigInt):
    assert_eq!(
        reduce_sum_of_three_cubes_divisor_discriminant(3, 4, -1),
        Some((4, -5)),
        "(-5)^3 + 4^3 + 4^3 = 3 must reduce with z=4, s=-1"
    );
    assert!(
        verify_divisor_discriminant_cascade_bigint(x, y, z, 42).unwrap(),
        "Booker-Sutherland k=42 witness must satisfy the exact divisor/discriminant cascade"
    );
    assert!(!verify_divisor_discriminant_cascade_bigint(x, y, z + 1, 42).unwrap());

    // 5. 4-prime RNS CRT batch filter:
    assert!(verify_sum_of_three_cubes_rns_4prime(x, y, z, 42));
    assert!(!verify_sum_of_three_cubes_rns_4prime(x, y, z + 1, 42));

    // 6. NQuin post-order DAG encoding with 5-field XOR parity and cyclic-index rejection:
    let quins = sum_of_three_cubes_quins(42);
    for q in &quins {
        assert!(
            q.verify_ecc_parity(),
            "all CAS Quins must carry valid 5-field XOR parity"
        );
    }
    let decoded = from_quins(&quins).expect("decode post-order CAS Quins");
    assert_eq!(decoded, sum_of_three_cubes_expr(42));

    let mut cyclic = quins.clone();
    let last_idx = cyclic.len() - 1;
    cyclic[last_idx].object = last_idx as u64;
    cyclic[last_idx].recalculate_parity();
    assert!(from_quins(&cyclic).is_err());

    let mut int_quins = Vec::new();
    push_int_const_node(&mut int_quins, x);
    assert_eq!(decode_int_const_quin(&int_quins[0]), Some(x));
}

#[test]
fn pre_solver_qubo_penalty_sufficiency_theorem_enforces_feasible_ground_state() {
    use qualia_core_db::solvers::qpu::pre_solver::{
        minimum_sufficient_penalty, verify_qubo_penalty_sufficiency, Objective, ObjectiveType,
        PreSolver, ProblemDescription, QuboFormulation, QuboPenaltyBounds, Variable,
        VariableDomain,
    };
    use qualia_core_db::solvers::qpu::ProblemType;

    let problem = ProblemDescription {
        problem_type: ProblemType::Annealing,
        variables: vec![
            Variable {
                name: "x0".into(),
                domain: VariableDomain::Binary,
                index: 0,
            },
            Variable {
                name: "x1".into(),
                domain: VariableDomain::Binary,
                index: 1,
            },
        ],
        constraints: vec![],
        objective: Objective {
            objective_type: ObjectiveType::Linear,
            expression: "x0 + x1".into(),
            minimize: true,
        },
    };

    let insufficient = QuboPenaltyBounds {
        objective_lower_bound: -6.0,
        feasible_upper_bound: -3.0,
        min_violation_delta: 1.0,
        penalty_weight: 2.0,
    };
    assert!(verify_qubo_penalty_sufficiency(&insufficient).is_err());

    let mut solver = PreSolver::new();
    assert!(solver
        .formulate_penalized_qubo(&problem, &insufficient)
        .is_err());

    let p_suff = minimum_sufficient_penalty(-6.0, -3.0, 1.0, 1.0).unwrap();
    assert!((p_suff - 4.0).abs() < 1e-12);
    let sufficient = QuboPenaltyBounds {
        objective_lower_bound: -6.0,
        feasible_upper_bound: -3.0,
        min_violation_delta: 1.0,
        penalty_weight: p_suff,
    };
    assert!(verify_qubo_penalty_sufficiency(&sufficient).is_ok());
    assert!(solver
        .formulate_penalized_qubo(&problem, &sufficient)
        .is_ok());

    let mut good_qubo = QuboFormulation::new(2);
    good_qubo.add_linear_term(0, -3.0);
    good_qubo.add_linear_term(1, -3.0);
    good_qubo.add_quadratic_term(0, 1, p_suff);
    assert_eq!(good_qubo.evaluate_binary(&[1, 0]), -3.0);
    assert_eq!(good_qubo.evaluate_binary(&[1, 1]), -2.0);
    assert!(good_qubo.evaluate_binary(&[1, 1]) > good_qubo.evaluate_binary(&[1, 0]));
}

