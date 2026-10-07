//! WASM-bindgen API — semantic domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
// ─── SHACL: inline constraint validation ─────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ShaclValidateParams {
    pub constraint_type: String,
    pub value: f64,
    pub target_value: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_shacl_constraint_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ShaclValidateParams = serde_wasm_bindgen::from_value(val)?;
    let passes = match p.constraint_type.as_str() {
        "minInclusive" => p.target_value >= p.value,
        "maxInclusive" => p.target_value <= p.value,
        "minExclusive" => p.target_value > p.value,
        "maxExclusive" => p.target_value < p.value,
        "minCount" | "minLength" => p.target_value >= p.value,
        "maxCount" | "maxLength" => p.target_value <= p.value,
        other => {
            return Err(JsValue::from_str(&format!(
                "unsupported numeric SHACL constraint: {other}"
            )));
        }
    };
    #[derive(Serialize)]
    struct ValidationOut {
        passes: bool,
        constraint_type: String,
        value: f64,
        target_value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&ValidationOut {
        passes,
        constraint_type: p.constraint_type,
        value: p.value,
        target_value: p.target_value,
    })?)
}

/// Full graph SHACL validation from N3/N-Triples data and JSON ShapeSpecs.
/// Returns the complete `ValidationReport` preserving conforms, focus node, path,
/// severity, and constraint component.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_shacl_json_wasm(data_n3: &str, shapes_json: &str) -> Result<JsValue, JsValue> {
    let report_json =
        crate::modalities::logic::shacl::text_input::validate_json(data_n3, shapes_json)
            .map_err(|e| JsValue::from_str(&e))?;
    let val: serde_json::Value = serde_json::from_str(&report_json)
        .map_err(|e| JsValue::from_str(&format!("failed to parse report json: {e}")))?;
    Ok(serde_wasm_bindgen::to_value(&val)?)
}

/// Validates raw packed 48-byte Quins against a list of JSON ShapeSpecs.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_shacl_graph_wasm(db_bytes: &[u8], shapes_json: &str) -> Result<JsValue, JsValue> {
    if db_bytes.len() % 48 != 0 {
        return Err(JsValue::from_str(
            "db_bytes length must be a multiple of 48",
        ));
    }
    let quins = unsafe {
        std::slice::from_raw_parts(
            db_bytes.as_ptr() as *const crate::NQuin,
            db_bytes.len() / 48,
        )
    };
    let specs: Vec<crate::modalities::logic::shacl::text_input::ShapeSpec> =
        serde_json::from_str(shapes_json)
            .map_err(|e| JsValue::from_str(&format!("invalid shapes JSON: {e}")))?;
    let shapes: Vec<crate::modalities::logic::shacl::CompiledShape> = specs
        .iter()
        .map(crate::modalities::logic::shacl::text_input::shape_from_spec)
        .collect();
    let engine = crate::modalities::logic::shacl::ShaclEngine::new(quins, &shapes);
    let report = engine.validate(&|_h| None);
    Ok(serde_wasm_bindgen::to_value(&report)?)
}

/// Machine-readable SHACL capability and constraint coverage manifest (QW-05).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_shacl_capability_manifest_wasm() -> JsValue {
    #[derive(Serialize)]
    struct ShaclManifest {
        standard_core: Vec<&'static str>,
        extended_modalities: Vec<&'static str>,
        computational_economics: Vec<&'static str>,
        turtle_nodeshape: bool,
        profile: &'static str,
        engine_version: &'static str,
    }
    let m = ShaclManifest {
        standard_core: vec![
            "minInclusive",
            "maxInclusive",
            "minExclusive",
            "maxExclusive",
            "minCount",
            "maxCount",
            "minLength",
            "maxLength",
            "pattern",
            "class",
            "datatype",
            "nodeKind",
            "in",
            "hasValue",
            "equals",
            "lessThan",
            "lessThanOrEquals",
            "greaterThan",
            "greaterThanOrEquals",
            "node",
            "not",
            "and",
            "or",
            "xone",
            "closed",
            "languageIn",
            "uniqueLang",
        ],
        extended_modalities: vec![
            "deonticObligate",
            "deonticPermit",
            "deonticForbid",
            "deonticNotExpired",
            "epistemicKnowledge",
            "epistemicBelief",
            "commonKnowledge",
            "ltlConstraint",
            "paraconsistentConstraint",
            "calculusConstraint",
            "graphConstraint",
            "argumentationConstraint",
            "dialecticalConstraint",
            "valuesConsentNonCoerced",
            "valuesHarmBelowCeiling",
            "fuzzyMinDegree",
        ],
        computational_economics: vec![
            "econVaRPositive",
            "econWelfareAboveFloor",
            "econRiskBelowThreshold",
            "econPositivePrice",
            "econConvergedModel",
        ],
        turtle_nodeshape: true,
        profile: "qualia-shacl-0.0.39-full",
        engine_version: "0.0.39",
    };
    serde_wasm_bindgen::to_value(&m).unwrap_or(JsValue::NULL)
}

