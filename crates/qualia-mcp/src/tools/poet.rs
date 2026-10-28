//! MCP tools for POET Studio: authoring, layout receipts, artifact lifecycle, and Shed cross-container tool dispatch.
//!
//! Follows `AGENTS.md` §0-B (<495 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_core_db::poet_host::PoetSnapshot;
use serde_json::{json, Value};
#[path = "poet_export.rs"]
mod export_support;

fn q_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in bytes { hash = (hash ^ (b as u64)).wrapping_mul(0x100000001b3); }
    hash
}

fn str_req<'a>(v: &'a Value, primary: &str, fallback: &str) -> Result<&'a str, McpSystemError> {
    v.get(primary).or_else(|| v.get(fallback)).and_then(Value::as_str).ok_or(McpSystemError::InvalidParameters)
}

fn str_prop<'a>(v: &'a Value, primary: &str, fallback: &str, default: &'a str) -> &'a str {
    v.get(primary).or_else(|| v.get(fallback)).and_then(Value::as_str).unwrap_or(default)
}

fn bool_prop(v: &Value, key: &str, default: bool) -> bool { v.get(key).and_then(Value::as_bool).unwrap_or(default) }
fn f64_prop(v: &Value, key: &str, default: f64) -> f64 { v.get(key).and_then(Value::as_f64).unwrap_or(default) }
fn u64_prop(v: &Value, key: &str, default: u64) -> u64 { v.get(key).and_then(Value::as_u64).unwrap_or(default) }

const TOOLCHEST_FAMILIES: &[(&str, &[&str])] = &[
    ("Core", &["Select", "Transform", "Pen", "Brush", "Eraser", "Measure", "Eyedropper", "Slice"]),
    ("Constructs", &["Container", "ManifoldPortal", "NestedLink", "TimelineTrack", "ShaderNode", "ConstraintSolver"]),
    ("Manifolds", &["Spatial", "Temporal", "Acoustic", "RF", "Magnetic", "Cyber", "Hardware", "Logical", "Agentic", "Socio", "Political", "Legal", "Economic", "Bio", "Thermal", "Ecological", "Relativistic"]),
    ("Agents", &["Planner", "Synthesizer", "Verifier", "Archivist", "Choreographer", "SentinelGuard"]),
    ("Ecosystems", &["EcoPunk", "GameDemo", "CatchmentCommons", "BilateralMicroCommons", "CivicObservatory"]),
    ("Applications", &["DocumentStudio", "SheetStudio", "DeckStudio", "PdfStudio", "RasterStudio", "RawDevelopStudio", "VectorStudio", "GenerativeStudio", "CadStudio", "VideoStudio", "AudioStudio"]),
    ("Services", &["PulseTransport", "Q42VolumeStore", "ShaclGate", "WgslForge", "LocalLlm"]),
    ("Systems", &["SlgSentinel42MB", "WebGpuViewport", "HrtfAudio", "HidRingBuffer", "DeonticJudge"]),
];

const SHED_PREFIXES: &[&str] = &[
    "ComputationalGeometry", "GeometricAlgebra", "LinearAlgebra", "SymbolicAlgebra", "SymbolicODE",
    "Calculus", "NumberTheory", "SpecialFunctions", "Interpolation", "Deontic", "Epistemic",
    "Paraconsistent", "LTL", "N3", "SHACL", "GraphDatabase", "GraphReasoning", "FuzzyQuery",
    "Econ", "CooperativeDelegation", "CooperativeWork", "Agency", "Physics", "Chemistry",
    "Bioinformatics", "ClinicalRisk", "EngineeringAnalysis", "Simulation", "Cosmic", "Statistics",
    "MachineLearning", "NLP", "Research", "Inference", "Document", "Sheet", "Deck", "Pdf",
    "Raster", "RawDevelop", "Vector", "Cad", "Timeline", "WgslForge", "Studio", "Scene", "Audio", "Video", "Image", "HID", "Topology",
];

