//! Live checkpoint history, restore, and snapshot export UI.

use base64::Engine;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlAnchorElement, HtmlElement, HtmlInputElement};

pub fn build_checkpoint_tray_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("workflow-panel checkpoint-tray");
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display:flex;flex-direction:column;flex:1;gap:8px;padding:8px;overflow:auto;\
         font-family:var(--font-mono);",
    );
    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("📔 Checkpoint History · main"));
    root.append_child(&title).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();

    match super::manifest::checkpoint_history() {
        Ok(history) if history.is_empty() => {
            status.set_text_content(Some("No checkpoints yet. Use File › Save As…"));
        }
        Ok(history) => {
            status.set_text_content(Some(&format!("{} stored checkpoint(s).", history.len())));
            for (index, checkpoint) in history.iter().rev().enumerate() {
                let row = document.create_element("label").unwrap();
                row.set_attribute(
                    "style",
                    "display:flex;gap:6px;padding:6px;border:1px solid var(--border-subtle);border-radius:4px;",
                )
                .ok();
                let choice: HtmlInputElement = document
                    .create_element("input")
                    .unwrap()
                    .dyn_into()
                    .unwrap();
                choice.set_type("radio");
                choice.set_name("checkpoint-choice");
                choice.set_value(&checkpoint.id);
                choice.set_checked(index == 0);
                row.append_child(&choice).unwrap();
                let text = document.create_element("span").unwrap();
                let label = if checkpoint.label.is_empty() {
                    checkpoint.id.as_str()
                } else {
                    checkpoint.label.as_str()
                };
                text.set_text_content(Some(&format!(
                    "{:?} · {} · {} · {}",
                    checkpoint.save_mode, label, checkpoint.actor, checkpoint.timestamp
                )));
                row.append_child(&text).unwrap();
                root.append_child(&row).unwrap();
            }
        }
        Err(error) => status.set_text_content(Some(&error)),
    }

    let actions = document.create_element("div").unwrap();
    actions
        .set_attribute("style", "display:flex;gap:6px;flex-wrap:wrap;")
        .ok();
    let restore = action_button(document, "↩ Restore selected");
    let export = action_button(document, "📤 Export selected .hmc");
    let branch = action_button(document, "🌼 Branch");
    branch.set_attribute("disabled", "").ok();
    branch.set_attribute("aria-disabled", "true").ok();
    branch
        .set_attribute(
            "title",
            "Unavailable: branch DAG and merge semantics are not implemented in the checkpoint store.",
        )
        .ok();
    actions.append_child(&restore).unwrap();
    actions.append_child(&export).unwrap();
    actions.append_child(&branch).unwrap();
    root.append_child(&actions).unwrap();

    let root_restore = root.clone();
    let status_restore = status.clone();
    let restore_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let Some(id) = selected_checkpoint(&root_restore) else {
            status_restore.set_text_content(Some("Select a checkpoint first."));
            return;
        };
        match super::manifest::load_checkpoint_seeds(&id)
            .and_then(super::restore_manifold_checkpoint)
        {
            Ok(()) => status_restore.set_text_content(Some(&format!("Restored `{id}`."))),
            Err(error) => status_restore.set_text_content(Some(&error)),
        }
    }) as Box<dyn FnMut(_)>);
    restore
        .add_event_listener_with_callback("click", restore_closure.as_ref().unchecked_ref())
        .unwrap();
    restore_closure.forget();

    let root_export = root.clone();
    let status_export = status.clone();
    let export_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let Some(id) = selected_checkpoint(&root_export) else {
            status_export.set_text_content(Some("Select a checkpoint first."));
            return;
        };
        match export_checkpoint(&id) {
            Ok(()) => status_export
                .set_text_content(Some(&format!("Exported `{id}` as a construct .hmc."))),
            Err(error) => status_export.set_text_content(Some(&error)),
        }
    }) as Box<dyn FnMut(_)>);
    export
        .add_event_listener_with_callback("click", export_closure.as_ref().unchecked_ref())
        .unwrap();
    export_closure.forget();
    root
}

pub fn build_checkpoint_indicator_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("workflow-widget checkpoint-indicator");
    let text = super::manifest::checkpoint_history()
        .ok()
        .and_then(|history| history.last().cloned())
        .map(|checkpoint| format!("🌼 main │ last: {} │ persisted", checkpoint.timestamp))
        .unwrap_or_else(|| "🌼 main │ last: (none) │ unsaved".into());
    root.set_text_content(Some(&text));
    root
}

fn action_button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn selected_checkpoint(root: &Element) -> Option<String> {
    root.query_selector("input[name=checkpoint-choice]:checked")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .filter(|id| !id.is_empty())
}

fn export_checkpoint(id: &str) -> Result<(), String> {
    let seeds = super::manifest::load_checkpoint_seeds(id)?;
    let construct_id = super::current_construct_id();
    let mut construct = crate::tool_chest::constructs::construct_by_id(&construct_id)
        .ok_or_else(|| format!("unknown construct `{construct_id}`"))?;
    let visible_ids: std::collections::BTreeSet<String> = super::visible_seeds()
        .into_iter()
        .map(|seed| seed.id)
        .collect();
    let manifolds: Vec<_> = if construct.id == "poet" {
        seeds
    } else {
        seeds
            .into_iter()
            .filter(|seed| visible_ids.contains(&seed.id))
            .collect()
    };
    construct.manifold_ids = manifolds.iter().map(|seed| seed.id.clone()).collect();
    let observer = super::current_observer_did();
    let author = if observer.is_empty() {
        super::manifest::DEFAULT_ACTOR_DID
    } else {
        observer.as_str()
    };
    let bytes = super::manifest::export_construct_hmc(&construct, &manifolds, author)?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("no document")?;
    let anchor: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|_| "could not create download")?
        .dyn_into()
        .map_err(|_| "download element was not an anchor")?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    anchor.set_href(&format!("data:application/vnd.qualia.hmc;base64,{encoded}"));
    anchor.set_download(&format!("{}-{id}.hmc", construct.id));
    anchor.click();
    Ok(())
}