/// Compile Turtle / N3 `sh:NodeShape` documents into ShapeSpec-compatible JSON (UE-050).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compile_shacl_turtle_wasm(turtle: &str) -> Result<JsValue, JsValue> {
    use crate::modalities::logic::shacl::shapes_from_turtle;

    let shapes = shapes_from_turtle(turtle).map_err(|e| JsValue::from_str(&e))?;
    #[derive(Serialize)]
    struct ShapeOut {
        target_class: String,
        path: String,
        severity: String,
        constraint_count: usize,
        name: Option<String>,
    }
    #[derive(Serialize)]
    struct CompileOut {
        shape_count: usize,
        shapes: Vec<ShapeOut>,
        engine_version: &'static str,
    }
    let out: Vec<ShapeOut> = shapes
        .into_iter()
        .map(|s| ShapeOut {
            target_class: s.shape_class,
            path: s.property_path,
            severity: format!("{:?}", s.severity),
            constraint_count: s.constraints.len(),
            name: s.name,
        })
        .collect();
    Ok(serde_wasm_bindgen::to_value(&CompileOut {
        shape_count: out.len(),
        shapes: out,
        engine_version: "0.0.39",
    })?)
}

// ─── Query Engine & Ingestion Formats ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn execute_ntriples_query(query: &str, db_bytes: &[u8], max_results: usize) -> String {
    let mut program = [0u8; 1024];
    if crate::mini_parser::compile_ntriples_to_bytecode(query.as_bytes(), &mut program).is_err() {
        return r#"{"error": "Malformed query or program too large"}"#.to_string();
    }

    if db_bytes.len() % 48 != 0 {
        return r#"{"error": "db_bytes length must be a multiple of 48"}"#.to_string();
    }
    let quins = unsafe {
        std::slice::from_raw_parts(
            db_bytes.as_ptr() as *const crate::NQuin,
            db_bytes.len() / 48,
        )
    };

    let mut out = vec![crate::NQuin::default(); max_results];
    match crate::webizen_bytecode::execute_program_with_stats(&program, quins, &mut out, None) {
        Ok(stats) => {
            #[derive(Serialize)]
            struct MatchOut {
                s: String,
                p: String,
                o: String,
                c: String,
                m: String,
            }
            let mut matches = Vec::new();
            for i in 0..stats.match_count {
                matches.push(MatchOut {
                    s: out[i].subject.to_string(),
                    p: out[i].predicate.to_string(),
                    o: out[i].object.to_string(),
                    c: out[i].context.to_string(),
                    m: out[i].metadata.to_string(),
                });
            }
            #[derive(Serialize)]
            struct Res {
                matches: Vec<MatchOut>,
                vm_cycles: u64,
                direct_jump_ops: u64,
                lexicon_lookup_ops: u64,
            }

            serde_json::to_string(&Res {
                matches,
                vm_cycles: stats.vm_cycles,
                direct_jump_ops: stats.direct_jump_ops,
                lexicon_lookup_ops: stats.lexicon_lookup_ops,
            })
            .unwrap_or_else(|_| "{}".to_string())
        }
        Err(_) => r#"{"error": "VM execution error"}"#.to_string(),
    }
}

/// Bounded stride sample of packed 48-byte Quins. Browser graphs cannot mmap
/// `.q42` files; this is the WASM-safe equivalent of `mmap_sample_quins`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn sample_packed_quins_wasm(db_bytes: &[u8], max_quins: usize) -> Result<Vec<u8>, JsValue> {
    if db_bytes.len() % 48 != 0 {
        return Err(JsValue::from_str(
            "db_bytes length must be a multiple of 48",
        ));
    }
    let quins = unsafe {
        std::slice::from_raw_parts(
            db_bytes.as_ptr() as *const crate::NQuin,
            db_bytes.len() / 48,
        )
    };
    let sampled = crate::query_engine::sample_quins(quins, max_quins);
    let mut out = Vec::with_capacity(sampled.len() * 48);
    for quin in sampled {
        out.extend_from_slice(bytemuck::bytes_of(&quin));
    }
    Ok(out)
}

