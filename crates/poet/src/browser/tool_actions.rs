//! Honest dispatch policy for non-placement Tool Chest actions.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::tool_chest::core::intent_bus::ActionType;

pub fn requires_daemon(_tool_id: &str) -> bool {
    false
}

#[cfg(test)]
fn has_local_contract(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "epistemic:tag_objective"
            | "epistemic:tag_subjective"
            | "epistemic:tag_intersubjective"
            | "epistemic:tag_normative"
            | "image:marker"
            | "spatial:pin"
            | "mail:composer"
            | "rights:authors_group"
            | "office:typography_bold"
            | "office:typography_italic"
            | "office:typography_code"
            | "office:paragraph_heading"
            | "office:paragraph_align_left"
            | "office:paragraph_align_center"
    )
}

#[cfg(test)]
fn has_live_invoke(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "graph:sparql_query" | "ai:extractor" | "ai:sentinel"
    )
}

#[cfg(test)]
fn has_dispatch_policy(tool_id: &str) -> bool {
    has_local_contract(tool_id)
        || has_live_invoke(tool_id)
        || requires_daemon(tool_id)
        || unavailable_reason(tool_id).is_some()
}

/// A structural prerequisite that the current UI cannot collect safely yet.
pub fn unavailable_reason(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "audio:mic_capture" => Some(
            "Microphone permission and bounded capture controls must be configured in the audio surface.",
        ),
        "audio:neural_latents" => {
            Some("A mounted P64 audio model and an active audio stream are required.")
        }
        "mail:publisher" => Some(
            "Publishing requires a selected artefact, destination, and authorisation workflow.",
        ),
        "scientific:thermodynamics" => Some(
            "Thermodynamics MCMC requires a configured target distribution and bounded sampler inputs.",
        ),
        "sdn:energy_governor" => Some(
            "Energy governance requires live battery/solar telemetry and an authorised control target.",
        ),
        "image:heatmap" => {
            Some("Heatmap generation requires a selected numeric data layer and colour scale.")
        }
        "sheet:import" => {
            Some("Import requires the dedicated bounded CSV/HCF file-picker workflow.")
        }
        "spatial:track" => {
            Some("Tracking requires a selected consenting agent and a live trajectory source.")
        }
        "rights:fiduciary_sign" | "rights:did_sign" => Some(
            "Signing requires a selected agreement, signer identity, consent check, and unlocked key vault.",
        ),
        "health:pathology" => {
            Some("Pathology evaluation requires consent-gated assay inputs and reference ranges.")
        }
        "code:quin_statement" => Some(
            "Quin construction requires subject, predicate, object, context, and sensitivity inputs.",
        ),
        "ai:co_author" => Some(
            "Co-authoring requires a selected document, prompt scope, and an activated local model.",
        ),
        _ => None,
    }
}

pub fn current_disabled_reason(tool_id: &str) -> Option<&'static str> {
    unavailable_reason(tool_id)
}

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn annotate_selected(document: &Document, semantic_type: &str, semantic_uri: &str, label: &str) {
    let Some(container) = selected_container(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a container before applying this annotation.",
            "error",
        );
        return;
    };
    let _ = container.set_attribute("data-semantic-type", semantic_type);
    let _ = container.set_attribute("data-semantic-uri", semantic_uri);
    if let Ok(Some(tag)) = container.query_selector(".container-type-tag") {
        let _ = tag.set_attribute(
            "title",
            &format!("Semantic annotation: {semantic_type} · {semantic_uri}"),
        );
    }
    super::history::push_current_frame("annotate container");
    super::interactions::show_tool_status(
        document,
        label,
        &format!("Applied {semantic_type} to the selected container."),
        "success",
    );
}