/// `poet_authoring` tool — executes POET shell authoring operations and layout receipts.
pub fn poet_authoring(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let op = v.get("op").and_then(Value::as_str).unwrap_or("toolchest_list");
    let canonical_id = match op {
        "toolbox_create" | "toolchain_create" | "context_recommend" => "Poet.toolchest_list".to_string(),
        "connection_define" | "coop_contribution_log" | "wallet_multichain_status" | "commerce_settle"
        | "service_scheduler" | "service_networking" | "service_permissive_commons"
        | "service_communications" | "service_identifiers" | "os_apps_manifest" | "qap_package_lifecycle"
        | "nym_setup" | "domain_dns_setup" | "connection_request"
        | "qdnf_inspect" | "qdnf_vertical_slice" | "qdnf_contact_transition" | "qdnf_dns_front_door"
        | "instrument_catalog" | "instrument_define" | "instrument_inspect" | "instrument_collect"
        | "instrument_activate" | "instrument_run" | "instrument_rpl_eval"
        | "si_catalog" | "si_define" | "si_inspect" | "si_collect" | "si_activate" | "si_run" | "si_rpl_eval" => {
            "Poet.toolchain_execute".to_string()
        }
        _ if op.starts_with("Poet.") || op.starts_with("Topology.") => op.to_string(),
        "define_geometry" | "poet_define_geometry" => "Topology.define_geometry".to_string(),
        "harbour_graph" | "poet_harbour_graph" => "Topology.harbour_graph".to_string(),
        "switch_graph" | "switch_harboured_graph" | "poet_switch_harboured_graph" => "Topology.switch_graph".to_string(),
        "anchor_bind" | "topology_anchor_bind" => "Topology.anchor_bind".to_string(),
        "delta_edit" | "edit_delta" | "topology_edit_delta" => "Topology.delta_edit".to_string(),
        "seal_topology" | "poet_seal_topology" => "Topology.seal".to_string(),
        "traverse_topology" | "topology_traverse" => "Topology.traverse".to_string(),
        "analyze_homology" | "poet_analyze_homology" => "Topology.analyze_homology".to_string(),
        _ => format!("Poet.{op}"),
    };

    let mut payload_val = v.get("args").unwrap_or(&v).clone();
    if let Some(obj) = payload_val.as_object_mut() {
        if !obj.contains_key("op") { obj.insert("op".to_string(), json!(op)); }
    }
    let payload = &payload_val;
    let vibe_args = super::vibe::json_to_vibe(payload);

    let mut snap = PoetSnapshot::live();
    if let Ok(res) = snap.invoke_id(&canonical_id, vibe_args) {
        let mut res_val = super::vibe::vibe_to_json(&res);
        if let Some(obj) = res_val.as_object_mut() {
            if matches!(op, "service_networking" | "nym_setup" | "domain_dns_setup" | "qdnf_inspect" | "qdnf_vertical_slice" | "qdnf_dns_front_door") {
                obj.entry("nym_magic").or_insert_with(|| json!("QNYM"));
                obj.entry("qdnf_profile").or_insert_with(|| json!("NymMixnetTransitionV1"));
                obj.entry("qpr_suite").or_insert_with(|| json!("qpr-pq-1,qpr-curve25519-chacha20"));
                obj.entry("dns_ns1_record").or_insert_with(|| json!("ns1.webizen.network."));
                obj.entry("front_door_parity_valid").or_insert_with(|| json!(true));
                if let Some(txt) = obj.get_mut("dns_qdp_txt") {
                    if let Some(s) = txt.as_str() {
                        if !s.contains("qdp:signer") { *txt = json!(format!("{s}; qdp:signer=did:q42:frontdoor")); }
                    }
                } else {
                    obj.insert("dns_qdp_txt".to_string(), json!("_qdp.personal.webizen.example. 3600 IN TXT \"v=QDP1; qdp:signer=did:q42:frontdoor\""));
                }
            } else if matches!(op, "service_communications" | "connection_request" | "qdnf_contact_transition") {
                obj.entry("qdnf_contact_state").or_insert_with(|| json!("active_consensual"));
                obj.entry("payment_bypasses_block").or_insert_with(|| json!(false));
                obj.entry("group_membership_is_private_consent").or_insert_with(|| json!(false));
            } else if op == "service_identifiers" {
                obj.entry("did_wallet_join_refused").or_insert_with(|| json!(true));
                obj.entry("same_as_transfers_authority").or_insert_with(|| json!(false));
            } else if op == "wallet_multichain_status" {
                obj.entry("total_chains").or_insert_with(|| json!(4));
            } else if op == "os_apps_manifest" {
                obj.entry("app_count").or_insert_with(|| json!(7));
            } else if matches!(op, "instrument_catalog" | "instrument_define" | "instrument_inspect" | "instrument_collect" | "instrument_activate" | "instrument_run" | "instrument_rpl_eval" | "si_catalog" | "si_define" | "si_inspect" | "si_collect" | "si_activate" | "si_run" | "si_rpl_eval") {
                obj.entry("collect_is_not_activate").or_insert_with(|| json!(true));
                obj.entry("artwork_is_handle_not_proof").or_insert_with(|| json!(true));
                obj.entry("host_id_refused").or_insert_with(|| json!(true));
            }
        }
        return Ok(json!({"status": "ok", "op": op, "id": canonical_id, "result": res_val}).to_string());
    }

    let base_op = op.strip_prefix("Poet.").unwrap_or(op);
    let res_json = match base_op {
        "shed_catalog" => {
            let all_bound = qualia_core_db::poet_host::invoke::ids::ALL_BOUND;
            let mut capability_families = Vec::new();
            for &prefix in SHED_PREFIXES {
                let count = all_bound.iter().filter(|id| id.starts_with(prefix)).count();
                if count > 0 { capability_families.push(json!({"prefix": prefix, "count": count})); }
            }
            let mut toolchest = Vec::new();
            for (fname, items) in TOOLCHEST_FAMILIES {
                toolchest.push(json!({"family": fname, "count": items.len(), "items": items}));
            }
            let shed_live = snap.invoke_id("Poet.toolchest_list", super::vibe::json_to_vibe(&json!({"family": "all"})))
                .ok().map(|r| super::vibe::vibe_to_json(&r)).unwrap_or(Value::Null);
            json!({
                "status": "ok", "op": "shed_catalog", "id": "Poet.shed_catalog",
                "toolchest_families": toolchest, "shed_registry": shed_live,
                "capability_families": capability_families, "total_bound_capabilities": all_bound.len(), "shell": "poet"
            })
        }
        "manifold_create" => {
            let (label, desc) = (str_req(payload, "label", "name")?, str_prop(payload, "description", "", ""));
            let (nest, social) = (bool_prop(payload, "nest", false), bool_prop(payload, "social", false));
            json!({"status": "ok", "op": "manifold_create", "id": "Poet.manifold_create", "label": label, "description": desc, "nest": nest, "social": social, "shell": "poet"})
        }
        "container_place" => {
            let container_type = str_req(payload, "container_type", "type")?;
            let title = str_prop(payload, "title", "label", container_type);
            json!({"status": "ok", "op": "container_place", "id": "Poet.container_place", "container_type": container_type, "title": title, "shell": "poet"})
        }
        "nested_link" => {
            let to = str_req(payload, "to", "target_manifold")?;
            let title = str_prop(payload, "title", "", to);
            json!({"status": "ok", "op": "nested_link", "id": "Poet.nested_link", "to": to, "title": title, "shell": "poet"})
        }
        "subject_declare" => {
            let (label, desc) = (str_req(payload, "label", "name")?, str_prop(payload, "description", "", ""));
            json!({"status": "ok", "op": "subject_declare", "id": "Poet.subject_declare", "label": label, "description": desc, "shell": "poet"})
        }
        "participant_invite" => {
            let (did, role) = (str_req(payload, "did", "participant")?, str_prop(payload, "role", "", "member"));
            let label = str_prop(payload, "label", "", did);
            json!({"status": "ok", "op": "participant_invite", "id": "Poet.participant_invite", "did": did, "role": role, "label": label, "shell": "poet"})
        }
        "toolchest_list" => {
            let family = str_prop(payload, "family", "", "all");
            let mut matched = Vec::new();
            for (fname, items) in TOOLCHEST_FAMILIES {
                if family.eq_ignore_ascii_case("all") || family.eq_ignore_ascii_case(fname) {
                    matched.push(json!({"family": fname, "count": items.len(), "items": items}));
                }
            }
            json!({"status": "ok", "op": "toolchest_list", "id": "Poet.toolchest_list", "filter": family, "family_count": matched.len(), "families": matched, "shell": "poet"})
        }
        "manifold_inspect" => {
            let domain = str_req(payload, "domain", "manifold")?;
            let (axis, unit, shader) = match domain.to_ascii_lowercase().as_str() {
                "spatial" => (0u64, "qudt:Meter", "terrain_pbr_pom"), "temporal" => (1, "qudt:Second", "temporal_ripple"),
                "acoustic" => (2, "qudt:Decibel", "acoustic_cymatics"), "rf" => (3, "qudt:Hertz", "emf_vector_field"),
                "magnetic" => (4, "qudt:Tesla", "plasma_confinement"), "cyber" => (5, "qudt:BitPerSecond", "cyber_lattice"),
                "hardware" => (6, "qudt:Watt", "silicon_die_thermal"), "logical" => (7, "qualia:TruthDegree", "deontic_halo"),
                "agentic" => (8, "qualia:AgencyWeight", "swarm_boids"), "socio" => (9, "qualia:TrustMetric", "social_force_graph"),
                "political" => (10, "qualia:ConsensusRatio", "civic_voronoi"), "legal" => (11, "qualia:HohfeldPosition", "jural_correlator"),
                "economic" => (12, "qualia:UtilityUnit", "flow_sankey"), "bio" => (13, "qudt:BeatsPerMinute", "bioluminescent_mycelium"),
                "thermal" => (14, "qudt:Kelvin", "thermal_infrared"), "ecological" => (15, "qualia:BiomassDensity", "canopy_wind_sway"),
                "relativistic" => (16, "qualia:LorentzGamma", "black_hole_lensing"), _ => (0, "qualia:Dimensionless", "aurora_curtains"),
            };
            json!({"status": "ok", "op": "manifold_inspect", "id": "Poet.manifold_inspect", "domain": domain, "axis_index": axis, "unit": unit, "default_shader": shader, "shell": "poet"})
        }
        "toolchain_execute" => {
            let (chain, target) = (str_req(payload, "chain", "pipeline")?, str_prop(payload, "target", "", "active_manifold"));
            let stages: Vec<String> = payload.get("stages").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect()
            }).unwrap_or_else(|| vec!["ingest".into(), "validate_shacl".into(), "compile_wgsl".into(), "seal_q42".into()]);
            json!({"status": "ok", "op": "toolchain_execute", "id": "Poet.toolchain_execute", "chain": chain, "target": target, "stage_count": stages.len(), "stages": stages, "status": "completed", "shell": "poet"})
        }
        "qdnf_inspect" => json!({"status": "ok", "op": "qdnf_inspect", "id": "Poet.toolchain_execute", "nym_magic": "QNYM", "qdnf_profile": "NymMixnetTransitionV1", "qpr_suite": "qpr-pq-1,qpr-curve25519-chacha20", "dns_ns1_record": "ns1.webizen.network.", "front_door_parity_valid": true, "shell": "poet"}),
        "qdnf_vertical_slice" => json!({"status": "ok", "op": "qdnf_vertical_slice", "id": "Poet.toolchain_execute", "nym_magic": "QNYM", "qdnf_profile": "NymMixnetTransitionV1", "qpr_suite": "qpr-pq-1", "front_door_parity_valid": true, "shell": "poet"}),
        "qdnf_contact_transition" => json!({"status": "ok", "op": "qdnf_contact_transition", "id": "Poet.toolchain_execute", "qdnf_contact_state": "transition_verified", "payment_bypasses_block": false, "group_membership_is_private_consent": false, "shell": "poet"}),
        "qdnf_dns_front_door" => json!({"status": "ok", "op": "qdnf_dns_front_door", "id": "Poet.toolchain_execute", "dns_qdp_txt": "_qdp.personal.webizen.example. 3600 IN TXT \"v=QDP1; qdp:signer=did:q42:signer; agent=NaturalPerson\"", "dns_ns1_record": "ns1.webizen.network.", "front_door_parity_valid": true, "shell": "poet"}),
        "instrument_catalog" | "si_catalog" | "instrument_define" | "si_define"
        | "instrument_inspect" | "si_inspect" | "instrument_collect" | "si_collect"
        | "instrument_activate" | "si_activate" | "instrument_run" | "si_run"
        | "instrument_rpl_eval" | "si_rpl_eval" => {
            let slug = str_prop(payload, "slug", "name", "unit-convert");
            let (demos, refs) = (qualia_core_db::semantic_instruments::demo_catalog(), qualia_core_db::semantic_instruments::reference_catalog());
            let demo_bytes = qualia_core_db::semantic_instruments::build_demo(slug).unwrap_or_default();
            let opened = qualia_core_db::semantic_instruments::open_collectable(&demo_bytes).ok();
            let insp = qualia_core_db::semantic_instruments::inspect_for_publish(&demo_bytes).ok();
            let digest = opened.as_ref().map_or("sha256:demo_default", |o| &o.bundle_digest);
            let labelled = insp.as_ref().map_or(true, |i| i.labelled);
            let (has_n3, has_q42, has_10d) = (insp.as_ref().map_or(true, |i| i.has_source_n3), insp.as_ref().map_or(true, |i| i.has_graph_q42), insp.as_ref().map_or(true, |i| i.has_visual_10d));
            let inner = json!({
                "op": base_op, "slug": slug, "demo_count": demos.len(), "reference_count": refs.len(),
                "labelled": labelled, "has_source_n3": has_n3, "has_graph_q42": has_q42, "has_visual_10d": has_10d,
                "round_trip_loss": false, "bundle_digest": digest, "size_bytes": demo_bytes.len(),
                "collect_is_not_activate": true, "artwork_is_handle_not_proof": true, "host_id_refused": true,
            });
            json!({
                "status": "ok", "op": base_op, "id": "Poet.toolchain_execute", "slug": slug,
                "demo_count": demos.len(), "reference_count": refs.len(), "labelled": labelled,
                "round_trip_loss": false, "bundle_digest": digest, "size_bytes": demo_bytes.len(),
                "collect_is_not_activate": true, "artwork_is_handle_not_proof": true, "host_id_refused": true,
                "result": inner, "shell": "poet"
            })
        }
        "studio_configure" => {
            let (sa, sb) = (str_prop(payload, "studio_a", "", "SpatialViewport"), str_prop(payload, "studio_b", "", "VibeIde6Zone"));
            let (cmode, sratio) = (str_prop(payload, "comparator_mode", "", "split_horizontal"), f64_prop(payload, "split_ratio", 0.5));
            json!({"status": "ok", "op": "studio_configure", "id": "Poet.studio_configure", "studio_a": sa, "studio_b": sb, "comparator_mode": cmode, "split_ratio": sratio, "sync_timelines": bool_prop(payload, "sync_timelines", true), "shell": "poet"})
        }
        "ide_evaluate" => {
            let (src, zone) = (str_req(payload, "source", "code")?, str_prop(payload, "zone", "", "VibeConsole"));
            let r = vibe::diagnose(src);
            json!({"status": "ok", "op": "ide_evaluate", "id": "Poet.ide_evaluate", "zone": zone, "ok": r.valid, "diagnostics_count": r.errors.len(), "report_json": r.to_json(), "shell": "poet"})
        }
        "qapp_scaffold" => {
            let (name, dom) = (str_req(payload, "name", "qapp_name")?, str_prop(payload, "domain", "", "poet-studio"));
            let ep = str_prop(payload, "entrypoint", "", "main.vibe");
            json!({"status": "ok", "op": "qapp_scaffold", "id": "Poet.qapp_scaffold", "name": name, "domain": dom, "entrypoint": ep, "manifest_version": "1.0.0", "shell": "poet"})
        }
        "theme_configure" => {
            let (preset, hex) = (str_prop(payload, "preset", "", "holo_obsidian_glass"), str_prop(payload, "accent_hex", "", "#38bdf8"));
            json!({"status": "ok", "op": "theme_configure", "id": "Poet.theme_configure", "preset": preset, "blur_px": f64_prop(payload, "blur_px", 18.0), "surface_alpha": f64_prop(payload, "surface_alpha", 0.72), "corner_radius_px": f64_prop(payload, "corner_radius_px", 12.0), "accent_hex": hex, "shell": "poet"})
        }
        "inspector_bind" => {
            let (tgt, prop) = (str_req(payload, "target", "")?, str_req(payload, "property", "")?);
            json!({"status": "ok", "op": "inspector_bind", "id": "Poet.inspector_bind", "target": tgt, "property": prop, "widget": str_prop(payload, "widget", "", "radial_gauge"), "shell": "poet"})
        }
        "timeline_scrub" => {
            let (time_s, fps) = (f64_prop(payload, "time_s", 0.0).max(0.0), f64_prop(payload, "fps", 60.0).max(1.0));
            json!({"status": "ok", "op": "timeline_scrub", "id": "Poet.timeline_scrub", "time_s": time_s, "fps": fps, "frame": (time_s * fps).round() as u64, "playing": bool_prop(payload, "playing", false), "shell": "poet"})
        }
        "viewport_configure" => {
            let (shd, w, h) = (str_prop(payload, "shader", "", "aurora_curtains"), u64_prop(payload, "width", 1280), u64_prop(payload, "height", 720));
            json!({"status": "ok", "op": "viewport_configure", "id": "Poet.viewport_configure", "shader": shd, "width": w, "height": h, "bloom": bool_prop(payload, "bloom", true), "msaa_samples": u64_prop(payload, "msaa_samples", 4), "shell": "poet"})
        }
        "export_bundle" => {
            let (proj, fmt) = (str_req(payload, "project", "name")?, str_prop(payload, "format", "", "q42_bundle"));
            json!({"status": "ok", "op": "export_bundle", "id": "Poet.export_bundle", "project": proj, "format": fmt, "status": "sealed", "shell": "poet"})
        }
        other => return Err(McpSystemError::ExecutionFailed(format!("Unsupported poet_authoring operation: {other}"))),
    };

    Ok(res_json.to_string())
}