/// Compiles a query string (SPARQL WHERE-clause or N-Triples pattern) to a JSON
/// description of the Webizen VM bytecode program.  Useful for playground inspection
/// and benchmarking the compilation pipeline without supplying a database.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compile_query_to_json(query: &str) -> String {
    use crate::query_compiler::QueryCompiler;

    #[derive(Serialize)]
    struct InstructionOut {
        op: String,
    }
    #[derive(Serialize)]
    struct ProgramOut {
        source: &'static str,
        compiled_len: usize,
        instructions: Vec<InstructionOut>,
    }

    // Try SPARQL / JSON-LD / N3 path first (has WHERE { } block)
    let bytecode = QueryCompiler::compile_to_bytecode(query);
    if !bytecode.is_empty() {
        let instructions: Vec<InstructionOut> = bytecode
            .iter()
            .map(|op| InstructionOut {
                op: format!("{:?}", op),
            })
            .collect();
        let compiled_len = instructions.len();
        return serde_json::to_string(&ProgramOut {
            source: "query_compiler",
            compiled_len,
            instructions,
        })
        .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string());
    }

    // Fall back to N-Triples mini_parser pattern
    let mut program = [0u8; 1024];
    match crate::mini_parser::compile_ntriples_to_bytecode(query.as_bytes(), &mut program) {
        Ok(len) => {
            let instructions: Vec<InstructionOut> = program[..len]
                .iter()
                .enumerate()
                .map(|(i, &b)| InstructionOut {
                    op: format!("byte[{}]={:#04x}", i, b),
                })
                .collect();
            serde_json::to_string(&ProgramOut {
                source: "mini_parser",
                compiled_len: len,
                instructions,
            })
            .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string())
        }
        Err(e) => format!(r#"{{"error":"compilation failed: {:?}"}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_turtle_wasm(payload: &str) -> JsValue {
    use rio_api::parser::TriplesParser;
    #[derive(Serialize)]
    struct QOut {
        subject: String,
        predicate: String,
        object: String,
    }

    let cursor = std::io::Cursor::new(payload.as_bytes());
    let mut parser = rio_turtle::TurtleParser::new(cursor, None);
    let mut triples = Vec::new();
    let mut on_triple = |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
        triples.push(QOut {
            subject: t.subject.to_string(),
            predicate: t.predicate.to_string(),
            object: t.object.to_string(),
        });
        Ok(())
    };
    if parser.parse_all(&mut on_triple).is_err() {
        return JsValue::NULL; // Handle error appropriately
    }

    serde_wasm_bindgen::to_value(&triples).unwrap_or(JsValue::NULL)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_n3logic_wasm(payload: &str) -> JsValue {
    #[derive(Serialize)]
    struct QOut {
        subject: String,
        predicate: String,
        object: String,
    }

    let mut parser = crate::modalities::logic::n3_parser::N3Parser::new(payload);
    let mut triples = Vec::new();

    let on_n3_event = |event: crate::modalities::logic::n3_parser::N3Event| -> Result<(), crate::modalities::logic::n3_parser::N3ParserError> {
        if let crate::modalities::logic::n3_parser::N3Event::StaticTriple(triple) = event {
            let s = match triple.subject {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s)
                | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
            };
            let p = match triple.predicate {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s)
                | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
            };
            let o = match triple.object {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s)
                | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
            };
            triples.push(QOut {
                subject: s,
                predicate: p,
                object: o,
            });
        }
        Ok(())
    };

    if parser.parse_all(on_n3_event).is_err() {
        return JsValue::NULL;
    }

    serde_wasm_bindgen::to_value(&triples).unwrap_or(JsValue::NULL)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_cbor_ld_wasm(payload: &[u8]) -> JsValue {
    // Compact array codec is a Qualia vendor encoding until JSON-LD round-trip
    // is proven. Do not advertise as W3C CBOR-LD.
    const PROFILE: &str = "application/vnd.qualia.nquin-cbor";
    match crate::cbor_compiler::parse_cbor_ld_to_quin(payload) {
        Ok(q) => {
            #[derive(Serialize)]
            struct QOut {
                profile: &'static str,
                subject: String,
                predicate: String,
                object: String,
                context: String,
            }
            let out = QOut {
                profile: PROFILE,
                subject: q.subject.to_string(),
                predicate: q.predicate.to_string(),
                object: q.object.to_string(),
                context: q.context.to_string(),
            };
            serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
        }
        Err(_) => JsValue::NULL,
    }
}

/// Return the pinned Qualia JSON-LD 1.1 context + SHA-256 digest (UE-012).
///
/// Packages should embed `context` and record `digest` on receipts — do not
/// fetch remote `@context` URLs at admission time.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn jsonld_context_digest_wasm() -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::{
        context_digest_hex, pinned_context_bytes, QUALIA_JSONLD_CONTEXT_ID,
        QUALIA_JSONLD_CONTEXT_V1,
    };

    #[derive(Serialize)]
    struct ContextDigestOut {
        id: &'static str,
        digest_alg: &'static str,
        digest: String,
        context: &'static str,
        byte_len: usize,
        engine_version: &'static str,
    }

    Ok(serde_wasm_bindgen::to_value(&ContextDigestOut {
        id: QUALIA_JSONLD_CONTEXT_ID,
        digest_alg: "sha256",
        digest: context_digest_hex(),
        context: QUALIA_JSONLD_CONTEXT_V1,
        byte_len: pinned_context_bytes().len(),
        engine_version: "0.0.39",
    })?)
}

