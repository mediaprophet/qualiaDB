//! Truthful publication workflow over the implemented checkpoint and
//! construct-package primitives.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

pub fn build_publication_workflow_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("workflow-panel publication-workflow");
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display:flex;flex-direction:column;flex:1;gap:8px;padding:8px;overflow:auto;\
         font-family:var(--font-mono);",
    );
    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("📦 Publication Workflow"));
    root.append_child(&title).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some(
        "Checkpoint snapshots and construct HCF/HMC packages are live and checksummed.",
    ));
    root.append_child(&status).unwrap();

    let snapshot = button(document, "Create snapshot checkpoint");
    let hcf = button(document, "Export current construct .hcf");
    let hmc = button(document, "Archive current construct .hmc");
    root.append_child(&snapshot).unwrap();
    root.append_child(&hcf).unwrap();
    root.append_child(&hmc).unwrap();

    let status_snapshot = status.clone();
    let snapshot_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        match super::manifest::save_checkpoint(
            "Publication snapshot",
            super::manifest::SaveMode::Snapshot,
        ) {
            Ok(checkpoint) => status_snapshot
                .set_text_content(Some(&format!("Stored snapshot `{}`.", checkpoint.id))),
            Err(error) => status_snapshot.set_text_content(Some(&error)),
        }
    }) as Box<dyn FnMut(_)>);
    snapshot
        .add_event_listener_with_callback("click", snapshot_closure.as_ref().unchecked_ref())
        .unwrap();
    snapshot_closure.forget();

    for (control, archive) in [(hcf, false), (hmc, true)] {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            super::construct_shelf::export_construct(
                &document,
                &super::current_construct_id(),
                archive,
            );
        }) as Box<dyn FnMut(_)>);
        control
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    for (label, reason) in [
        (
            "Prune tombstones",
            "Unavailable: the checkpoint store has no operation/tombstone DAG to prune.",
        ),
        (
            "Generate credits",
            "Unavailable: contributor and transformation provenance records are not registered.",
        ),
        (
            "Export signed .q42 distribution",
            "Unavailable: requires consent records, a Q42 distribution encoder, and an unlocked DID signing session.",
        ),
        (
            "Strip metadata",
            "Unavailable: requires a verified fiduciary capability and a provenance-aware distribution artifact.",
        ),
    ] {
        let disabled = button(document, label);
        disabled.set_attribute("disabled", "").ok();
        disabled.set_attribute("aria-disabled", "true").ok();
        disabled.set_attribute("title", reason).ok();
        disabled.set_attribute("data-disabled-reason", reason).ok();
        root.append_child(&disabled).unwrap();
    }
    root
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}