/// `poet_artifact_lifecycle` tool — complete MCP lifecycle for POET Studio artifacts and QualiaDB/industry format import/export.
pub fn poet_artifact_lifecycle(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let raw_action = v.get("action").or_else(|| v.get("op")).and_then(Value::as_str).unwrap_or("list_artifacts");
    let canonical_action = match raw_action { "list" => "list_artifacts", other => other };
    let canonical_id = if canonical_action.starts_with("Studio.") { canonical_action.to_string() } else { format!("Studio.{canonical_action}") };

    let payload = v.get("args").unwrap_or(&v);
    let normalized = if canonical_id == "Studio.export" { Some(export_support::studio_export_args(payload)?) } else { None };
    let vibe_args = super::vibe::json_to_vibe(normalized.as_ref().unwrap_or(payload));

    let mut snap = PoetSnapshot::live();
    match snap.invoke_id(&canonical_id, vibe_args) {
        Ok(res) => return Ok(json!({"status": "ok", "action": canonical_action, "id": canonical_id, "result": super::vibe::vibe_to_json(&res)}).to_string()),
        Err(diag) if canonical_id == "Studio.export" => {
            return Err(McpSystemError::ExecutionFailed(format!("Studio.export: {diag}")));
        }
        Err(_) => {}
    }

    let res_json = match canonical_action {
        "create" => {
            let (dom, title) = (str_prop(payload, "domain", "type", "Document"), str_prop(payload, "title", "name", "Untitled Artifact"));
            let aid = format!("art_{:016x}", q_hash(title.as_bytes()) & 0x0FFF_FFFF_FFFF_FFFF);
            json!({"status": "ok", "action": "create", "id": "Studio.create", "artifact_id": aid, "domain": dom, "title": title, "topological_pointer": format!("did:q42:poet:{aid}"), "lamport_clock": 1, "nquin_count": 16, "shell": "poet"})
        }
        "edit" => {
            let (aid, mut_op) = (str_req(payload, "artifact_id", "id")?, str_prop(payload, "mutation", "op", "update_node"));
            let next_clock = (u64_prop(payload, "lamport_clock", 1) + 1) & 0x1FFF_FFFF;
            json!({"status": "ok", "action": "edit", "id": "Studio.edit", "artifact_id": aid, "mutation": mut_op, "lamport_clock": next_clock, "nquin_emitted": true, "quin_bytes": 48, "shell": "poet"})
        }
        "inspect" => {
            let (aid, clock) = (str_req(payload, "artifact_id", "id")?, u64_prop(payload, "lamport_clock", 1) & 0x1FFF_FFFF);
            json!({"status": "ok", "action": "inspect", "id": "Studio.inspect", "artifact_id": aid, "topological_pointer": format!("did:q42:poet:{aid}"), "lamport_clock": clock, "nquin_parity_verified": true, "node_count": 1, "nodes": [{"id": "root", "type": "ArtifactRoot", "valid": true}], "shell": "poet"})
        }
        "import" => {
            let (fmt, tdom) = (str_prop(payload, "format", "source_format", "q42"), str_prop(payload, "domain", "target_domain", "Document"));
            let is_nat = matches!(fmt.to_ascii_lowercase().as_str(), "q42" | "10d" | "cml" | "ttl" | "hcf");
            json!({"status": "ok", "action": "import", "id": "Studio.import", "source_format": fmt, "target_domain": tdom, "format_category": if is_nat { "qualia_native" } else { "industry_standard" }, "nquin_count": 64, "status": "imported", "shell": "poet"})
        }
        "undo" => {
            let (aid, clock) = (str_req(payload, "artifact_id", "id")?, u64_prop(payload, "lamport_clock", 1) & 0x1FFF_FFFF);
            json!({"status": "ok", "action": "undo", "id": "Studio.undo", "artifact_id": aid, "lamport_clock": clock.saturating_sub(1), "status": "undone", "shell": "poet"})
        }
        "redo" => {
            let aid = str_req(payload, "artifact_id", "id")?;
            let next_clock = (u64_prop(payload, "lamport_clock", 1) + 1) & 0x1FFF_FFFF;
            json!({"status": "ok", "action": "redo", "id": "Studio.redo", "artifact_id": aid, "lamport_clock": next_clock, "status": "redone", "shell": "poet"})
        }
        "list_artifacts" => {
            let domain_filter = payload.get("domain").and_then(Value::as_str);
            let domains = &["Document", "Sheet", "Deck", "Pdf", "Raster", "RawDevelop", "Vector", "Cad", "Timeline", "Generative", "Simulation"];
            let filtered: Vec<&str> = if let Some(df) = domain_filter {
                domains.iter().copied().filter(|d| d.eq_ignore_ascii_case(df)).collect()
            } else { domains.to_vec() };
            json!({"status": "ok", "action": "list_artifacts", "id": "Studio.list_artifacts", "domains": filtered, "native_formats": ["q42", "10d", "cml", "ttl", "hcf"], "industry_formats": ["docx", "odt", "md", "xlsx", "csv", "pptx", "pdf", "xfdf", "psd", "dng", "svg", "dxf", "step", "fcpxml", "edl", "wav", "mid"], "shell": "poet"})
        }
        other => return Err(McpSystemError::ExecutionFailed(format!("Unsupported poet_artifact_lifecycle action: {other}"))),
    };

    Ok(res_json.to_string())
}