/// Package exposure receipt: context + shapes + vibe AST digests (UE-053).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn package_exposure_manifest_wasm(
    shapes_json: &str,
    vibe_program_cbor: Option<Vec<u8>>,
) -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::package_exposure_manifest;

    let vibe = vibe_program_cbor.unwrap_or_default();
    let m = package_exposure_manifest(shapes_json.as_bytes(), &vibe);

    #[derive(Serialize)]
    struct Out {
        profile: &'static str,
        engine_version: &'static str,
        context_id: &'static str,
        context_digest_sha256: String,
        shapes_digest_sha256: String,
        vibe_ast_tag: u64,
        vibe_ast_digest_sha256: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        profile: m.profile,
        engine_version: m.engine_version,
        context_id: m.context_id,
        context_digest_sha256: m.context_digest_sha256,
        shapes_digest_sha256: m.shapes_digest_sha256,
        vibe_ast_tag: m.vibe_ast_tag,
        vibe_ast_digest_sha256: m.vibe_ast_digest_sha256,
    })?)
}

/// RDFC-1.0 graph hash — **honest fail-closed** until a conforming implementation ships (UE-013).
///
/// Never returns a digest labelled as RDFC-1.0. Optionally includes a
/// `provisional_spo_sha256` under profile `qualia:provisional-spo-sha256-v1`
/// for scaffolding only.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn rdfc10_graph_hash_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::{
        provisional_spo_digest_hex, rdfc10_available, rdfc10_hash_hex,
        PROVISIONAL_SPO_DIGEST_PROFILE, RDFC10_PROFILE,
    };
    use crate::NQuin;

    #[derive(Deserialize)]
    struct Params {
        #[serde(default)]
        quins: Vec<[u64; 6]>,
        /// When true, attach a provisional SPO digest (not RDFC-1.0).
        #[serde(default)]
        include_provisional: bool,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid rdfc params: {e}")))?;

    let quins: Vec<NQuin> = p
        .quins
        .iter()
        .map(|arr| NQuin {
            subject: arr[0],
            predicate: arr[1],
            object: arr[2],
            context: arr[3],
            metadata: arr[4],
            parity: arr[5],
        })
        .collect();

    let rdfc_err = rdfc10_hash_hex(&quins).err().map(|e| e.to_string());

    #[derive(Serialize)]
    struct Provisional {
        profile: &'static str,
        digest_alg: &'static str,
        digest: String,
        warning: &'static str,
    }
    #[derive(Serialize)]
    struct RdfcOut {
        profile: &'static str,
        available: bool,
        rdfc10_digest: Option<String>,
        error: Option<String>,
        provisional: Option<Provisional>,
        engine_version: &'static str,
    }

    let provisional = if p.include_provisional {
        Some(Provisional {
            profile: PROVISIONAL_SPO_DIGEST_PROFILE,
            digest_alg: "sha256",
            digest: provisional_spo_digest_hex(&quins),
            warning: "NOT RDFC-1.0 — do not use for signatures or RDF dataset identity",
        })
    } else {
        None
    };

    Ok(serde_wasm_bindgen::to_value(&RdfcOut {
        profile: RDFC10_PROFILE,
        available: rdfc10_available(),
        rdfc10_digest: None,
        error: rdfc_err,
        provisional,
        engine_version: "0.0.39",
    })?)
}

/// Parse JSON-LD 1.1 text into packed quins (Civics primary semantic format).
///
/// Profile: `application/ld+json`. Context must be embedded/pinned by the caller;
/// this binding does not fetch remote contexts.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_jsonld_wasm(payload: &str) -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
    use std::io::Cursor;

    let mut collector = QuinCollector::new();
    let count = parse_rdf(
        RdfFormat::JsonLd,
        Cursor::new(payload.as_bytes()),
        0,
        &mut collector,
    )
    .map_err(|e| JsValue::from_str(&format!("JSON-LD parse error: {e}")))?;

    #[derive(Serialize)]
    struct JsonLdParseOut {
        profile: &'static str,
        content_type: &'static str,
        quin_count: u64,
        truncated: bool,
        /// Decimal strings — u64 exceeds JS safe integer range.
        quins: Vec<[String; 6]>,
        context_id: &'static str,
        context_digest: String,
        engine_version: &'static str,
    }

    let quins: Vec<[String; 6]> = collector
        .as_slice()
        .iter()
        .map(|q| {
            [
                q.subject.to_string(),
                q.predicate.to_string(),
                q.object.to_string(),
                q.context.to_string(),
                q.metadata.to_string(),
                q.parity.to_string(),
            ]
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&JsonLdParseOut {
        profile: "https://www.w3.org/ns/json-ld#expanded",
        content_type: "application/ld+json",
        quin_count: count,
        truncated: collector.truncated,
        quins,
        context_id: crate::sparql_library::rdf_formats::QUALIA_JSONLD_CONTEXT_ID,
        context_digest: crate::sparql_library::rdf_formats::context_digest_hex(),
        engine_version: "0.0.39",
    })?)
}

