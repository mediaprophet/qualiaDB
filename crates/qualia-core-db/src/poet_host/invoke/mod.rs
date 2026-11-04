//! capability.invoke.
//!
//! Folders are **future crate seams** (D16). Do not extract workspace crates yet.
//! Add a family by adding a file in the matching seam folder and one match arm.

pub mod agent;
mod args;
mod asset_bind;
mod asset_store;
mod audio;
mod biosignal;
mod chat_graph;
mod chemistry;
mod clinical;
pub mod cooperative;
mod cosmic_bind;
pub mod coverage;
mod crypto;
mod dispatch_geom_ml;
mod dispatch_gov_media;
mod dispatch_math_stats;
mod dispatch_render_econ;
mod dispatch_sci_studio;
mod docs;
pub mod econ;
mod engineering;
mod geometry;
mod governance;
mod graph;
mod hid_seam;
mod hypermedia;
pub mod ids;
pub mod inference;
mod logic;
pub mod manifold;
mod math;
mod medical;
mod ml;
mod net;
mod nlp;
pub mod os_shared_services;
mod poet_shell;
pub mod qdnf_bridge;
mod render;
mod research;
mod runtime;
mod sampler_bind;
mod science;
pub mod semantic_instrument_bridge;
pub mod shed_registry;
mod sheet;
mod social;
mod solvers;
pub mod space_weather_bridge;
mod stats;
mod studio_domains;
pub mod studio_store;
pub mod topology_bind;
mod vision;
mod wgsl_forge_seam;