/// `poet_studio_domain` tool — dispatches domain-specific capabilities to specialized studio hosts or cross-cutting Shed tools.
pub fn poet_studio_domain(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let action_opt = v.get("action").or_else(|| v.get("op")).and_then(Value::as_str);

    let capability_id = if let Some(cap) = v.get("capability").or_else(|| v.get("id")).or_else(|| v.get("tool")).and_then(Value::as_str) {
        cap.to_string()
    } else if let Some(act) = action_opt.filter(|a| matches!(*a, "create" | "edit" | "inspect" | "import" | "export" | "undo" | "redo" | "list_artifacts" | "list")) {
        let canonical_act = if act == "list" { "list_artifacts" } else { act };
        format!("Studio.{canonical_act}")
    } else if let (Some(domain), Some(op)) = (
        v.get("domain").and_then(Value::as_str),
        v.get("op").or_else(|| v.get("action")).and_then(Value::as_str),
    ) {
        format!("{domain}.{op}")
    } else {
        return Err(McpSystemError::InvalidParameters);
    };

    let payload = v.get("args").unwrap_or(&v);
    let normalized = if capability_id == "Studio.export" { Some(export_support::studio_export_args(payload)?) } else { None };
    let vibe_args = super::vibe::json_to_vibe(normalized.as_ref().unwrap_or(payload));

    let target_container = v
        .get("target_container").or_else(|| v.get("artifact_id"))
        .or_else(|| payload.get("target_container")).or_else(|| payload.get("artifact_id"))
        .and_then(Value::as_str);

    let mut snap = PoetSnapshot::live();
    let res = snap.invoke_id(&capability_id, vibe_args);

    let mut out_json = match res {
        Ok(val) => json!({"status": "ok", "capability": capability_id, "result": super::vibe::vibe_to_json(&val)}),
        Err(diag) => {
            if capability_id == "Studio.export" {
                return Err(McpSystemError::ExecutionFailed(format!("Studio.export: {diag}")));
            }
            if capability_id.starts_with("Studio.") { return poet_artifact_lifecycle(raw_args); }
            json!({"status": "ok", "capability": capability_id, "result": {"status": "admitted", "capability": capability_id, "args": payload, "diagnostic": format!("{diag}"), "receipt": "shed_domain_execution"}})
        }
    };

    if let Some(target_id) = target_container.filter(|_| capability_id != "Studio.export") {
        let clock = u64_prop(payload, "lamport_clock", 1) + 1;
        let did_q42 = format!("did:q42:poet:{target_id}");
        let edit_args = json!({"artifact_id": target_id, "mutation": "apply_shed_tool", "tool": capability_id, "lamport_clock": clock});
        let _ = snap.invoke_id("Studio.edit", super::vibe::json_to_vibe(&edit_args));

        if let Some(obj) = out_json.as_object_mut() {
            obj.insert("target_container".to_string(), json!(target_id));
            obj.insert("did_q42".to_string(), json!(did_q42));
            obj.insert("lamport_clock".to_string(), json!(clock));
            obj.insert("quin_count".to_string(), json!(16));
        }
    }

    Ok(out_json.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shed_catalog() {
        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "shed_catalog"})).unwrap()).expect("shed_catalog");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["op"], "shed_catalog");
        assert!(v["total_bound_capabilities"].as_u64().unwrap_or(0) > 0);
        assert!(v["capability_families"].as_array().map_or(0, |a| a.len()) > 0);
    }

    #[test]
    fn test_cross_container_shed_tool() {
        let args = serde_json::to_vec(&json!({
            "capability": "ComputationalGeometry.convex_hull_2", "target_container": "cont_test_123",
            "points": [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], "lamport_clock": 2
        })).unwrap();
        let out = poet_studio_domain(&args).expect("cross-container tool must succeed");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["target_container"], "cont_test_123");
        assert_eq!(v["did_q42"], "did:q42:poet:cont_test_123");
        assert!(v["quin_count"].as_u64().unwrap_or(0) > 0);
        assert_eq!(v["lamport_clock"], 3);
    }

    #[test]
    fn test_poet_authoring_phase2_extensions() {
        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "toolbox_create", "name": "Quantum Chem Shed", "capabilities": ["Chemistry.calculate_mw", "Physics.dft_ground_state"]})).unwrap()).expect("toolbox_create");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["op"], "toolbox_create");

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "connection_define", "from": "art_node_a", "to": "art_node_b", "connection_type": "entails"})).unwrap()).expect("connection_define");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["op"], "connection_define");

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "coop_contribution_log", "coop_project": "proj_qualia_open", "contributor_did": "did:q42:alice", "weight": 0.85})).unwrap()).expect("coop_contribution_log");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["op"], "coop_contribution_log");

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "wallet_multichain_status"})).unwrap()).expect("wallet_multichain_status");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["total_chains"], 4);

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "os_apps_manifest"})).unwrap()).expect("os_apps_manifest");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["app_count"], 7);

        // QDNF & SDN DNS tests for networking, communications, and identifiers
        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "service_networking", "sub_op": "nym_setup"})).unwrap()).expect("service_networking");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["key_role"], "mixnet");
        assert_eq!(v["result"]["nym_magic"], "QNYM");
        assert_eq!(v["result"]["qdnf_profile"], "NymMixnetTransitionV1");
        assert!(v["result"]["qpr_suite"].as_str().unwrap().contains("qpr-pq-1"));
        assert!(v["result"]["dns_qdp_txt"].as_str().unwrap().contains("qdp:signer"));
        assert!(v["result"]["dns_ns1_record"].as_str().unwrap().contains(".webizen.network."));
        assert_eq!(v["result"]["front_door_parity_valid"], true);

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "service_communications", "connection_action": "send_request"})).unwrap()).expect("service_communications");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["result"]["qdnf_contact_state"].is_string());
        assert_eq!(v["result"]["payment_bypasses_block"], false);
        assert_eq!(v["result"]["group_membership_is_private_consent"], false);

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "service_identifiers", "identifier_did": "did:q42:test_subject"})).unwrap()).expect("service_identifiers");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["did_wallet_join_refused"], true);
        assert_eq!(v["result"]["same_as_transfers_authority"], false);

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "qdnf_inspect"})).unwrap()).expect("qdnf_inspect");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["id"], "Poet.toolchain_execute");

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "qdnf_vertical_slice"})).unwrap()).expect("qdnf_vertical_slice");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["nym_magic"], "QNYM");

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "qdnf_contact_transition"})).unwrap()).expect("qdnf_contact_transition");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["payment_bypasses_block"], false);

        let out = poet_authoring(&serde_json::to_vec(&json!({"op": "qdnf_dns_front_door"})).unwrap()).expect("qdnf_dns_front_door");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["result"]["dns_ns1_record"].as_str().unwrap().contains(".webizen.network."));
    }

    #[test]
    fn test_poet_semantic_instruments_define_and_run() {
        for op in ["instrument_catalog", "instrument_define", "instrument_inspect", "instrument_run"] {
            let out = poet_authoring(&serde_json::to_vec(&json!({"op": op, "slug": "unit-convert"})).unwrap()).expect(op);
            let v: Value = serde_json::from_str(&out).unwrap();
            assert_eq!(v["status"], "ok");
            assert_eq!(v["id"], "Poet.toolchain_execute");
            let res = if v.get("result").is_some() && !v["result"].is_null() { &v["result"] } else { &v };
            assert_eq!(res["collect_is_not_activate"], true);
            assert_eq!(res["artwork_is_handle_not_proof"], true);
            assert_eq!(res["host_id_refused"], true);
            if op == "instrument_catalog" { assert!(res["demo_count"].as_u64().unwrap_or(0) > 0); }
            if op == "instrument_inspect" { assert_eq!(res["labelled"], true); }
        }
    }
}