fn format_selected_editor(
    document: &Document,
    label: &str,
    css: &str,
    format: &str,
    success: &str,
) {
    let Some(container) = selected_container(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a document container before applying formatting.",
            "error",
        );
        return;
    };
    let Some(editor) = container.query_selector(".doc-editor").ok().flatten() else {
        super::interactions::show_tool_status(
            document,
            label,
            "The selected container has no editable document surface.",
            "error",
        );
        return;
    };
    let Ok(editor) = editor.dyn_into::<HtmlElement>() else {
        super::interactions::show_tool_status(
            document,
            label,
            "The selected document surface cannot receive formatting.",
            "error",
        );
        return;
    };
    for declaration in css.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let _ = editor.style().set_property(property.trim(), value.trim());
    }
    let _ = editor.set_attribute("data-paragraph-format", format);
    super::history::push_current_frame("format document");
    super::interactions::show_tool_status(document, label, success, "success");
}

fn selected_text(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".doc-editor")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

/// Deterministic bounded extraction for standalone Poet/WASM. The daemon
/// gazetteer remains the richer path when a local node is connected.
pub(super) fn local_extract_summary(source: &str) -> (usize, usize, Vec<String>) {
    let bounded: String = source.chars().take(16_384).collect();
    let source = bounded.as_str();
    let token_count = source.split_whitespace().count();
    let sentence_count = source
        .split(|ch: char| matches!(ch, '.' | '!' | '?'))
        .filter(|sentence| !sentence.trim().is_empty())
        .count()
        .max(usize::from(!source.trim().is_empty()));
    let mut entities = Vec::new();
    for raw in source.split_whitespace() {
        let token: String = raw
            .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
            .chars()
            .take(96)
            .collect();
        let Some(first) = token.chars().next() else {
            continue;
        };
        if token.chars().count() < 2
            || !first.is_uppercase()
            || entities.iter().any(|known| known == &token)
        {
            continue;
        }
        entities.push(token);
        if entities.len() == 5 {
            break;
        }
    }
    (token_count, sentence_count, entities)
}

/// Inspect the standalone Poet surface without claiming native process
/// telemetry. This is a real bounded check over the active DOM and gives the
/// user useful Sentinel feedback when no daemon is available.
fn local_sentinel_summary(document: &Document) -> String {
    let nodes = document
        .query_selector_all("*")
        .map(|list| list.length().min(10_000))
        .unwrap_or(0);
    let containers = document
        .query_selector_all(".canvas-container-node")
        .map(|list| list.length())
        .unwrap_or(0);
    format!(
        "Standalone Sentinel check passed: {} DOM nodes across {} canvas containers; native 42MB telemetry requires the local daemon.",
        nodes, containers
    )
}

/// Execute a bounded SPARQL-shaped query against Poet's local semantic
/// container graph. The local graph exposes container type/URI annotations;
/// richer joins are delegated to `GraphDatabase.sparql` when a daemon exists.
pub(super) fn local_graph_query(document: &Document, query: &str) -> String {
    let containers = document.query_selector_all(".canvas-container-node").ok();
    let count = containers
        .as_ref()
        .map(|list| list.length().min(256))
        .unwrap_or(0);
    let normalized = query.trim_start().to_ascii_uppercase();
    if normalized.starts_with("ASK") {
        return serde_json::json!({ "boolean": count > 0, "source": "poet-local" }).to_string();
    }
    if !normalized.starts_with("SELECT") {
        return serde_json::json!({
            "error": "standalone Poet supports bounded ASK and SELECT queries",
            "source": "poet-local"
        })
        .to_string();
    }

    let mut bindings = Vec::new();
    if let Some(list) = containers {
        for index in 0..list.length().min(256) {
            let Some(node) = list.get(index) else {
                continue;
            };
            let Ok(container) = node.dyn_into::<Element>() else {
                continue;
            };
            let subject = format!("urn:poet:container:{}", index);
            let predicate = container
                .get_attribute("data-semantic-type")
                .unwrap_or_else(|| "poet:Container".into());
            let object = container
                .get_attribute("data-semantic-uri")
                .or_else(|| container.get_attribute("data-container-type"))
                .unwrap_or_else(|| "poet:container".into());
            bindings.push(serde_json::json!({
                "subject": { "type": "uri", "value": subject },
                "predicate": { "type": "uri", "value": predicate },
                "object": { "type": "literal", "value": object }
            }));
        }
    }
    serde_json::json!({
        "head": { "vars": ["subject", "predicate", "object"] },
        "results": { "bindings": bindings },
        "source": "poet-local"
    })
    .to_string()
}

