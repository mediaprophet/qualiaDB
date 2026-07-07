//! **Library** panel — the personal hypermedia asset library. Ingest a document (it's *processed* to derive
//! topics + a searchable text representation), then find your files **by meaning** — topic, depiction, place,
//! project, purpose — never by folder. A flagged ingest under a guardianship relation notifies the guardian.

use super::host_client::{ingest_document, list_library, search_library};
use dioxus::prelude::*;

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn arr_join(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

#[component]
pub fn WellfairLibraryPanel() -> Element {
    let mut results = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);

    let mut ing_uri = use_signal(|| "urn:doc:my-note".to_string());
    let mut ing_text = use_signal(|| "Notes: the liver is an organ; keep this receipt for the tax deduction.".to_string());
    let mut ing_guardian = use_signal(String::new);

    let mut facet = use_signal(|| "topic".to_string());
    let mut value = use_signal(|| "biology".to_string());

    let reload = move || {
        spawn(async move {
            if let Ok(serde_json::Value::Array(rows)) = list_library().await {
                results.set(rows);
            }
        });
    };
    use_effect(move || reload());

    let do_ingest = move |_| {
        let (uri, text, g) = (ing_uri(), ing_text(), ing_guardian());
        spawn(async move {
            let guardian = if g.trim().is_empty() { None } else { Some(g) };
            match ingest_document(&uri, "text/markdown", &text, guardian).await {
                Ok(v) => {
                    let topics = arr_join(&v, "topics");
                    let n_flags = v.get("flags").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
                    let n_notif = v.get("guardian_notifications").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
                    status.set(format!(
                        "Ingested {uri} — topics: [{topics}]{}{}",
                        if n_flags > 0 { format!(" · {n_flags} flag(s)") } else { String::new() },
                        if n_notif > 0 { format!(" · notified guardian ({n_notif})") } else { String::new() },
                    ));
                    reload();
                }
                Err(e) => status.set(format!("Ingest failed: {e}")),
            }
        });
    };
    let do_search = move |_| {
        let (f, v) = (facet(), value());
        spawn(async move {
            match search_library(&f, &v).await {
                Ok(serde_json::Value::Array(rows)) => {
                    let n = rows.len();
                    results.set(rows);
                    status.set(format!("{n} result(s) for {f} = \"{v}\"."));
                }
                Ok(_) => results.set(Vec::new()),
                Err(e) => status.set(format!("Search failed: {e}")),
            }
        });
    };

    let field_style = "padding:0.3rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;background:var(--qualia-surface-2,#fff);";
    let label_style = "display:flex;flex-direction:column;gap:0.15rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);";

    rsx! {
        section {
            aria_label: "WellFair hypermedia library",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);display:flex;flex-direction:column;gap:0.8rem;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Library — find your files by meaning" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Show all"
                }
            }
            p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                "Ingest a document — it's processed to derive its topics and a searchable text form — then find it by topic, place, project or purpose. Not a folder of files; a graph of meaning."
            }
            if !status().is_empty() {
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-accent,#2b6);", "{status()}" }
            }

            // ── Ingest ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.5rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.8rem;font-weight:600;", "Ingest a document" }
                label { style: "{label_style}", "Asset id (uri)"
                    input { style: "{field_style}", value: "{ing_uri}", oninput: move |e| ing_uri.set(e.value()) } }
                label { style: "{label_style}", "Text"
                    textarea { style: "{field_style}min-height:3rem;font-family:inherit;", value: "{ing_text}", oninput: move |e| ing_text.set(e.value()) } }
                label { style: "{label_style}", "Guardian DID (optional — flagged ingest notifies them)"
                    input { style: "{field_style}", value: "{ing_guardian}", oninput: move |e| ing_guardian.set(e.value()) } }
                button {
                    style: "align-self:flex-start;padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.78rem;cursor:pointer;",
                    onclick: do_ingest,
                    "Ingest"
                }
            }

            // ── Search ──
            div {
                style: "display:flex;gap:0.4rem;align-items:flex-end;flex-wrap:wrap;",
                label { style: "{label_style}", "Find by"
                    select {
                        style: "{field_style}",
                        value: "{facet}",
                        oninput: move |e| facet.set(e.value()),
                        option { value: "topic", "Topic" }
                        option { value: "depicts", "Depicts" }
                        option { value: "place", "Place" }
                        option { value: "project", "Project" }
                        option { value: "purpose", "Purpose" }
                    }
                }
                label { style: "{label_style}", "Value"
                    input { style: "{field_style}", value: "{value}", oninput: move |e| value.set(e.value()) } }
                button {
                    style: "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                    onclick: do_search,
                    "Search"
                }
            }

            // ── Results ──
            if !results.read().is_empty() {
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
                    for r in results.read().clone() {
                        li {
                            key: "{str_field(&r, \"asset_uri\")}",
                            style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.74rem;display:flex;flex-direction:column;gap:0.15rem;",
                            strong { "{str_field(&r, \"asset_uri\")}" }
                            if !arr_join(&r, "topics").is_empty() {
                                div { style: "font-size:0.68rem;color:var(--qualia-accent,#2b6);", "topics: {arr_join(&r, \"topics\")}" }
                            }
                            div { style: "color:var(--qualia-text-muted,#777);", "{str_field(&r, \"excerpt\")}" }
                            if r.get("flags").and_then(|x| x.as_array()).map(|a| !a.is_empty()).unwrap_or(false) {
                                div { style: "font-size:0.68rem;color:var(--qualia-danger,#b44);", "flagged" }
                            }
                        }
                    }
                }
            }
        }
    }
}
