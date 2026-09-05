//! Quin triple inspection, reification, and project module management.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "code:quin-inspect" => Some(inspect_quin(container)),
        "code:quin-ref" => Some(query_references(container)),
        "code:quin-reify" => Some(reify_quin(container)),
        "code:add-module" => Some(add_module(container)),
        "code:remove-module" => Some(remove_module(container)),
        _ => None,
    }
}

fn inspect_quin(container: &Element) -> Result<(), String> {
    let s = container.get_attribute("data-quin-subject").unwrap_or_else(|| "did:qualia:subject".to_string());
    let p = container.get_attribute("data-quin-predicate").unwrap_or_else(|| "schema:predicate".to_string());
    let o = container.get_attribute("data-quin-object").unwrap_or_else(|| "did:qualia:object".to_string());
    let summary = format!("Quin(S={s}, P={p}, O={o})");
    container
        .set_attribute("data-quin-inspection", &summary)
        .map_err(|_| "Failed to write Quin inspection summary.".to_string())
}

fn query_references(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-quin-references", "queried:matching_references_active")
        .map_err(|_| "Failed to query Quin references.".to_string())
}

fn reify_quin(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-quin-reified", "rdf-star:statement_reified")
        .map_err(|_| "Failed to reify Quin statement.".to_string())
}

fn add_module(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-project-modules")
        .unwrap_or_default();
    let entry = "mod:module_entry";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current};{entry}")
    };
    container
        .set_attribute("data-project-modules", &updated)
        .map_err(|_| "Failed to add project module.".to_string())
}

fn remove_module(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-project-modules")
        .unwrap_or_default();
    let updated: Vec<_> = current
        .split(';')
        .filter(|m| !m.is_empty())
        .collect();
    let updated_str = if updated.len() > 1 {
        updated[..updated.len() - 1].join(";")
    } else {
        String::new()
    };
    container
        .set_attribute("data-project-modules", &updated_str)
        .map_err(|_| "Failed to remove project module.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quin_actions_route_safely() {
        assert!(run(&web_sys::Element::from(wasm_bindgen::JsValue::NULL), "code:unknown").is_none());
    }
}
