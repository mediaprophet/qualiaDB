//! office:shapes — live N3Logic.evaluate + SHACL.validate (second Tool Chest chain).
//!
//! Same honesty as office:graph: standalone Poet stays executable; daemon
//! upgrades to the live ALL_BOUND id. Local paths never claim rule firing or
//! quin-backed SHACL. UTF-8 source; no ASCII fold.

use web_sys::{Document, Element};

const DEFAULT_N3: &str = concat!(
    "@prefix q42: <https://ns.webcivics.net/> .\n",
    "# 容器\n",
    "q42:doc a q42:Container .\n"
);

/// Bounded local N3 sketch. Counts prefixes, rule arrows, and triple-shaped
/// lines. Does not fire rules.
pub(super) fn local_n3_sketch(source: &str) -> String {
    let bounded: String = source.chars().take(16_384).collect();
    let mut prefixes = 0u32;
    let mut rule_arrows = 0u32;
    let mut triple_shaped_lines = 0u32;
    for line in bounded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");
        let first_fold: String = first.to_lowercase();
        if matches!(first_fold.as_str(), "@prefix" | "@base" | "prefix" | "base") {
            prefixes = prefixes.saturating_add(1);
            continue;
        }
        if trimmed.contains("=>")
            || trimmed.contains("~>")
            || trimmed.contains("^>")
            || trimmed.contains("-o")
        {
            rule_arrows = rule_arrows.saturating_add(1);
        }
        let tokens = trimmed
            .trim_end_matches('.')
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .count();
        if tokens >= 3 {
            triple_shaped_lines = triple_shaped_lines.saturating_add(1);
        }
    }
    serde_json::json!({
        "honesty": "local",
        "source": "poet-local",
        "prefixes": prefixes,
        "rule_arrows": rule_arrows,
        "triple_shaped_lines": triple_shaped_lines,
        "note": "Rule firing needs the QualiaDB daemon (N3Logic.evaluate)."
    })
    .to_string()
}

/// Standalone minCount-1 check: selected container carries a semantic annotation.
pub(super) fn local_shacl_mincount(subject: &str, has_annotation: bool) -> String {
    serde_json::json!({
        "honesty": "local",
        "source": "poet-local",
        "subject": subject,
        "kind": "minCount",
        "conforms": has_annotation,
        "note": "Property-path SHACL against live quins needs SHACL.validate on the daemon."
    })
    .to_string()
}

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
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

fn shacl_subject(container: &Element) -> String {
    container
        .get_attribute("data-semantic-uri")
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            container
                .get_attribute("data-semantic-type")
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| {
            container
                .get_attribute("data-container-id")
                .filter(|value| !value.trim().is_empty())
                .map(|id| format!("urn:poet:container:{id}"))
                .unwrap_or_else(|| "urn:poet:container:selected".into())
        })
}

fn has_semantic_annotation(container: &Element) -> bool {
    container
        .get_attribute("data-semantic-uri")
        .filter(|value| !value.trim().is_empty())
        .is_some()
        || container
            .get_attribute("data-semantic-type")
            .filter(|value| !value.trim().is_empty())
            .is_some()
}

pub(super) fn run_n3_evaluate(document: &Document, label: &str) {
    let label = label.to_string();
    let source = selected_text(document).unwrap_or_else(|| DEFAULT_N3.to_string());
    if !super::native_daemon::is_daemon_connected() {
        let detail = local_n3_sketch(&source);
        super::interactions::show_tool_status(document, &label, &detail, "success");
        return;
    }
    super::interactions::show_tool_status(document, &label, "Running N3Logic.evaluate…", "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let local_source = source.clone();
        let args = serde_json::json!({ "source": source, "mode": "evaluate" });
        match super::native_daemon::daemon_invoke("N3Logic.evaluate", args).await {
            Ok(response) if response.ok => {
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => {
                let detail = format!(
                    "Local N3 sketch after daemon rejection ({}): {}",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("N3Logic.evaluate failed."),
                    local_n3_sketch(&local_source)
                );
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
        }
    });
}

pub(super) fn run_shacl_validate(document: &Document, label: &str) {
    let Some(container) = selected_container(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a container before validating SHACL.",
            "error",
        );
        return;
    };
    let subject = shacl_subject(&container);
    let annotated = has_semantic_annotation(&container);
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let detail = local_shacl_mincount(&subject, annotated);
        super::interactions::show_tool_status(document, &label, &detail, "success");
        return;
    }
    super::interactions::show_tool_status(document, &label, "Running SHACL.validate…", "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({
            "subject": subject,
            "kind": "minCount",
            "value": 1
        });
        match super::native_daemon::daemon_invoke("SHACL.validate", args).await {
            Ok(response) if response.ok => {
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => {
                let detail = format!(
                    "Local SHACL sketch after daemon rejection ({}): {}",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("SHACL.validate failed."),
                    local_shacl_mincount(&subject, annotated)
                );
                super::interactions::show_tool_status(&document, &label, &detail, "success");
            }
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_n3_sketch_counts_utf8_source() {
        let source = concat!(
            "@prefix q42: <https://ns.webcivics.net/> .\n",
            "# 形状\n",
            "{ q42:doc a q42:Container } => { q42:doc a q42:Layout } .\n"
        );
        let sketch: serde_json::Value =
            serde_json::from_str(&local_n3_sketch(source)).expect("json");
        assert_eq!(sketch["prefixes"], 1);
        assert_eq!(sketch["rule_arrows"], 1);
        assert_eq!(sketch["honesty"], "local");
        assert!(!local_n3_sketch(source).to_lowercase().contains("fired"));
    }

    #[test]
    fn local_shacl_mincount_is_honest() {
        let ok: serde_json::Value =
            serde_json::from_str(&local_shacl_mincount("urn:poet:container:selected", true))
                .expect("json");
        assert_eq!(ok["conforms"], true);
        let miss: serde_json::Value =
            serde_json::from_str(&local_shacl_mincount("urn:poet:container:selected", false))
                .expect("json");
        assert_eq!(miss["conforms"], false);
        assert!(miss["note"]
            .as_str()
            .unwrap_or("")
            .contains("SHACL.validate"));
    }

    #[test]
    fn default_n3_is_utf8_and_not_empty() {
        assert!(DEFAULT_N3.contains("容器"));
        assert!(DEFAULT_N3.contains("@prefix"));
    }
}