// ─── Forward Chaining ────────────────────────────────────────────────────────

/// Forward-chaining defeasible inference engine.
/// Input: `{ facts: ["bird", "penguin"], rules: [{ head: "flies", body: ["bird"], defeaters: ["penguin"] }, ...] }`
/// Output: `{ inferred: ["swims"] }`
#[cfg(target_arch = "wasm32")]
#[cfg(feature = "wasm-scientific")]
#[wasm_bindgen]
pub fn forward_chain_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solvers::symbolic_logic::{
        DefeasibleRule, Fact, ForwardChainingDefeasible, Literal, RuleType,
    };
    use crate::solvers::SolverConfig;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct RuleInput {
        head: String,
        body: Vec<String>,
        defeaters: Vec<String>,
    }
    #[derive(Deserialize)]
    struct FcInput {
        facts: Vec<String>,
        rules: Vec<RuleInput>,
    }
    let input: FcInput =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Build atom → u8 index map
    // Variable index 0 is reserved as antecedent terminator in ForwardChainingDefeasible.
    let mut atom_map: HashMap<String, u8> = HashMap::new();
    let mut next_idx: u8 = 1;
    let get_idx = |s: &str, map: &mut HashMap<String, u8>, nxt: &mut u8| -> u8 {
        if let Some(&i) = map.get(s) {
            return i;
        }
        let i = *nxt;
        map.insert(s.to_string(), i);
        *nxt = nxt.saturating_add(1);
        i
    };
    for f in &input.facts {
        get_idx(f, &mut atom_map, &mut next_idx);
    }
    for r in &input.rules {
        get_idx(&r.head, &mut atom_map, &mut next_idx);
        for b in &r.body {
            get_idx(b, &mut atom_map, &mut next_idx);
        }
        for d in &r.defeaters {
            get_idx(d, &mut atom_map, &mut next_idx);
        }
    }

    let mut solver = ForwardChainingDefeasible::new(SolverConfig::default());

    // Add initial facts
    for (fact_id, atom) in input.facts.iter().enumerate() {
        let var = *atom_map.get(atom.as_str()).unwrap_or(&0);
        solver
            .add_fact(Fact {
                id: (fact_id as u32) + 1,
                literal: Literal {
                    variable: var,
                    negated: false,
                },
                supporting_rules: [0; 3],
                defeated: false,
                confidence: 1.0,
            })
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    }

    // Add rules and defeaters
    let base_id = input.facts.len() as u32 + 1;
    for (rule_id, r) in input.rules.iter().enumerate() {
        let head_var = *atom_map.get(r.head.as_str()).unwrap_or(&0);
        let mut antecedents = [Literal::default(); 5];
        for (i, b) in r.body.iter().take(5).enumerate() {
            antecedents[i] = Literal {
                variable: *atom_map.get(b.as_str()).unwrap_or(&0),
                negated: false,
            };
        }

        // Main defeasible rule: head fires when all body atoms hold
        let main_rule = DefeasibleRule {
            id: base_id + (rule_id as u32) * 2,
            rule_type: if r.defeaters.is_empty() {
                RuleType::Strict
            } else {
                RuleType::Defeasible
            },
            antecedents,
            consequent: Literal {
                variable: head_var,
                negated: false,
            },
            priority: 500,
            active: true,
            fire_count: 0,
        };
        solver
            .add_rule(main_rule)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // Defeater rules: for each defeater atom, add a Defeater rule that cancels the head
        for (d_i, d) in r.defeaters.iter().enumerate() {
            let d_var = *atom_map.get(d.as_str()).unwrap_or(&0);
            let mut d_antecedents = [Literal::default(); 5];
            d_antecedents[0] = Literal {
                variable: d_var,
                negated: false,
            };
            let defeater_rule = DefeasibleRule {
                id: base_id + (rule_id as u32) * 2 + 1 + d_i as u32,
                rule_type: RuleType::Defeater,
                antecedents: d_antecedents,
                consequent: Literal {
                    variable: head_var,
                    negated: true,
                },
                priority: 600,
                active: true,
                fire_count: 0,
            };
            solver
                .add_rule(defeater_rule)
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        }
    }

    solver
        .infer()
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    // Build reverse map to recover atom names from variable indices
    let rev_map: HashMap<u8, &str> = atom_map.iter().map(|(k, &v)| (v, k.as_str())).collect();
    let initial_fact_set: std::collections::HashSet<String> = input.facts.iter().cloned().collect();

    let mut inferred = Vec::new();
    for fact in &solver.facts {
        if fact.id == 0 || fact.defeated {
            continue;
        }
        if let Some(&name) = rev_map.get(&fact.literal.variable) {
            if !fact.literal.negated && !initial_fact_set.contains(name) {
                inferred.push(name.to_string());
            }
        }
    }
    inferred.sort();
    let inferred_count = inferred.len();

    let mut facts_sorted = input.facts.clone();
    facts_sorted.sort();
    let receipt_material = format!(
        "forward_chain:facts={}|inferred={}|rules={}",
        facts_sorted.join(","),
        inferred.join(","),
        input.rules.len()
    );
    let receipt_hash = format!("{:016x}", crate::q_hash(&receipt_material));

    #[derive(Serialize)]
    struct FcReceipt {
        engine_version: &'static str,
        algorithm: &'static str,
        sample_size: usize,
        seed: Option<u64>,
        converged: bool,
        tolerances: Option<f64>,
        warnings: Vec<&'static str>,
        receipt_hash: String,
    }
    #[derive(Serialize)]
    struct FcOut {
        inferred: Vec<String>,
        fact_count: usize,
        rule_count: usize,
        inferred_count: usize,
        receipt: FcReceipt,
    }
    Ok(serde_wasm_bindgen::to_value(&FcOut {
        inferred,
        fact_count: input.facts.len(),
        rule_count: input.rules.len(),
        inferred_count,
        receipt: FcReceipt {
            engine_version: "0.0.39",
            algorithm: "ForwardChainingDefeasible",
            sample_size: input.facts.len() + input.rules.len(),
            seed: None,
            converged: true,
            tolerances: None,
            warnings: vec!["atom indices are local to this invoke; not an N3 document hash"],
            receipt_hash,
        },
    })?)
}

