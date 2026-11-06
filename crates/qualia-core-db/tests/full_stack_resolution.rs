//! Capstone — the whole hybrid-identity stack resolving one identifier end-to-end,
//! plus Packet `EVAL-22`: Unified Tri-Source Evaluation Aggregator (Etalon + Network -> QualiaDB `.q42` v4).

use qualia_core_db::indexing::QuinIndex;
use qualia_core_db::lexicon::LexiconInterner;
use qualia_core_db::modal_kind::{kind_name, tag_kind, KIND_DICTIONARY};
use qualia_core_db::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_OBLIGATE,
};
use qualia_core_db::net::qdnf::harness::tri_source_eval::{
    EtalonSourceReceipt, NetworkSourceReceipt, QualiaDbInvariantsReceipt,
    TriSourceEvaluationReport,
};
use qualia_core_db::net::qdnf::policy_labels::egress::{
    enumerate_egress_audit_quin, EgressAuditReceipt, EgressCaller,
};
use qualia_core_db::net::qdnf::policy_labels::projection::DerivedSink;
use qualia_core_db::net::qdnf::policy_labels::types::Confidentiality;
use qualia_core_db::net::qdnf::types::StrongDigest;
use qualia_core_db::q42::execution_profile::ExactnessProfile;
use qualia_core_db::q42::q42_volume::{
    Q42Volume, FLAG_PAYLOAD_CATALOGUE, FLAG_PERMISSIVE_COMMONS, FLAG_SANCTUARY, Q42_VERSION_V4,
};
use qualia_core_db::resolve::resolve_in_index;
use qualia_core_db::sparql_library::parsers::turtle_doc::parse_turtle_doc_into;
use qualia_core_db::sparql_library::quin_sink::QuinSink;
use qualia_core_db::{q_hash, NQuin};
use std::io;

const CONCEPT_N3: &str =
    include_str!("../../../core-ontologies/concepts/duty-to-suppress-forced-labour.n3");

/// A sink that does what real ingest does: collect quins AND intern terms into the
/// collision-aware lexicon (turtle_doc calls `push_lex` for every term).
#[derive(Default)]
struct CapstoneSink {
    quins: Vec<NQuin>,
    lex: LexiconInterner,
}
impl QuinSink for CapstoneSink {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        self.quins.push(q);
        Ok(())
    }
    fn push_lex(&mut self, hash: u64, term: &str) {
        self.lex.intern(hash, term);
    }
}

#[test]
fn whole_stack_resolves_one_identifier_end_to_end() {
    // ── 1. Ingest the real instrument concept (generate_60bit_token + lexicon intern). ──
    let mut sink = CapstoneSink::default();
    parse_turtle_doc_into(CONCEPT_N3.as_bytes(), 0, &mut sink).expect("concept ingests");
    let CapstoneSink { mut quins, lex } = sink;
    assert!(!quins.is_empty(), "ingest produced no quins");

    // ── 2. Tag the obligation node with a modal identifier-KIND (open kind fabric). ──
    let deontic = q_hash("https://ns.webcivics.net/concept/DTSFL-deontic");
    quins.push(tag_kind(deontic, KIND_DICTIONARY));

    // ── 3. Build the zero-alloc index and resolve the identifier through resolve.rs. ──
    let idx = QuinIndex::from_slice(&quins);
    let resolved = resolve_in_index(&idx, deontic);
    assert_eq!(resolved.kind, Some(KIND_DICTIONARY), "modal kind resolves");
    assert_eq!(kind_name(resolved.kind.unwrap()), Some("DictionaryHash"));
    assert!(
        resolved.out_degree >= 4,
        "obligation node has its relations + the kind edge (got {})",
        resolved.out_degree
    );

    // ── 4. #14 bridge: read the bearer + required act OUT OF the ingested corpus. ──
    let borne_by = q_hash("https://ns.webcivics.net/values/borneBy");
    let requires = q_hash("https://ns.webcivics.net/values/requires");
    let party = idx
        .object_of(deontic, borne_by)
        .expect("bearer found in corpus");
    let action = idx
        .object_of(deontic, requires)
        .expect("required act found in corpus");
    assert_eq!(party, q_hash("https://ns.webcivics.net/values/State"));
    assert_eq!(
        action,
        q_hash("https://ns.webcivics.net/action/SuppressForcedLabour")
    );

    // ── 5. Deontic governance: the State's obligation is Active over corpus terms. ──
    let contract = q_hash("contract:capstone:ilo-c105");
    let norm = compile_norm_quin(party, OP_OBLIGATE, action, action, contract, 0, false);
    let mut out = [DeonticVerdict::default(); 2];
    let n = evaluate_deontic_contract(&[norm], 1_717_200_000u32, &mut out).expect("deontic eval");
    assert_eq!(n, 1);
    assert_eq!(
        out[0].status,
        DeonticStatus::Active,
        "the State's obligation to suppress forced labour is in force"
    );

    // ── 6. Lexicon value recovery: the handle resolves to its human-readable IRI. ──
    assert_eq!(
        lex.resolve(party),
        Some("https://ns.webcivics.net/values/State"),
        "the bearer handle recovers its lexical value through the lexicon backstop"
    );
    assert_eq!(
        lex.resolve(action),
        Some("https://ns.webcivics.net/action/SuppressForcedLabour")
    );
}

