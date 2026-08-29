//! Reconcile persisted semantic connector contracts with negotiated host capabilities.

use std::collections::BTreeMap;

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

use crate::browser::native_daemon::{
    daemon_capabilities, daemon_library_ingest, daemon_records_query, NativeCapabilityEntry,
    NativeLibraryIngestRequest, NativeRecordQueryRequest,
};

pub fn build_connector_health(document: &Document, workspace: &Element) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-connector-health", "").ok();
    style(
        &root,
        "border:1px solid var(--border-medium);border-radius:6px;padding:8px;display:flex;flex-direction:column;gap:6px;",
    );
    let header = document.create_element("div").unwrap();
    style(
        &header,
        "display:flex;justify-content:space-between;gap:8px;",
    );
    let title = document.create_element("strong").unwrap();
    title.set_text_content(Some("Semantic connector registry health"));
    header.append_child(&title).unwrap();
    let refresh = button(document, "Reconcile");
    header.append_child(&refresh).unwrap();
    root.append_child(&header).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-connector-health-list", "").ok();
    list.set_text_content(Some("No reconciliation run yet."));
    root.append_child(&list).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    style(
        &status,
        "font:9px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let local_root = root.clone();
    let local_workspace = workspace.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        reconcile(&local_root, &local_workspace);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    reconcile(&root, workspace);
    root
}

fn reconcile(root: &Element, workspace: &Element) {
    set_status(
        root,
        "Reconciling records with the negotiated capability catalogue…",
    );
    let root = root.clone();
    let workspace = workspace.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let capabilities = daemon_capabilities().await;
        let records = daemon_records_query(NativeRecordQueryRequest {
            family: "project_integration".into(),
            ..Default::default()
        })
        .await;
        match (capabilities, records) {
            (Ok(capabilities), Ok(records)) if records.ok => {
                render(&root, &workspace, &records.data, &capabilities.capabilities);
                set_status(
                    &root,
                    "Reconciled. Ready means a negotiated local capability exists; external configuration is not presented as connected.",
                );
            }
            (Err(error), _) | (_, Err(error)) => set_status(&root, &error),
            (_, Ok(records)) => set_status(
                &root,
                records
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Connector records could not be loaded."),
            ),
        }
    });
}

fn render(
    root: &Element,
    workspace: &Element,
    data: &serde_json::Value,
    capabilities: &[NativeCapabilityEntry],
) {
    let Some(list) = root
        .query_selector("[data-connector-health-list]")
        .ok()
        .flatten()
    else {
        return;
    };
    list.set_inner_html("");
    let document = root.owner_document().unwrap();
    let capability_map = capabilities
        .iter()
        .map(|capability| (capability.id.as_str(), capability))
        .collect::<BTreeMap<_, _>>();
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for record in records {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let connector_id = field(fields, "connector_id");
        let transport = field(fields, "transport");
        let capability_id = field(fields, "capability_id");
        let state = if transport == "local-invoke" {
            match capability_map.get(capability_id) {
                Some(capability) if capability.available => "READY",
                Some(_) => "NEGOTIATED / UNAVAILABLE",
                None => "UNBOUND",
            }
        } else {
            "CONFIGURED / ADAPTER NOT PROBED"
        };
        let card = document.create_element("article").unwrap();
        style(
            &card,
            "border-left:2px solid var(--accent-cyan);padding:6px 8px;background:var(--surface-panel);margin:4px 0;",
        );
        let description = document.create_element("div").unwrap();
        description.set_text_content(Some(&format!(
            "{connector_id} · {state}\ninterface: {}\ninput: {}\noutput: {}\ntransport: {transport} · auth: {} · sensitivity: {}",
            field(fields, "interface_iri"),
            field(fields, "input_class_iri"),
            field(fields, "output_class_iri"),
            field(fields, "auth_mode"),
            field(fields, "sensitivity")
        )));
        style(
            &description,
            "white-space:pre-wrap;font:9px var(--font-mono);",
        );
        card.append_child(&description).unwrap();
        let publish = button(&document, "Publish semantics to Library");
        let semantic_fields = fields.cloned().unwrap_or_default();
        let publish_root = root.clone();
        let publish_connector_id = connector_id.to_string();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            publish_semantics(&publish_root, &publish_connector_id, &semantic_fields);
        }) as Box<dyn FnMut(_)>);
        publish
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        card.append_child(&publish).unwrap();
        if state == "READY" {
            let probe = button(&document, "Run saved probe");
            let args = field(fields, "probe_args");
            let capability_id = capability_id.to_string();
            let connector_id = connector_id.to_string();
            let effect_class = capability_map
                .get(capability_id.as_str())
                .map(|capability| capability.effect_class.clone())
                .unwrap_or_else(|| field(fields, "effect_class").to_string());
            let root = root.clone();
            let workspace = workspace.clone();
            let raw = if args.trim().is_empty() { "{}" } else { args }.to_string();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                run_probe(
                    &root,
                    &workspace,
                    &connector_id,
                    &capability_id,
                    &raw,
                    &effect_class,
                );
            }) as Box<dyn FnMut(_)>);
            probe
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            card.append_child(&probe).unwrap();
        }
        list.append_child(&card).unwrap();
    }
    if records.is_empty() {
        list.set_text_content(Some(
            "No semantic connector contracts have been saved. Use the descriptor panel above.",
        ));
    }
}