// ─── Engine metadata ─────────────────────────────────────────────────────────

// --- Data Format: RDF Serializer (Solid / LDP content types) ----------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct RdfSerializeParams {
    pub quins: Vec<[u64; 6]>,
    /// Format id (`turtle`, `jsonld`, `n3`, …) **or** Solid MIME type.
    pub format: String,
    /// When true and format is JSON-LD, emit compact document with pinned `@context`.
    #[serde(default)]
    pub compact: bool,
}

#[cfg(target_arch = "wasm32")]
fn quins_from_wire(rows: &[[u64; 6]]) -> Vec<crate::NQuin> {
    rows.iter()
        .map(|arr| crate::NQuin {
            subject: arr[0],
            predicate: arr[1],
            object: arr[2],
            context: arr[3],
            metadata: arr[4],
            parity: arr[5],
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn write_rdf_document(
    format: crate::sparql_library::rdf_formats::RdfFormat,
    compact_jsonld: bool,
    quins: &[crate::NQuin],
    out: &mut Vec<u8>,
) -> Result<&'static str, String> {
    use crate::sparql_library::rdf_formats::RdfFormat;
    use crate::sparql_library::serialisers::rdf_serializers::{
        serialize_to_jsonld, serialize_to_jsonld_compact, serialize_to_n3, serialize_to_nquads,
        serialize_to_ntriples, serialize_to_trig, serialize_to_turtle,
    };
    match format {
        RdfFormat::NTriples => serialize_to_ntriples(out, quins)?,
        RdfFormat::Turtle => serialize_to_turtle(out, quins)?,
        RdfFormat::NQuads => serialize_to_nquads(out, quins)?,
        RdfFormat::TriG => serialize_to_trig(out, quins)?,
        RdfFormat::N3 => serialize_to_n3(out, quins)?,
        RdfFormat::JsonLd => {
            if compact_jsonld {
                serialize_to_jsonld_compact(out, quins)?;
            } else {
                serialize_to_jsonld(out, quins)?;
            }
        }
        RdfFormat::CborLd => {
            return Err(
                "use serialize_to_cborld / parse_cbor_ld_wasm for application/vnd.qualia.nquin-cbor"
                    .into(),
            );
        }
    }
    Ok(format.solid_content_type())
}

/// Serialize quins to RDF. `format` accepts Qualia ids (`turtle`, `jsonld`, `n3`)
/// or Solid MIME types (`text/turtle`, `application/ld+json`, `text/n3`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn serialize_rdf_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::RdfFormat;

    let p: RdfSerializeParams = serde_wasm_bindgen::from_value(val)?;
    let format = RdfFormat::from_media_type(&p.format)
        .or_else(|| RdfFormat::from_str(&p.format))
        .ok_or_else(|| JsValue::from_str("Invalid RDF format / Content-Type"))?;
    let quins = quins_from_wire(&p.quins);
    let mut rdf_output = Vec::new();
    let content_type = write_rdf_document(format, p.compact, &quins, &mut rdf_output)
        .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct SerializeResult {
        rdf_data: String,
        content_type: &'static str,
        format: &'static str,
        compact: bool,
        engine_version: &'static str,
    }

    Ok(serde_wasm_bindgen::to_value(&SerializeResult {
        rdf_data: String::from_utf8(rdf_output).map_err(|e| JsValue::from_str(&e.to_string()))?,
        content_type,
        format: format.as_str(),
        compact: p.compact && matches!(format, RdfFormat::JsonLd),
        engine_version: "0.0.39",
    })?)
}

/// Parse an RDF document by Solid/LDP Content-Type (or Qualia format id).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_rdf_document_wasm(content_type: &str, payload: &str) -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
    use std::io::Cursor;

    let format = RdfFormat::from_media_type(content_type)
        .or_else(|| RdfFormat::from_str(content_type))
        .ok_or_else(|| JsValue::from_str("Unsupported RDF Content-Type / format"))?;
    if matches!(format, RdfFormat::CborLd) {
        return Err(JsValue::from_str(
            "CBOR documents are binary — use parse_cbor_ld_wasm",
        ));
    }

    let mut collector = QuinCollector::new();
    let count = parse_rdf(format, Cursor::new(payload.as_bytes()), 0, &mut collector)
        .map_err(|e| JsValue::from_str(&format!("RDF parse error: {e}")))?;

    let quins: Vec<[String; 6]> = collector
        .as_slice()
        .iter()
        .map(|q| {
            [
                q.subject.to_string(),
                q.predicate.to_string(),
                q.object.to_string(),
                q.context.to_string(),
                q.metadata.to_string(),
                q.parity.to_string(),
            ]
        })
        .collect();

    #[derive(Serialize)]
    struct Out {
        content_type: &'static str,
        format: &'static str,
        quin_count: u64,
        truncated: bool,
        /// Decimal strings — u64 fields exceed JS Number.MAX_SAFE_INTEGER.
        quins: Vec<[String; 6]>,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        content_type: format.solid_content_type(),
        format: format.as_str(),
        quin_count: count,
        truncated: collector.truncated,
        quins,
        engine_version: "0.0.39",
    })?)
}

