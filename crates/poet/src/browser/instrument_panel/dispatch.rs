//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Click dispatch from instrument-panel tool buttons.

use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

pub(super) fn dispatch_instrument_action(document: &Document, tool_id: &str, label: &str) {
    match tool_id {
        "doc:bold" => super::commands::exec_document_command(document, "bold", None, label),
        "doc:italic" => super::commands::exec_document_command(document, "italic", None, label),
        "doc:code" => super::commands::exec_document_command(
            document,
            "insertHTML",
            Some("<code class=\"cml-code\">code</code>"),
            label,
        ),
        "doc:entity" => super::commands::exec_document_command(
            document,
            "insertHTML",
            Some(
                "<q-entity category=\"entity\" iri=\"did:qualia:entity#term\" class=\"cml-entity\">Tagged Entity</q-entity>",
            ),
            label,
        ),
        "doc:objective" | "epi:objective" => crate::browser::tool_actions::dispatch(
            document,
            "epistemic:tag_objective",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "doc:subjective" | "epi:subjective" => crate::browser::tool_actions::dispatch(
            document,
            "epistemic:tag_subjective",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "epi:inter" => crate::browser::tool_actions::dispatch(
            document,
            "epistemic:tag_intersubjective",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "epi:normative" => crate::browser::tool_actions::dispatch(
            document,
            "epistemic:tag_normative",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "doc:view-md" => {
            super::commands::click_selected(document, ".doc-view-tab[data-doc-view=\"markdown\"]", label)
        }
        "doc:view-rdf" => super::commands::click_selected(document, ".doc-view-tab[data-doc-view=\"rdf\"]", label),
        "code:run" | "graph:sparql" | "health:nlp_ingest" => super::commands::click_selected(
            document,
            &format!("[data-instrument-action=\"{tool_id}\"]"),
            label,
        ),
        "office:doc" => crate::browser::interactions::place_container_via_menu(document, "doc", label),
        "office:ont" => crate::browser::interactions::place_container_via_menu(document, "ontology", label),
        "office:slide" => crate::browser::interactions::place_container_via_menu(document, "slide", label),
        "img:media" => crate::browser::interactions::place_container_via_menu(document, "media", label),
        "sheet:place" => crate::browser::interactions::place_container_via_menu(document, "sheet", label),
        "spatial:map" => crate::browser::interactions::place_container_via_menu(document, "map", label),
        "spatial:3d" => crate::browser::interactions::place_container_via_menu(document, "3d", label),
        "comm:social" => crate::browser::interactions::place_container_via_menu(document, "social", label),
        "comm:webrtc" => crate::browser::interactions::place_container_via_menu(document, "webrtc", label),
        "comm:webview" => crate::browser::interactions::place_container_via_menu(document, "webview", label),
        "health:place" => crate::browser::interactions::place_container_via_menu(document, "health", label),
        "health:anat" => crate::browser::interactions::place_container_via_menu(document, "anatomy", label),
        "code:vibe" => crate::browser::interactions::place_container_via_menu(document, "code", label),
        "ai:triad" => crate::browser::interactions::place_container_via_menu(document, "triad", label),
        "img:marker" => crate::browser::tool_actions::dispatch(
            document,
            "image:marker",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "spatial:pin" => crate::browser::tool_actions::dispatch(
            document,
            "spatial:pin",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "rights:group" => crate::browser::tool_actions::dispatch(
            document,
            "rights:authors_group",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Invoke,
        ),
        "ai:extractor" | "ai:sentinel" => crate::browser::tool_actions::dispatch(
            document,
            tool_id,
            label,
            crate::tool_chest::core::intent_bus::ActionType::Invoke,
        ),
        "scene:create" => super::commands::invoke_session(
            document,
            label,
            "Scene.create",
            serde_json::json!({ "name": "poet-scene-session" }),
        ),
        "render:gpu_adapter" => super::commands::invoke_session(
            document,
            label,
            "Render.gpu_adapter_info",
            serde_json::json!({}),
        ),
        "audio:transport_play" => super::commands::invoke_session(
            document,
            label,
            "Audio.transport",
            serde_json::json!({ "action": "play", "tempo": 120.0 }),
        ),
        "audio:transport_stop" => super::commands::invoke_session(
            document,
            label,
            "Audio.transport",
            serde_json::json!({ "action": "stop", "tempo": 120.0 }),
        ),
        "audio:oscillator" => super::commands::invoke_session(
            document,
            label,
            "Audio.oscillator",
            serde_json::json!({
                "waveform": "sine",
                "frequency": 440.0,
                "sample_rate": 44100.0,
                "n": 512
            }),
        ),
        "sheet:fx" => {
            if let Some(input) = super::commands::selected_container(document).and_then(|container| {
                container
                    .query_selector(".vibe-toolbar input")
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            }) {
                let _ = input.focus();
                crate::browser::interactions::show_tool_status(
                    document,
                    label,
                    "Formula bar focused. Enter =SUM(A1:A10) in a cell.",
                    "success",
                );
            } else {
                crate::browser::interactions::show_tool_status(
                    document,
                    label,
                    "Select a sheet container first.",
                    "error",
                );
            }
        }
        "sheet:sum" => super::commands::invoke_session(
            document,
            label,
            "Sheet.sum_range",
            super::commands::sheet_grid_args(document),
        ),
        "sheet:avg" => super::commands::invoke_session(document, label, "Sheet.stats", super::commands::sheet_grid_args(document)),
        "ont:add-row" => super::commands::click_selected(document, "[data-cop-family] button", label),
        "ont:shacl" => super::commands::invoke_session(document, label, "SHACL.extensions", serde_json::json!({})),
        "ont:classes" => super::commands::invoke_session(
            document,
            label,
            "GraphDatabase.stats",
            serde_json::json!({}),
        ),
        "ont:export" => super::commands::invoke_session(
            document,
            label,
            "GraphAuthoring.process",
            serde_json::json!({
                "source": crate::browser::ontology_views::persist::PERSON_SAFE_N3,
                "mode": "ontology_compile",
                "format": "turtle"
            }),
        ),
        "social:connect" => super::commands::invoke_session(
            document,
            label,
            "Pulse.publish_notification",
            serde_json::json!({ "channel": "poet/social-requests" }),
        ),
        "social:chat" => super::commands::invoke_session(
            document,
            label,
            "Pulse.publish",
            serde_json::json!({ "channel": "poet/social", "payload_type": "agent-message" }),
        ),
        "social:agent" => super::commands::invoke_session(
            document,
            label,
            "Pulse.publish_agent_message",
            serde_json::json!({ "channel": "poet/social" }),
        ),
        "social:graph" => {
            let demo = serde_json::json!({
                "fragments": [
                    {
                        "fragment_id": "aaaaaaaaaaaaaaaa",
                        "message_lamport": 1,
                        "anchor_start": 0,
                        "anchor_end": 12,
                        "anchor_text": "hello social"
                    },
                    {
                        "fragment_id": "bbbbbbbbbbbbbbbb",
                        "message_lamport": 2,
                        "anchor_start": 0,
                        "anchor_end": 5,
                        "anchor_text": "reply"
                    }
                ],
                "edges": [{
                    "child_fragment_id": "bbbbbbbbbbbbbbbb",
                    "parent_fragment_id": "aaaaaaaaaaaaaaaa",
                    "reply_message_lamport": 2
                }]
            });
            if !crate::browser::native_daemon::is_daemon_connected() {
                let report = crate::browser::tool_dual_path::local_sketch(
                    "ChatGraph.session_summary",
                    "2 fragments / 1 edge (caller-supplied demo; Host ChatGraph.* — not desktop jsonl)",
                );
                crate::browser::interactions::show_tool_status(
                    document,
                    label,
                    &report.message,
                    report.status_kind,
                );
            } else {
                super::commands::invoke_session(
                    document,
                    label,
                    "ChatGraph.session_summary",
                    demo,
                );
            }
        }
        "graph:expand" | "graph:collapse" | "graph:layout" => super::commands::invoke_session(
            document,
            label,
            "GraphDatabase.stats",
            serde_json::json!({}),
        ),
        "map:pin" => crate::browser::tool_actions::dispatch(
            document,
            "spatial:pin",
            label,
            crate::tool_chest::core::intent_bus::ActionType::Annotate,
        ),
        "3d:orbit" | "3d:pan" | "3d:zoom" | "3d:wireframe" => super::commands::invoke_session(
            document,
            label,
            "Render.gpu_adapter_info",
            serde_json::json!({}),
        ),
        "health:biomarker" => crate::browser::interactions::place_container_via_menu(
            document,
            "health_calculators",
            label,
        ),
        "health:tomography" => super::commands::invoke_session(
            document,
            label,
            "MedicalImaging.hu_window",
            serde_json::json!({
                "study_uid": "urn:poet:anatomy:demo-slice",
                "width": 2,
                "height": 2,
                "pixels": [-160.0, 40.0, 240.0, 1000.0],
                "window": 400.0,
                "level": 40.0
            }),
        ),
        "health:anatomy" => {
            crate::browser::interactions::place_container_via_menu(document, "anatomy", label)
        }
        "code:ast" => super::commands::click_selected(document, "[data-instrument-action=\"code:run\"]", label),
        "code:pulse" => {
            super::commands::insert_into_editor(document, "pulse::emit(\"poet/topic\", \"payload\")", label)
        }
        "code:cap" => super::commands::insert_into_editor(
            document,
            "capability.invoke(\"Poet.manifold_create\", { label: \"New lens\", nest: true })",
            label,
        ),
        "rights:sign" | "rights:audit" => super::commands::invoke_session(
            document,
            label,
            "DeonticLogic.evaluate",
            serde_json::json!({ "modality": "obligate", "body": "rights" }),
        ),
        "rights:consent" => super::commands::click_selected(document, "[data-cop-family] button", label),
        "webview:clip" => super::commands::invoke_session(
            document,
            label,
            "Document.ingest",
            serde_json::json!({ "text": "sandbox navigation record", "uri": "urn:poet:webview" }),
        ),
        _ => crate::browser::interactions::show_tool_status(
            document,
            label,
            "Unavailable: this instrument has no registered standalone runtime contract.",
            "unavailable",
        ),
    }
}
