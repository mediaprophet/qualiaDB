//! Semantically described connector catalogue and execution console.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

use crate::browser::cop_records::{build_family_panel, CopField};
use crate::browser::native_daemon::{
    daemon_capabilities, is_daemon_connected, NativeCapabilityEntry,
};

const CONNECTOR_FIELDS: &[CopField] = &[
    CopField {
        key: "connector_id",
        placeholder: "Stable connector id",
    },
    CopField {
        key: "interface_iri",
        placeholder: "Interface IRI (urn:qualia:interface:…)",
    },
    CopField {
        key: "input_class_iri",
        placeholder: "Input semantic class IRI",
    },
    CopField {
        key: "output_class_iri",
        placeholder: "Output semantic class IRI",
    },
    CopField {
        key: "transport",
        placeholder: "local-invoke | http | websocket | mcp | pulse | file",
    },
    CopField {
        key: "capability_id",
        placeholder: "Native capability id (optional with endpoint)",
    },
    CopField {
        key: "endpoint",
        placeholder: "Endpoint URI (optional with capability)",
    },
    CopField {
        key: "auth_mode",
        placeholder: "none | capability | oauth | did-signature",
    },
    CopField {
        key: "probe_args",
        placeholder: "JSON object used to test the binding",
    },
    CopField {
        key: "effect_class",
        placeholder: "Pure | Cold | unknown",
    },
    CopField {
        key: "arg_schema",
        placeholder: "Negotiated input machine schema",
    },
    CopField {
        key: "return_schema",
        placeholder: "Negotiated output machine schema",
    },
    CopField {
        key: "sensitivity",
        placeholder: "public | restricted | classified",
    },
    CopField {
        key: "status",
        placeholder: "draft | configured | enabled | disabled",
    },
];

pub fn build_integrations_view(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-connector-workspace", "semantic")
        .ok();
    style(
        &root,
        "display:flex;flex-direction:column;gap:8px;padding:9px;overflow:auto;",
    );

    let intro = document.create_element("div").unwrap();
    intro.set_inner_html(
        "<h3 style=\"margin:0 0 3px\">Connectors</h3>\
         <p style=\"margin:0;color:var(--text-muted);font-size:10px\">\
         Discover runnable host capabilities, describe connector inputs and outputs as semantic \
         classes, and invoke a selected local capability with explicit JSON arguments.</p>",
    );
    root.append_child(&intro).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    style(
        &status,
        "font:10px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let controls = document.create_element("div").unwrap();
    let refresh = button(document, "Discover host connectors");
    controls.append_child(&refresh).unwrap();
    root.append_child(&controls).unwrap();

    let catalogue = document.create_element("div").unwrap();
    catalogue.set_attribute("data-connector-catalogue", "").ok();
    style(
        &catalogue,
        "display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:6px;max-height:270px;overflow:auto;",
    );
    root.append_child(&catalogue).unwrap();

    let runner = document.create_element("div").unwrap();
    style(
        &runner,
        "display:grid;grid-template-columns:1fr auto;gap:6px;border:1px solid var(--border-medium);border-radius:6px;padding:8px;",
    );
    let capability = document.create_element("input").unwrap();
    capability.set_attribute("data-connector-run-id", "").ok();
    capability
        .set_attribute("placeholder", "Capability id, e.g. Inference.grounding")
        .ok();
    runner.append_child(&capability).unwrap();
    let run = button(document, "Run connector");
    runner.append_child(&run).unwrap();
    let args = document.create_element("textarea").unwrap();
    args.set_attribute("data-connector-run-args", "").ok();
    args.set_text_content(Some("{}"));
    style(
        &args,
        "grid-column:1/-1;min-height:80px;font-family:var(--font-mono);",
    );
    runner.append_child(&args).unwrap();
    let result = document.create_element("pre").unwrap();
    result.set_attribute("data-connector-result", "").ok();
    style(
        &result,
        "grid-column:1/-1;white-space:pre-wrap;max-height:220px;overflow:auto;",
    );
    result.set_text_content(Some("No connector invocation yet."));
    runner.append_child(&result).unwrap();
    root.append_child(&runner).unwrap();

    let refresh_root = root.clone();
    let refresh_status = status.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_catalogue(&refresh_root, &refresh_status);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
        .unwrap();
    refresh_closure.forget();

    let run_root = root.clone();
    let run_status = status.clone();
    let run_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let id = input_value(&run_root, "[data-connector-run-id]");
        let raw = textarea_value(&run_root, "[data-connector-run-args]");
        let parsed = serde_json::from_str::<serde_json::Value>(&raw);
        let Ok(arguments) = parsed else {
            run_status.set_text_content(Some("Connector arguments must be valid JSON."));
            return;
        };
        if id.trim().is_empty() {
            run_status.set_text_content(Some("Select or enter a capability id."));
            return;
        }
        run_status.set_text_content(Some("Invoking negotiated local connector…"));
        let root = run_root.clone();
        let status = run_status.clone();
        let output = root
            .query_selector("[data-connector-result]")
            .ok()
            .flatten();
        super::connector_runs::execute_and_record(
            &status,
            output,
            id.trim().to_string(),
            id.trim().to_string(),
            serde_json::to_string(&arguments).unwrap_or_else(|_| "{}".into()),
            "unknown".into(),
            1,
        );
    }) as Box<dyn FnMut(_)>);
    run.add_event_listener_with_callback("click", run_closure.as_ref().unchecked_ref())
        .unwrap();
    run_closure.forget();

    root.append_child(&build_family_panel(
        document,
        "project_integration",
        "Persist semantic connector contracts. A configured record is not considered connected until its transport succeeds.",
        CONNECTOR_FIELDS,
    ))
    .unwrap();
    root.append_child(&super::connector_health::build_connector_health(
        document, &root,
    ))
    .unwrap();
    root.append_child(&super::connector_runs::build_connector_runs(document))
        .unwrap();
    refresh_catalogue(&root, &status);
    root
}