/// Negotiate Solid `Accept` → preferred RDF Content-Type.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn solid_negotiate_accept_wasm(accept: &str) -> Result<JsValue, JsValue> {
    use crate::sparql_library::rdf_formats::{negotiate_solid_accept, RdfFormat, SOLID_RDF_MEDIA};

    let content_type = negotiate_solid_accept(accept);
    let format = RdfFormat::from_media_type(content_type).unwrap_or(RdfFormat::Turtle);

    #[derive(Serialize)]
    struct Row {
        content_type: &'static str,
        format: &'static str,
        solid_primary: bool,
    }
    #[derive(Serialize)]
    struct Out {
        content_type: &'static str,
        format: &'static str,
        supported: Vec<Row>,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        content_type,
        format: format.as_str(),
        supported: SOLID_RDF_MEDIA
            .iter()
            .map(|m| Row {
                content_type: m.content_type,
                format: m.format.as_str(),
                solid_primary: m.solid_primary,
            })
            .collect(),
        engine_version: "0.0.39",
    })?)
}

/// Pre-flight sanctuary notice for leave UI (desktop / Databox / PWA).
/// Advertised on `wasm-webcivics` as part of `solid-leave-migrate`.
#[cfg(all(target_arch = "wasm32", feature = "wasm-webcivics"))]
#[wasm_bindgen]
pub fn plan_sanctuary_migration_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solid_ldp::plan_sanctuary_migration_notice;

    #[derive(Deserialize)]
    struct Params {
        quins: Vec<[u64; 6]>,
    }
    let p: Params = serde_wasm_bindgen::from_value(val)?;
    let quins = quins_from_wire(&p.quins);
    let n = plan_sanctuary_migration_notice(&quins);

    #[derive(Serialize)]
    struct Out {
        sanctuary_quin_count: usize,
        classified_quin_count: usize,
        requires_choice: bool,
        title: String,
        body: String,
        choice_omit_label: String,
        choice_reclassify_label: String,
        classified_note: String,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        sanctuary_quin_count: n.sanctuary_quin_count,
        classified_quin_count: n.classified_quin_count,
        requires_choice: n.requires_choice,
        title: n.title,
        body: n.body,
        choice_omit_label: n.choice_omit_label,
        choice_reclassify_label: n.choice_reclassify_label,
        classified_note: n.classified_note,
        engine_version: "0.0.39",
    })?)
}