#[test]
fn eval22_tri_source_evaluation_aggregator_bakes_verified_q42_v4_volume() {
    let etalon_receipts = [EtalonSourceReceipt {
        device_backend_hash: q_hash("urn:qualia:etalon:device:vulkan-dx12:0"),
        quin_count: 1024,
        sieve_matches_cpu: true,
        radix_order_matches_cpu: true,
        parity_verified_all: true,
        oracle_mismatches: 0,
        sidecar_custody_intact: true,
        hardware_lane_executed: true,
        exactness_profile: ExactnessProfile::Exact,
    }];

    let egress_receipt = EgressAuditReceipt {
        caller: EgressCaller::Vibe,
        sink: DerivedSink::Summary,
        confidentiality: Confidentiality::C0Public,
        recipient: StrongDigest([0x11; 48]),
        compartment: StrongDigest([0x22; 48]),
        purpose: StrongDigest([0x33; 48]),
        env_profile: 1,
        allowed: true,
    };
    let egress_quin = enumerate_egress_audit_quin(
        &egress_receipt,
        q_hash("urn:qualia:q42:profile:network:1"),
        1_730_900_000,
    );
    assert!(egress_quin.verify_ecc_parity());

    let network_receipts = [NetworkSourceReceipt {
        session_id_hash: q_hash("urn:qualia:qdnf:session:cscp:01"),
        handshake_transcript_bound: true,
        rekey_generation_verified: true,
        qsr_384bit_verified: true,
        spf_dual_witness_verified: true,
        superblock_replication_verified: true,
        locator_no_proxy_launder_verified: true,
        egress_receipt,
    }];

    let qualiadb_invariants = QualiaDbInvariantsReceipt {
        shacl_plane_shapes_valid: true,
        five_field_parity_verified: true,
        plane_join_prohibition_verified: true,
        usri_cb_usri_grounded: true,
        cb_usri_context_token: q_hash("urn:omni:v1:physical:L4:sol:earth") & 0x0FFF_FFFF_FFFF_FFFF,
        contains_sanctuary_claims: false,
    };

    let report = TriSourceEvaluationReport {
        etalon_receipts: &etalon_receipts,
        network_receipts: &network_receipts,
        qualiadb_invariants,
        evaluation_epoch: 1_730_900_000,
    };

    // 1. Verify exact per-plane counts and strict qualification pass
    let counts = report
        .verify_qualified()
        .expect("all three source planes must qualify");
    assert_eq!(counts.etalon.passed, 1);
    assert_eq!(counts.etalon.failed, 0);
    assert_eq!(counts.etalon.unsupported, 0);
    assert_eq!(counts.network.passed, 1);
    assert_eq!(counts.network.failed, 0);
    assert_eq!(counts.qualiadb.passed, 1);
    assert_eq!(counts.qualiadb.failed, 0);

    // 2. Bake into a unified .q42 v4 volume and verify Merkle + 5-field ECC parity + PCAT
    let bytes = report
        .bake_evaluation_to_q42_v4()
        .expect("bake_evaluation_to_q42_v4 must succeed");
    let tmp = tempfile::tempdir().expect("tempdir");
    let q42_path = tmp.path().join("tri_source_eval.q42");
    std::fs::write(&q42_path, &bytes).expect("write .q42");
    let vol = Q42Volume::open(&q42_path).expect("valid .q42 v4 volume");
    let header_version = { vol.header().version };
    let header_flags = { vol.header().flags };
    assert_eq!(header_version, Q42_VERSION_V4);
    assert_ne!(header_flags & FLAG_PAYLOAD_CATALOGUE, 0);
    assert_ne!(header_flags & FLAG_PERMISSIVE_COMMONS, 0);
    assert_eq!(header_flags & FLAG_SANCTUARY, 0);
    let receipt = vol
        .verify_all_blocks()
        .expect("all SuperBlocks and 5-field XOR parity must verify 100%");
    assert_eq!(receipt.blocks_verified, 1);
    assert_eq!(receipt.quins_verified, 14);

    let pcat = vol
        .read_payload_catalogue()
        .expect("PCAT decode must succeed");
    assert_eq!(pcat.len(), 1);
    assert_eq!(pcat[0].entry.role, "tri-source-diagnostic-summary");

    // 3. Verify that an unrun/unsupported hardware lane or failed security gate never hides behind a pass percentage
    let mut unsupported_etalon = etalon_receipts;
    unsupported_etalon[0].hardware_lane_executed = false;
    let unsupported_report = TriSourceEvaluationReport {
        etalon_receipts: &unsupported_etalon,
        network_receipts: &network_receipts,
        qualiadb_invariants,
        evaluation_epoch: 1_730_900_000,
    };
    let unsupported_counts = unsupported_report.summarize_counts();
    assert_eq!(unsupported_counts.etalon.unsupported, 1);
    assert!(!unsupported_counts.is_qualified());
    assert!(unsupported_report.verify_qualified().is_err());

    // 4. Verify that InteractiveApproximate is rejected for policy qualification (ADR 0003)
    let mut approx_etalon = etalon_receipts;
    approx_etalon[0].exactness_profile = ExactnessProfile::InteractiveApproximate;
    let approx_report = TriSourceEvaluationReport {
        etalon_receipts: &approx_etalon,
        network_receipts: &network_receipts,
        qualiadb_invariants,
        evaluation_epoch: 1_730_900_000,
    };
    assert!(approx_report.verify_qualified().is_err());
}