fn refresh_catalogue(root: &Element, status: &Element) {
    if !is_daemon_connected() {
        status.set_text_content(Some("Unavailable: connect the local daemon."));
        return;
    }
    status.set_text_content(Some("Discovering negotiated native capabilities…"));
    let root = root.clone();
    let status = status.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_capabilities().await {
            Ok(document) => {
                render_catalogue(&root, &document.capabilities);
                status.set_text_content(Some(&format!(
                    "{} connector capabilities discovered from {}.",
                    document.capabilities.len(),
                    document.execution_host
                )));
            }
            Err(error) => status.set_text_content(Some(&error)),
        }
    });
}

fn render_catalogue(root: &Element, capabilities: &[NativeCapabilityEntry]) {
    let Some(catalogue) = root
        .query_selector("[data-connector-catalogue]")
        .ok()
        .flatten()
    else {
        return;
    };
    catalogue.set_inner_html("");
    let document = root.owner_document().unwrap();
    for entry in capabilities.iter().filter(|entry| entry.available) {
        let card = document.create_element("article").unwrap();
        style(
            &card,
            "border:1px solid var(--border-medium);border-radius:5px;padding:7px;background:var(--surface-panel);",
        );
        let title = document.create_element("strong").unwrap();
        title.set_text_content(Some(&entry.id));
        card.append_child(&title).unwrap();
        let semantic = document.create_element("div").unwrap();
        semantic.set_text_content(Some(&format!(
            "interface: urn:qualia:capability:{}\nfamily: {} · effect: {} · honesty: {}\nsemantics: {}\ntransport: {}\nmode: {}\ninput schema: {}\noutput schema: {}",
            entry.id.replace('.', ":"),
            entry.family,
            entry.effect_class,
            entry.honesty,
            entry.semantics,
            entry.transport,
            entry.mode,
            serde_json::to_string(&entry.arg_schema).unwrap_or_default(),
            serde_json::to_string(&entry.return_schema).unwrap_or_default()
        )));
        style(
            &semantic,
            "white-space:pre-wrap;font:9px var(--font-mono);color:var(--text-muted);",
        );
        card.append_child(&semantic).unwrap();
        let select = button(&document, "Use in runner");
        let id = entry.id.clone();
        let initial_args = default_arguments(&entry.arg_schema);
        let semantic_stem = entry.id.replace('.', ":");
        let effect_class = entry.effect_class.clone();
        let arg_schema = serde_json::to_string(&entry.arg_schema).unwrap_or_default();
        let return_schema = serde_json::to_string(&entry.return_schema).unwrap_or_default();
        let root_select = root.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            if let Some(input) = root_select
                .query_selector("[data-connector-run-id]")
                .ok()
                .flatten()
                .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
            {
                input.set_value(&id);
            }
            if let Some(input) = root_select
                .query_selector("[data-connector-run-args]")
                .ok()
                .flatten()
                .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
            {
                input.set_value(&initial_args);
            }
            set_descriptor_field(&root_select, "connector_id", &id);
            set_descriptor_field(
                &root_select,
                "interface_iri",
                &format!("urn:qualia:capability:{semantic_stem}"),
            );
            set_descriptor_field(
                &root_select,
                "input_class_iri",
                &format!("urn:qualia:schema:{semantic_stem}:input"),
            );
            set_descriptor_field(
                &root_select,
                "output_class_iri",
                &format!("urn:qualia:schema:{semantic_stem}:output"),
            );
            set_descriptor_field(&root_select, "transport", "local-invoke");
            set_descriptor_field(&root_select, "capability_id", &id);
            set_descriptor_field(&root_select, "auth_mode", "capability");
            set_descriptor_field(&root_select, "sensitivity", "restricted");
            set_descriptor_field(&root_select, "status", "configured");
            set_descriptor_field(&root_select, "probe_args", &initial_args);
            set_descriptor_field(&root_select, "effect_class", &effect_class);
            set_descriptor_field(&root_select, "arg_schema", &arg_schema);
            set_descriptor_field(&root_select, "return_schema", &return_schema);
        }) as Box<dyn FnMut(_)>);
        select
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        card.append_child(&select).unwrap();
        catalogue.append_child(&card).unwrap();
    }
}

fn default_arguments(schema: &serde_json::Value) -> String {
    match schema.get("type").and_then(serde_json::Value::as_str) {
        Some("list") | Some("array") => "[]".into(),
        Some("null") => "null".into(),
        _ => "{}".into(),
    }
}

fn set_descriptor_field(root: &Element, key: &str, value: &str) {
    if let Some(input) = root
        .query_selector(&format!(
            "[data-cop-family=\"project_integration\"] [data-cop-field=\"{key}\"]"
        ))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        input.set_value(value);
    }
}

fn input_value(root: &Element, selector: &str) -> String {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn textarea_value(root: &Element, selector: &str) -> String {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn style(element: &Element, css: &str) {
    element
        .clone()
        .dyn_into::<HtmlElement>()
        .unwrap()
        .style()
        .set_css_text(css);
}