use super::PoetSnapshot;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// Dispatches a VibeScript call by name.
pub fn dispatch(
    snap: &mut PoetSnapshot,
    id: &str,
    args: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    match id {
        ids::DAG_EXECUTE => agent::dag_execute(args, span),
        ids::DAG_VALIDATE => agent::dag_validate(args, span),
        ids::DAG_STATUS => agent::dag_status(args, span),
        ids::ORCH_SESSION_CREATE => agent::orchestration::session_create(args, span),
        ids::ORCH_SESSION_PLAN => agent::orchestration::session_plan(args, span),
        ids::ORCH_SESSION_EXECUTE => agent::orchestration::session_execute(args, span),
        ids::ORCH_SESSION_STATUS => agent::orchestration::session_status(args, span),
        ids::ORCH_ROSTER_REGISTER => agent::orchestration::roster_register(args, span),
        ids::ORCH_ROSTER_LIST => agent::orchestration::roster_list(args, span),
        ids::ORCH_ROSTER_CAPABILITIES => agent::orchestration::roster_capabilities(args, span),
        ids::ORCH_ASSIGN_AGENTS => agent::orchestration::assign_agents_seam(args, span),
        ids::AGENT_PLAN => agent::runtime::agent_plan(args, span),
        ids::AGENT_EXECUTE => agent::runtime::agent_execute(args, span),
        ids::AGENT_EVALUATE => agent::runtime::agent_evaluate(args, span),
        ids::CORPUS_LOAD => agent::runtime::corpus_load(args, span),
        ids::CORPUS_PARSE => agent::runtime::corpus_parse(args, span),
        ids::HID_POLL => hid_seam::hid_poll(snap, args, span),
        ids::HID_WAIT => hid_seam::hid_wait(snap, args, span),
        ids::HID_CLEAR => hid_seam::hid_clear(snap, args, span),
        ids::HID_POINTER_CAPTURE => hid_seam::pointer_capture(args, span),
        ids::HID_POINTER_RELEASE => hid_seam::pointer_release(args, span),
        ids::HID_SET_CURSOR => hid_seam::set_cursor(args, span),
        ids::HID_GAMEPAD_POLL => hid_seam::gamepad_poll(args, span),
        ids::HID_GAMEPAD_VIBRATE => hid_seam::gamepad_vibrate(args, span),
        ids::HID_MIDI_SEND => hid_seam::midi_send(args, span),
        ids::HID_MIDI_POLL => hid_seam::midi_poll(args, span),
        ids::HID_HAPTIC_PULSE => hid_seam::haptic_pulse(args, span),
        ids::HID_HAPTIC_PATTERN => hid_seam::haptic_pattern(args, span),
        ids::HID_SPATIAL_HEAD_POSE => hid_seam::spatial_head_pose(args, span),
        ids::HID_SPATIAL_HAND_SKELETON => hid_seam::spatial_hand_skeleton(args, span),
        ids::HID_SPATIAL_GAZE_RAY => hid_seam::spatial_gaze_ray(args, span),
        ids::HID_BIOSIGNAL_POLL => hid_seam::biosignal_poll(args, span),
        ids::DISCOVERY_LIST | "CapabilityDiscovery" | "list_capabilities" => Ok(runtime::list()),
        ids::HASH_IRI => runtime::iri(args, span),
        ids::SHACL_VALIDATE => graph::shacl_validate(snap, args, span),
        ids::SHACL_EXTENSIONS => graph::shacl_extensions(args, span),
        ids::GRAPH_STATS | "get_graph_stats" => Ok(graph::stats(snap)),
        ids::GRAPH_SPARQL => graph::sparql(snap, args, span),
        ids::GRAPH_VOLUME_OPEN => graph::volume_open(snap, args, span),
        "GraphDatabase.admit_pack_slice" => graph::admit_pack_slice(snap, args, span),
        ids::GRAPH_VOLUME_COMMIT => graph::volume_commit(snap, args, span),
        ids::GRAPH_LEXICON_MANIFEST => graph::lexicon_manifest(snap, args, span),
        ids::N3_EVALUATE => graph::n3_evaluate(args, span),
        ids::GRAPH_AUTHORING => graph::graph_authoring(args, span),
        ids::GRAPH_SHORTEST_PATH
        | "graph.shortest_path"
        | "navigation.a_star"
        | "solvers.graph.path"
        | "graph.dijkstra" => graph::shortest_path(args, span),
        "solvers.graph.dijkstra" | "solvers.graph.shortest_path" => {
            solvers::graph_shortest_path(args, span)
        }
        ids::GRAPH_SPREADING_ACTIVATION => graph::spreading_activation(args, span),
        ids::DEONTIC_EVAL | "deontic.evaluate" | "deontic.eval" => {
            logic::deontic_evaluate(snap, args, span)
        }
        ids::EPISTEMIC_EVAL => logic::epistemic_evaluate(snap, args, span),
        ids::PARACONSISTENT_ROUTE => logic::paraconsistent_route(snap, args, span),
        ids::LTL_GLOBALLY | "ltl.globally" => logic::ltl_globally(snap, args, span),
        ids::LTL_FINALLY | "ltl.finally" => logic::ltl_finally(snap, args, span),
        ids::LTL_EVALUATE | "ltl.evaluate" | "ltl.evaluate_trace" => {
            logic::ltl_evaluate(args, span)
        }
        "Econ.accounting_balance" | "accounting.balance" | "rea.balance" => {
            econ::econ::econ_accounting_balance(args, span)
        }
        ids::DL_SUBSUMES => logic::subsumption_check(snap, args, span),
        ids::ASP_ENUMERATE => logic::asp_evaluate(snap, args, span),
        ids::CAUSAL_CAUSED => logic::caused(snap, args, span),
        ids::FUZZY_TNORM => logic::t_norm(args, span),
        ids::ADVANCED_LOGIC_WORKBENCH => logic::advanced_compute(args, span),
        ids::FORMAL_LOGIC_WORKBENCH => logic::formal_compute(args, span),
        ids::LEGAL_LOGIC_WORKBENCH => logic::legal_compute(snap, args, span),
        ids::GOVERNANCE_LOGIC_WORKBENCH => logic::governance_compute(args, span),
        ids::SPATIAL_LOGIC_WORKBENCH
        | "temporal.allen"
        | "temporal.allen_relation"
        | "spatial.workbench" => logic::spatial_compute(args, span),
        ids::INFRA_LOGIC_WORKBENCH => logic::infra_compute(args, span),
        ids::INFRA_EXT_LOGIC_WORKBENCH => logic::infra_ext_compute(args, span),
        other => {
            if let Some(r) = dispatch_math_stats::try_dispatch(other, args, span) {
                return r;
            }
            if let Some(r) = dispatch_geom_ml::try_dispatch(other, args, span) {
                return r;
            }
            if let Some(r) = dispatch_sci_studio::try_dispatch(other, args, span) {
                return r;
            }
            if let Some(r) = dispatch_render_econ::try_dispatch(snap, other, args, span) {
                return r;
            }
            if let Some(r) = dispatch_gov_media::try_dispatch(other, args, span) {
                return r;
            }
            Err(Diagnostic::new(
                DiagCode::E300,
                span,
                format!("capability.invoke({other}): unbound; add poet_host/invoke/<seam>/<family>.rs"),
            ))
        }
    }
}