fn publish_semantics(
    root: &Element,
    connector_id: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
) {
    let interface = field(Some(fields), "interface_iri");
    if interface.is_empty() {
        set_status(root, "The connector has no semantic interface IRI.");
        return;
    }
    let capability_id = field(Some(fields), "capability_id");
    let binding_iri = if capability_id.is_empty() {
        field(Some(fields), "endpoint").to_string()
    } else {
        format!("urn:qualia:capability:{}", capability_id.replace('.', ":"))
    };
    let document = serde_json::json!({
        "@context": {
            "qualia": "urn:qualia:",
            "inputClass": {"@id": "qualia:inputClass", "@type": "@id"},
            "outputClass": {"@id": "qualia:outputClass", "@type": "@id"},
            "capability": {"@id": "qualia:capability", "@type": "@id"}
        },
        "@id": interface,
        "@type": "qualia:ConnectorInterface",
        "qualia:connectorId": connector_id,
        "inputClass": field(Some(fields), "input_class_iri"),
        "outputClass": field(Some(fields), "output_class_iri"),
        "qualia:transport": field(Some(fields), "transport"),
        "qualia:capability": binding_iri,
        "qualia:authMode": field(Some(fields), "auth_mode"),
        "qualia:sensitivity": field(Some(fields), "sensitivity"),
        "qualia:status": field(Some(fields), "status")
    });
    let request = NativeLibraryIngestRequest {
        uri: interface.to_string(),
        media_type: "application/ld+json".into(),
        text: serde_json::to_string_pretty(&document).unwrap_or_default(),
        section: Some("tools".into()),
        sensitivity: Some(field(Some(fields), "sensitivity").to_string()),
        projects: vec!["connectors".into()],
        purposes: vec!["agent-tool-discovery".into()],
        occurred_at: Some((js_sys::Date::now() / 1000.0).floor() as i64),
        place_label: None,
        lat: None,
        lon: None,
    };
    let root = root.clone();
    set_status(&root, "Publishing the JSON-LD connector description…");
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_library_ingest(request).await {
            Ok(response) if response.ok => set_status(
                &root,
                "Connector JSON-LD published to the persistent Semantic Library.",
            ),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Semantic publication failed."),
            ),
            Err(error) => set_status(&root, &error),
        }
    });
}

fn run_probe(
    root: &Element,
    workspace: &Element,
    connector_id: &str,
    id: &str,
    raw: &str,
    effect: &str,
) {
    let status = root
        .query_selector("[role=\"status\"]")
        .ok()
        .flatten()
        .unwrap_or_else(|| root.clone());
    let output = workspace
        .query_selector("[data-connector-result]")
        .ok()
        .flatten();
    super::connector_runs::execute_and_record(
        &status,
        output,
        connector_id.into(),
        id.into(),
        raw.into(),
        effect.into(),
        1,
    );
}

fn field<'a>(fields: Option<&'a serde_json::Map<String, serde_json::Value>>, key: &str) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn set_status(root: &Element, text: &str) {
    if let Some(status) = root.query_selector("[role=\"status\"]").ok().flatten() {
        status.set_text_content(Some(text));
    }
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