/// Build a Solid leave/migrate bundle (Turtle / N-Quads / JSON-LD / ACL / manifest).
///
/// Requires `sanctuary_choice`: `omit-sanctuary` | `reclassify-for-solid` when
/// sanctuary data is present (`Unset` fails closed). Classified never egresses.
/// Advertised on the `wasm-webcivics` profile as `solid-leave-migrate`.
#[cfg(all(target_arch = "wasm32", feature = "wasm-webcivics"))]
#[wasm_bindgen]
pub fn export_solid_migration_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solid_ldp::{
        build_migration_bundle, SanctuaryExportChoice, SolidBundleFormats, SolidMigrationOptions,
    };
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct Params {
        quins: Vec<[u64; 6]>,
        /// `omit-sanctuary` | `reclassify-for-solid` | `unset` (fails if sanctuary present).
        #[serde(default)]
        sanctuary_choice: Option<String>,
        /// Legacy alias for `reclassify-for-solid` when sanctuary_choice omitted.
        #[serde(default)]
        include_restricted: bool,
        #[serde(default)]
        grant_public_read: bool,
        #[serde(default)]
        owner_webid: Option<String>,
        #[serde(default)]
        lexicon: Option<HashMap<String, String>>,
    }

    let p: Params = serde_wasm_bindgen::from_value(val)?;
    let quins = quins_from_wire(&p.quins);
    let lex_map: Option<HashMap<u64, String>> = p.lexicon.map(|m| {
        m.into_iter()
            .filter_map(|(k, v)| k.parse::<u64>().ok().map(|h| (h, v)))
            .collect()
    });
    let choice = if let Some(s) = p.sanctuary_choice.as_deref() {
        SanctuaryExportChoice::from_str(s)
            .ok_or_else(|| JsValue::from_str("invalid sanctuary_choice"))?
    } else if p.include_restricted {
        SanctuaryExportChoice::ReclassifyForSolid
    } else {
        SanctuaryExportChoice::Unset
    };
    let opts = SolidMigrationOptions {
        sanctuary_choice: choice,
        formats: SolidBundleFormats::default(),
        owner_webid: p.owner_webid,
        grant_public_read: p.grant_public_read,
    };
    let bundle = build_migration_bundle(&quins, lex_map.as_ref(), &opts)
        .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct StatsOut {
        scanned: usize,
        exported: usize,
        redacted_classified: usize,
        redacted_restricted: usize,
        skipped_parity: usize,
    }
    #[derive(Serialize)]
    struct Out {
        turtle: Option<String>,
        nquads: Option<String>,
        jsonld: Option<String>,
        acl: Option<String>,
        manifest_jsonld: String,
        stats: StatsOut,
        sanctuary_choice: &'static str,
        outcome_summary: String,
        engine_version: &'static str,
    }

    Ok(serde_wasm_bindgen::to_value(&Out {
        turtle: bundle.turtle,
        nquads: bundle.nquads,
        jsonld: bundle.jsonld,
        acl: bundle.acl,
        manifest_jsonld: bundle.manifest_jsonld,
        stats: StatsOut {
            scanned: bundle.stats.scanned,
            exported: bundle.stats.exported,
            redacted_classified: bundle.stats.redacted_classified,
            redacted_restricted: bundle.stats.redacted_restricted,
            skipped_parity: bundle.stats.skipped_parity,
        },
        sanctuary_choice: bundle.sanctuary_choice.as_str(),
        outcome_summary: bundle.outcome_summary,
        engine_version: bundle.engine_version,
    })?)
}

/// Solid RDF Sources → Qualia `webcivics.vault-backup.v1` JSON (return path).
#[cfg(all(target_arch = "wasm32", feature = "wasm-webcivics"))]
#[wasm_bindgen]
pub fn import_solid_to_qualia_backup_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solid_ldp::solid_rdf_to_qualia_backup_json;

    #[derive(Deserialize)]
    struct Resource {
        name: String,
        content_type: String,
        body: String,
    }
    #[derive(Deserialize)]
    struct Params {
        resources: Vec<Resource>,
    }

    let p: Params = serde_wasm_bindgen::from_value(val)?;
    let resources: Vec<(String, String, String)> = p
        .resources
        .into_iter()
        .map(|r| (r.name, r.content_type, r.body))
        .collect();
    let package = solid_rdf_to_qualia_backup_json(&resources).map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct Out {
        backup_json: String,
        engine_version: &'static str,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        backup_json: package,
        engine_version: "0.0.39",
    })?)
}
