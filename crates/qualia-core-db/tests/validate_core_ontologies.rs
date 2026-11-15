//! `validate_core_ontologies` — native (Rust, zero-Python) governance gate for the
//! values-credential corpus (PLAN.md §9.1 / §17.1.3). A failing CI check.
//!
//! HARD violations (fail the build): namespace regression (`qualia.id/ns`), scraper
//! boilerplate surviving into the corpus, structural breakage (an instrument that is not a
//! `values:ValuesCredential` with a title, a source, and ≥1 provision), a Tier-B file outside
//! `mutable/`, or a missing spine file.
//!
//! WARN (reported, not fatal): governance-field coverage (`tier`/`legalForm`/`bindingStatus`/
//! `curationStatus`) — applied by the curation pass, so these tighten over time rather than
//! blocking day one.
//!
//! Textual checks by design: fast, dependency-free, and it guards exactly the regressions that
//! have actually bitten this corpus (the namespace migration, the scraped `Download: PDF`).
//! Full Turtle validation is the rdflib pass in `tools/build_index.py`.

use std::fs;
use std::path::{Path, PathBuf};

fn core_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../core-ontologies")
        .canonicalize()
        .expect("core-ontologies/ must exist relative to the crate")
}

fn ontologies_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ontologies")
        .canonicalize()
        .expect("ontologies/ must exist relative to the crate")
}