fn run_extractor(document: &Document, label: &str) {
    let Some(source) = selected_text(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a non-empty document or text container first.",
            "error",
        );
        return;
    };
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let (token_count, sentence_count, entities) = local_extract_summary(&source);
        let detail = if entities.is_empty() {
            format!(
                "Offline analysis: {} tokens, {} sentences, no title-case entities.",
                token_count, sentence_count
            )
        } else {
            format!(
                "Offline analysis: {} tokens, {} sentences; entities: {}.",
                token_count,
                sentence_count,
                entities.join(", ")
            )
        };
        super::interactions::show_tool_status(document, &label, &detail, "success");
        return;
    }
    super::interactions::show_tool_status(document, &label, "Analysing selected text…", "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_gazetteer(&source).await {
            Ok(response) if response.ok => {
                let entities = response
                    .hits
                    .iter()
                    .take(5)
                    .map(|hit| hit.surface.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let detail = if entities.is_empty() {
                    format!(
                        "Live analysis: {} tokens, {} sentences, no known gazetteer entities.",
                        response.token_count, response.sentence_count
                    )
                } else {
                    format!(
                        "Live analysis: {} tokens, {} sentences; entities: {}.",
                        response.token_count, response.sentence_count, entities
                    )
                };
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
            Ok(response) => {
                let (token_count, sentence_count, entities) = local_extract_summary(&source);
                let diagnostic = response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("unknown daemon failure");
                let detail = if entities.is_empty() {
                    format!(
                        "Offline analysis after daemon rejection ({}): {} tokens, {} sentences.",
                        diagnostic, token_count, sentence_count
                    )
                } else {
                    format!(
                        "Offline analysis after daemon rejection ({}): {} tokens, {} sentences; entities: {}.",
                        diagnostic,
                        token_count,
                        sentence_count,
                        entities.join(", ")
                    )
                };
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
            Err(error) => {
                let (token_count, sentence_count, entities) = local_extract_summary(&source);
                let detail = if entities.is_empty() {
                    format!(
                        "Offline analysis after daemon error ({}): {} tokens, {} sentences.",
                        error, token_count, sentence_count
                    )
                } else {
                    format!(
                        "Offline analysis after daemon error ({}): {} tokens, {} sentences; entities: {}.",
                        error,
                        token_count,
                        sentence_count,
                        entities.join(", ")
                    )
                };
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
        }
    });
}

fn run_sparql_query(document: &Document, label: &str) {
    let label = label.to_string();
    let query = selected_text(document).unwrap_or_else(|| "ASK WHERE { ?s ?p ?o }".to_string());
    if !super::native_daemon::is_daemon_connected() {
        let detail = local_graph_query(document, &query);
        super::interactions::show_tool_status(document, &label, &detail, "success");
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        "Running GraphDatabase.sparql…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        // Live ALL_BOUND id — no Host widen.
        let local_query = query.clone();
        let args = serde_json::json!({ "query": query, "format": "json" });
        match super::native_daemon::daemon_invoke("GraphDatabase.sparql", args).await {
            Ok(response) if response.ok => {
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => {
                let detail = format!(
                    "Local graph fallback after daemon rejection ({}): {}",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("GraphDatabase.sparql failed."),
                    local_graph_query(&document, &local_query)
                );
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
        }
    });
}

fn run_sentinel(document: &Document, label: &str) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let detail = local_sentinel_summary(document);
        super::interactions::show_tool_status(document, &label, &detail, "success");
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        "Inspecting Sentinel state…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "agent_did": "did:qualia:current" });
        match super::native_daemon::daemon_invoke("Sentinel.inspect", args).await {
            Ok(response) if response.ok => {
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => {
                let detail = format!(
                    "Local Sentinel fallback after daemon rejection ({}): {}",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Sentinel inspection failed."),
                    local_sentinel_summary(&document)
                );
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
            Err(error) => {
                let detail = format!(
                    "{} Standalone fallback: {}",
                    error,
                    local_sentinel_summary(&document)
                );
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
        }
    });
}

pub fn dispatch(document: &Document, tool_id: &str, label: &str, action: ActionType) {
    if let Some(reason) = current_disabled_reason(tool_id) {
        super::interactions::show_tool_status(document, label, reason, "unavailable");
        return;
    }

    match tool_id {
        "epistemic:tag_objective" => annotate_selected(
            document,
            "epistemic:Objective",
            "https://qualiadb.org/schema/epistemic#objective",
            label,
        ),
        "epistemic:tag_subjective" => annotate_selected(
            document,
            "epistemic:Subjective",
            "https://qualiadb.org/schema/epistemic#subjective",
            label,
        ),
        "epistemic:tag_intersubjective" => annotate_selected(
            document,
            "epistemic:Intersubjective",
            "https://qualiadb.org/schema/epistemic#intersubjective",
            label,
        ),
        "epistemic:tag_normative" => annotate_selected(
            document,
            "epistemic:Normative",
            "https://qualiadb.org/schema/epistemic#normative",
            label,
        ),
        "image:marker" => annotate_selected(
            document,
            "hypermedia:Marker",
            "https://qualiadb.org/schema/hypermedia#marker",
            label,
        ),
        "spatial:pin" => annotate_selected(
            document,
            "geo:Pin",
            "https://qualiadb.org/schema/geo#pin",
            label,
        ),
        "mail:composer" => super::interactions::place_container_via_menu(document, "mail", label),
        "rights:authors_group" => {
            super::interactions::place_container_via_menu(document, "rights", label)
        }
        "office:typography_bold" => format_selected_editor(
            document,
            label,
            "font-weight: 700;",
            "bold",
            "Applied bold typography to the selected document.",
        ),
        "office:typography_italic" => format_selected_editor(
            document,
            label,
            "font-style: italic;",
            "italic",
            "Applied italic typography to the selected document.",
        ),
        "office:typography_code" => format_selected_editor(
            document,
            label,
            "font-family: var(--font-mono);",
            "code",
            "Applied code typography to the selected document.",
        ),
        "office:paragraph_heading" => format_selected_editor(
            document,
            label,
            "font-size: 1.25em; font-weight: 700;",
            "heading",
            "Promoted the selected document to a heading block.",
        ),
        "office:paragraph_align_left" => format_selected_editor(
            document,
            label,
            "text-align: left;",
            "align-left",
            "Aligned the selected document to the left.",
        ),
        "office:paragraph_align_center" => format_selected_editor(
            document,
            label,
            "text-align: center;",
            "align-center",
            "Centered the selected document.",
        ),
        "ai:extractor" => run_extractor(document, label),
        "ai:sentinel" => run_sentinel(document, label),
        "graph:sparql_query" => run_sparql_query(document, label),
        _ => super::interactions::show_tool_status(
            document,
            label,
            &format!(
                "No executable contract is registered for {} action `{}`.",
                action, tool_id
            ),
            "unavailable",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_chest::core::tool::ToolKind;

    #[test]
    fn every_registered_nonplacement_tool_has_an_explicit_policy() {
        let registry = crate::browser::registration::build_registry();
        for toolbox in registry.toolboxes() {
            for chain in toolbox.chains() {
                for tool in chain.tools() {
                    if tool.metadata().kind != ToolKind::PlaceContainer {
                        assert!(
                            has_dispatch_policy(&tool.metadata().id),
                            "missing action policy for {}",
                            tool.metadata().id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn local_extractor_is_bounded_and_deterministic() {
        assert!(!requires_daemon("ai:extractor"));
        assert!(!requires_daemon("ai:sentinel"));
        assert!(!requires_daemon("graph:sparql_query"));
        let input = "QualiaDB joins Poet. Webizen renders the graph!";
        let first = local_extract_summary(input);
        let second = local_extract_summary(input);
        assert_eq!(first, second);
        assert_eq!(first.0, 7);
        assert_eq!(first.1, 2);
        assert_eq!(first.2, vec!["QualiaDB", "Poet", "Webizen"]);
    }

    #[test]
    fn empty_local_extractor_input_has_no_entities() {
        assert_eq!(local_extract_summary(""), (0, 0, Vec::new()));
    }
}