fn n3_files(dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("n3") {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

const SPINE: [&str; 8] = [
    "selfhood.n3",
    "values.n3",
    "agency.n3",
    "sense.n3",
    "tiering.n3",
    "policy.n3",
    "humanitarian-ict.n3",
    "license-rights.n3",
];
const BOILERPLATE: [&str; 5] = [
    "Download: PDF",
    "Download:",
    "Table of Contents",
    "Print this page",
    "Share via",
];
const GOV_FIELDS: [&str; 4] = ["tier", "legalForm", "bindingStatus", "curationStatus"];

#[test]
fn validate_core_ontologies() {
    let root = core_dir();
    let mut hard: Vec<String> = Vec::new();
    let mut warn: Vec<String> = Vec::new();

    // ── Spine files must exist ──
    for f in SPINE {
        if !root.join(f).exists() {
            hard.push(format!("missing spine file: {f}"));
        }
    }

    // ── Instruments: un-instruments/ + regional/ + mutable/ ──
    let mut instruments = n3_files(&root.join("un-instruments"));
    instruments.extend(n3_files(&root.join("regional")));
    instruments.extend(n3_files(&root.join("mutable")));

    let mut gov_present = [0usize; GOV_FIELDS.len()];

    for f in &instruments {
        let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        let txt = fs::read_to_string(f).unwrap_or_default();

        // HARD: namespace regression guard (the migration must stay done).
        if txt.contains("qualia.id/ns") {
            hard.push(format!("{rel}: residual `qualia.id/ns` namespace"));
        }
        // HARD: scraper boilerplate must never survive into the corpus.
        if let Some(b) = BOILERPLATE.iter().find(|b| txt.contains(*b)) {
            hard.push(format!("{rel}: scraper boilerplate `{b}`"));
        }
        // HARD: structural — a real values credential with title, source, ≥1 provision.
        if !txt.contains("values:ValuesCredential") {
            hard.push(format!("{rel}: not a `values:ValuesCredential`"));
        }
        if !txt.contains("dc:title") {
            hard.push(format!("{rel}: missing `dc:title`"));
        }
        if !txt.contains("values:source") {
            hard.push(format!("{rel}: missing `values:source`"));
        }
        if !txt.contains("values:partOf") {
            hard.push(format!("{rel}: no provisions (`values:partOf`)"));
        }
        // HARD: a Tier-B (Mutable) instrument must live under mutable/.
        if txt.contains("values:Mutable") && !rel.replace('\\', "/").contains("mutable/") {
            hard.push(format!(
                "{rel}: Tier-B (values:Mutable) instrument outside `mutable/`"
            ));
        }
        // WARN: governance-field coverage (curation pass tightens these).
        for (i, fld) in GOV_FIELDS.iter().enumerate() {
            if txt.contains(&format!("values:{fld} ")) {
                gov_present[i] += 1;
            }
        }
    }

    let n = instruments.len();
    for (i, fld) in GOV_FIELDS.iter().enumerate() {
        if gov_present[i] < n {
            warn.push(format!(
                "governance `values:{fld}`: {}/{n} present (curation pass pending)",
                gov_present[i]
            ));
        }
    }

    eprintln!(
        "validate_core_ontologies: scanned {n} instruments + {} spine files",
        SPINE.len()
    );
    for w in &warn {
        eprintln!("  WARN: {w}");
    }
    assert!(
        n >= 100,
        "expected the full corpus (≥100 instruments); found {n} — acquisition/dedup regression?"
    );
    if !hard.is_empty() {
        for h in &hard {
            eprintln!("  FAIL: {h}");
        }
        panic!(
            "validate_core_ontologies: {} HARD violation(s) — see FAIL lines above",
            hard.len()
        );
    }
    eprintln!("validate_core_ontologies: PASS — no hard violations.");
}

#[test]
fn license_rights_ontology_parses_into_nquins_and_enforces_shacl_non_thing() {
    use qualia_core_db::modalities::logic::n3_parser::{N3Event, N3Parser, Term};
    use qualia_core_db::{q_hash, NQuin};

    let path = core_dir().join("license-rights.n3");
    let content = fs::read_to_string(&path).expect("license-rights.n3 must be readable");

    assert!(content.contains("hro:CreatorProtectionPolicyShape"));
    assert!(content.contains("hro:LedgerPrivacyShape"));
    assert!(content.contains("hro:NoAutonomousAdjudicationShape"));
    assert!(!content.contains("rdfs:subClassOf owl:Thing"));
    let forbidden_term = format!("{}{}", "sover", "eign");
    assert!(!content.to_ascii_lowercase().contains(&forbidden_term));

    let term_hash = |term: Term<'_>| match term {
        Term::Uri(s) | Term::Literal(s) | Term::Variable(s) | Term::Formula(s) => q_hash(s),
    };
    let mut quins = Vec::new();
    let mut parser = N3Parser::new(&content);
    parser
        .parse_all(|event| {
            if let N3Event::StaticTriple(t) = event {
                let s = term_hash(t.subject);
                let p = term_hash(t.predicate);
                let o = term_hash(t.object);
                let ctx = 0x4852_4F00u64;
                quins.push(NQuin {
                    subject: s,
                    predicate: p,
                    object: o,
                    context: ctx,
                    metadata: 0,
                    parity: NQuin::calculate_parity(s, p, o, ctx, 0),
                });
            }
            Ok(())
        })
        .expect("license-rights.n3 must parse through N3Parser");
    assert!(
        quins.len() >= 60,
        "expected >= 60 NQuins from license-rights.n3, got {}",
        quins.len()
    );
}

#[test]
fn identifier_fabric_shapes_compile_and_enforce_plane_separation_and_co_attestation() {
    use qualia_core_db::modalities::logic::shacl::turtle_shapes::{
        shapes_from_turtle, validate_turtle_shapes,
    };

    let path = ontologies_dir().join("identifier_fabric_shapes.ttl");
    let ttl = fs::read_to_string(&path).expect("identifier_fabric_shapes.ttl must exist");
    assert!(!ttl.contains("owl:sameAs"));
    assert!(!ttl.contains("rdfs:subClassOf owl:Thing"));

    let compiled = shapes_from_turtle(&ttl).expect("identifier_fabric_shapes.ttl must compile");
    assert!(
        compiled.len() >= 25,
        "expected all plane, co-attestation, and F2 §§14-27 shapes to compile, got {}",
        compiled.len()
    );

    // 1. Valid NaturalAgent + 2-member CoAttestationBundle conforms.
    let valid_data = r#"
:alice a idf:NaturalAgent ;
    idf:plane "NaturalAgent" ;
    idf:framing "living-SHACL" .
:bundle1 a idf:CoAttestationBundle ;
    idf:attestationMember :m1 , :m2 ;
    idf:timeInterval "2026-11-06T00:00:00Z/2026-11-06T01:00:00Z" ;
    idf:memberKeyRole "SessionAuthentication" .
"#;
    let ok_report = validate_turtle_shapes(valid_data, &ttl).expect("validate valid_data");
    assert!(
        ok_report.conforms,
        "valid NaturalAgent + 2-member CoAttestationBundle must conform: {:?}",
        ok_report.results
    );

    // 2. Single-member CoAttestationBundle violates sh:minCount 2.
    let single_member_data = r#"
:bundle_bad a idf:CoAttestationBundle ;
    idf:attestationMember :m1 ;
    idf:timeInterval "2026-11-06T00:00:00Z/2026-11-06T01:00:00Z" ;
    idf:memberKeyRole "SessionAuthentication" .
"#;
    let single_report =
        validate_turtle_shapes(single_member_data, &ttl).expect("validate single_member_data");
    assert!(
        !single_report.conforms,
        "single-member CoAttestationBundle must fail sh:minCount 2"
    );

    // 3. NaturalAgent also typed as owl:Thing violates sh:not.
    let thing_agent_data = r#"
:alice_thing a idf:NaturalAgent ;
    a owl:Thing ;
    idf:plane "NaturalAgent" ;
    idf:framing "living-SHACL" .
"#;
    let thing_report =
        validate_turtle_shapes(thing_agent_data, &ttl).expect("validate thing_agent_data");
    assert!(
        !thing_report.conforms,
        "NaturalAgent typed as owl:Thing must fail sh:not"
    );

    // 4. CoAttestationBundle with an unknown key role violates sh:in.
    let bad_role_data = r#"
:bundle_role_bad a idf:CoAttestationBundle ;
    idf:attestationMember :m1 , :m2 ;
    idf:timeInterval "2026-11-06T00:00:00Z/2026-11-06T01:00:00Z" ;
    idf:memberKeyRole "InventedKeyRole" .
"#;
    let role_report = validate_turtle_shapes(bad_role_data, &ttl).expect("validate bad_role_data");
    assert!(
        !role_report.conforms,
        "CoAttestationBundle with unknown memberKeyRole must fail sh:in"
    );

    // 5. Unknown sh:* predicate fails closed at compile time.
    let bad_shacl_ttl = r#"
idf:BrokenShape a sh:NodeShape ;
    sh:targetClass idf:Broken ;
    sh:property idf:Broken-prop .
idf:Broken-prop sh:path idf:foo ;
    sh:inventedUnknownPredicate 1 .
"#;
    assert!(
        shapes_from_turtle(bad_shacl_ttl).is_err(),
        "unknown sh:* predicate must fail closed"
    );
}

#[test]
fn cbor_ld_five_field_parity_exact_cursor_and_n3_ingest_stream() {
    use qualia_core_db::query::cbor_compiler::{
        parse_cbor_ld_to_quin, parse_cbor_ld_to_quin_sized,
    };
    use qualia_core_db::query::ingest_formats::parse_bytes_to_raw;
    use qualia_core_db::query::ingest_job::IngestRdfFormat;
    use qualia_core_db::NQuin;

    // 4-element CBOR-LD Quin frame
    let cbor_4: [u8; 13] = [
        0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0,
    ];
    let (q4, used4) = parse_cbor_ld_to_quin_sized(&cbor_4).unwrap();
    assert_eq!(used4, 13);
    assert_eq!(q4.parity, NQuin::calculate_parity(1000, 2000, 3000, 4000, 0));
    assert!(q4.verify_ecc_parity());

    // 5-element CBOR-LD Quin frame with metadata routing bits
    let meta: u64 = (0b10u64 << 61) | 42;
    let mut cbor_5 = [0u8; 22];
    cbor_5[0] = 0x85;
    cbor_5[1..13].copy_from_slice(&cbor_4[1..13]);
    cbor_5[13] = 27;
    cbor_5[14..22].copy_from_slice(&meta.to_be_bytes());
    let (q5, used5) = parse_cbor_ld_to_quin_sized(&cbor_5).unwrap();
    assert_eq!(used5, 22);
    assert_eq!(q5.metadata, meta);
    assert_eq!(
        q5.parity,
        NQuin::calculate_parity(1000, 2000, 3000, 4000, meta)
    );
    assert!(q5.verify_ecc_parity());
    assert_eq!(parse_cbor_ld_to_quin(&cbor_5).unwrap(), q5);

    // Concatenated CBOR-LD stream with embedded 0x84 byte inside integer payload
    let frame_with_0x84: [u8; 13] = [
        0x84, 0x19, 0x00, 0x84, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0,
    ];
    let mut stream = Vec::new();
    stream.extend_from_slice(&frame_with_0x84);
    stream.extend_from_slice(&cbor_5);
    let raw_cbor = parse_bytes_to_raw(IngestRdfFormat::CborLd, &stream).unwrap();
    assert_eq!(raw_cbor.len(), 2);
    assert_eq!(raw_cbor[0].subject, "132");
    assert_eq!(raw_cbor[1].subject, "1000");

    // IngestRdfFormat::N3 streaming static triples + reified rule formulas
    let n3_src = b":alice a :NaturalAgent .\n{ ?c a :CorporatePerson } => { ?c :flag :Deny } .\n";
    let raw_n3 = parse_bytes_to_raw(IngestRdfFormat::N3, n3_src).unwrap();
    assert!(raw_n3.len() >= 4);
}
