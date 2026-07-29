//! **Directory** pane — the personal "Active Directory"-like view of a person's relationships, built for
//! a directory that grows large: **concept-aware search** (a query token expands across a concept cluster,
//! so "doctor" finds a "clinician") + a **faceted sidebar** (Category / Kind / Source / Verification /
//! Agreements, each with live drill-down counts).
//!
//! Hosts the addressbook (Parties joined across the directory-actor + chat-contact stores by DID) and a
//! per-entry slot for the **agreements** governing that relationship. Backend:
//! `qualia_client_core::api::search_directory` + friends (see
//! `docs/plans/rights-aware-peer-agreement-addressbook.md`). Agreement wiring lands with the agreement
//! store (plan P1); until then each entry shows its agreement slot honestly as empty.

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if !crate::endpoints::is_native_host() {
        return Err("The desktop host is unavailable in this preview.".to_string());
    }
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

const PANEL: &str = "background: #1f2937; padding: 14px; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.3);";
const INPUT: &str = "width: 100%; box-sizing: border-box; padding: 8px 10px; background: #111827; color: #f3f4f6; border: 1px solid #374151; border-radius: 8px; font-family: inherit;";
const BTN: &str = "background: #8b5cf6; color: white; padding: 7px 14px; border: none; border-radius: 8px; font-weight: 600; cursor: pointer;";
const CHIP: &str = "display: inline-block; font-size: 11px; padding: 2px 8px; border-radius: 999px; background: #0f172a; color: #a5b4fc; margin: 2px 4px 2px 0; border: 1px solid #334155;";

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}
fn arr(v: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    v.get(key)
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
}
fn strs(v: &serde_json::Value, key: &str) -> Vec<String> {
    arr(v, key)
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .collect()
}

#[component]
pub fn DirectoryPane() -> Element {
    let query = use_signal(String::new);
    // Selected facets: a JSON object { facet_id: [values] }, passed straight to search_directory.
    let facets = use_signal(|| serde_json::json!({}));
    let result = use_signal(|| serde_json::Value::Null);
    let new_cat = use_signal(String::new);
    let status = use_signal(String::new);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (&query, &facets, &result, &new_cat, &status);
    }

    // Initial search on mount (empty query, no facets → the whole directory + all facet counts).
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let (mut result, mut status) = (result, status);
            spawn(async move {
                match invoke_json::<serde_json::Value>(
                    "search_directory",
                    json!({ "query": "", "facetsJson": "" }),
                )
                .await
                {
                    Ok(v) => result.set(v),
                    Err(e) => status.set(format!("Load directory failed: {e}")),
                }
            });
        }
    });

    let r = result();
    let facet_defs = arr(&r, "facets");
    let entries = arr(&r, "entries");
    let categories = arr(&r, "categories");
    let total = r.get("total").and_then(|x| x.as_u64()).unwrap_or(0);

    rsx! {
        div { style: "padding: 18px; background: #111827; color: #f3f4f6; height: 100%; box-sizing: border-box; overflow-y: auto;",
            div { style: "max-width: 1000px; margin: 0 auto;",
                h2 { style: "color: #a78bfa; margin: 0 0 4px; font-size: 24px;", "Directory" }
                p { style: "color: #9ca3af; margin: 0 0 12px; font-size: 13px;",
                    "Your people, organisations and agents — searchable by meaning, filterable by facet, each carrying the agreements that define your relationship. The terms are between you and them; no platform in the middle."
                }

                // ── Search box ───────────────────────────────────────────────
                input {
                    style: "{INPUT} margin-bottom: 6px; font-size: 15px;",
                    placeholder: "Search — try \"doctor\", a name, an organisation…", value: "{query}",
                    oninput: move |e| {
                        let mut q = query; q.set(e.value());
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (query, facets, mut result, mut status) = (query, facets, result, status);
                            spawn(async move {
                                let fj = serde_json::to_string(&facets()).unwrap_or_else(|_| "{}".into());
                                match invoke_json::<serde_json::Value>("search_directory", json!({ "query": query(), "facetsJson": fj })).await {
                                    Ok(v) => result.set(v),
                                    Err(e) => status.set(format!("Search failed: {e}")),
                                }
                            });
                        }
                    }
                }
                div { style: "color: #6b7280; font-size: 12px; margin-bottom: 12px;", "{total} result(s)" }
                if !status().is_empty() {
                    div { style: "background: #3b0b0b; border: 1px solid #ef4444; color: #fecaca; padding: 8px 12px; border-radius: 8px; margin-bottom: 12px; font-size: 13px;", "{status}" }
                }

                div { style: "display: grid; grid-template-columns: 240px 1fr; gap: 16px; align-items: start;",

                    // ── Facets ──────────────────────────────────────────────
                    div { style: "{PANEL}",
                        for facet in facet_defs.clone() {
                            {
                                let fid = s(&facet, "id");
                                let flabel = s(&facet, "label");
                                let values = arr(&facet, "values");
                                rsx! {
                                    div { style: "margin-bottom: 12px;",
                                        div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 6px;", "{flabel}" }
                                        for val in values {
                                            {
                                                let value = s(&val, "value");
                                                let vlabel = s(&val, "label");
                                                let count = val.get("count").and_then(|x| x.as_u64()).unwrap_or(0);
                                                let is_sel = val.get("selected").and_then(|x| x.as_bool()).unwrap_or(false);
                                                let fid2 = fid.clone();
                                                let value2 = value.clone();
                                                rsx! {
                                                    div {
                                                        style: if is_sel { "display: flex; justify-content: space-between; align-items: center; padding: 4px 8px; margin-bottom: 3px; border-radius: 6px; cursor: pointer; background: rgba(139,92,246,0.18); color: #ddd6fe; font-size: 13px;" } else { "display: flex; justify-content: space-between; align-items: center; padding: 4px 8px; margin-bottom: 3px; border-radius: 6px; cursor: pointer; color: #cbd5e1; font-size: 13px;" },
                                                        onclick: move |_| {
                                                            // Toggle this facet value, then re-search.
                                                            let mut obj = facets().as_object().cloned().unwrap_or_default();
                                                            let list = obj.entry(fid2.clone()).or_insert(serde_json::json!([]));
                                                            let mut vals: Vec<String> = list.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
                                                            if let Some(pos) = vals.iter().position(|x| x == &value2) { vals.remove(pos); } else { vals.push(value2.clone()); }
                                                            *list = serde_json::json!(vals);
                                                            let mut f = facets; f.set(serde_json::Value::Object(obj));
                                                            #[cfg(target_arch = "wasm32")]
                                                            {
                                                                let (query, facets, mut result, mut status) = (query, facets, result, status);
                                                                spawn(async move {
                                                                    let fj = serde_json::to_string(&facets()).unwrap_or_else(|_| "{}".into());
                                                                    match invoke_json::<serde_json::Value>("search_directory", json!({ "query": query(), "facetsJson": fj })).await {
                                                                        Ok(v) => result.set(v),
                                                                        Err(e) => status.set(format!("Search failed: {e}")),
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        span {
                                                            if is_sel { "✓ " } else { "" }
                                                            "{vlabel}"
                                                        }
                                                        span { style: "color: #6b7280; font-size: 11px;", "{count}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { style: "border-top: 1px solid #374151; padding-top: 10px;",
                            input {
                                style: "{INPUT} margin-bottom: 6px; font-size: 12px;", placeholder: "New category", value: "{new_cat}",
                                oninput: move |e| { let mut n = new_cat; n.set(e.value()); }
                            }
                            button {
                                style: "{BTN} width: 100%; font-size: 12px;",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (new_cat, query, facets, mut result, mut status) = (new_cat, query, facets, result, status);
                                        spawn(async move {
                                            let label = new_cat().trim().to_string();
                                            if label.is_empty() { return; }
                                            match invoke_json::<serde_json::Value>("create_directory_category", json!({ "label": label })).await {
                                                Ok(_) => {
                                                    let mut nc = new_cat; nc.set(String::new());
                                                    let fj = serde_json::to_string(&facets()).unwrap_or_else(|_| "{}".into());
                                                    if let Ok(v) = invoke_json::<serde_json::Value>("search_directory", json!({ "query": query(), "facetsJson": fj })).await { result.set(v); }
                                                }
                                                Err(e) => status.set(format!("Create category failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                "＋ Add category"
                            }
                        }
                    }

                    // ── Entries ─────────────────────────────────────────────
                    div {
                        if entries.is_empty() {
                            div { style: "{PANEL} color: #6b7280; font-size: 13px;",
                                "Nothing matches. Clear the search/facets, or connect with someone (Connect & Chat → generate/accept an invite) and they'll appear here."
                            }
                        }
                        for entry in entries.clone() {
                            {
                                let did = s(&entry, "did");
                                let name = s(&entry, "display_name");
                                let org = s(&entry, "organization");
                                let kinds = strs(&entry, "kinds");
                                let sources = strs(&entry, "sources");
                                let cats = strs(&entry, "categories");
                                let agreements = strs(&entry, "agreement_ids");
                                let all_cats = categories.clone();
                                #[cfg(target_arch = "wasm32")]
                                let did_add = did.clone();
                                #[cfg(target_arch = "wasm32")]
                                let cats_add = cats.clone();
                                rsx! {
                                    div { style: "{PANEL} margin-bottom: 10px;",
                                        div { style: "display: flex; justify-content: space-between; align-items: baseline; gap: 10px;",
                                            div {
                                                span { style: "font-weight: 700; color: #f3f4f6; font-size: 15px;", "{name}" }
                                                if !org.is_empty() {
                                                    span { style: "color: #9ca3af; font-size: 12px; margin-left: 8px;", "· {org}" }
                                                }
                                            }
                                            span { style: "color: #4b5563; font-size: 10px;", "{sources.join(\" · \")}" }
                                        }
                                        div { style: "color: #6b7280; font-size: 11px; font-family: monospace; margin: 3px 0 6px; word-break: break-all;", "{did}" }
                                        if !kinds.is_empty() {
                                            div { style: "margin-bottom: 4px;",
                                                for k in kinds.clone() { span { style: "{CHIP} color: #7dd3fc;", "{k}" } }
                                            }
                                        }
                                        div { style: "margin-bottom: 6px;",
                                            for c in cats.clone() {
                                                {
                                                    let label = all_cats.iter().find(|x| s(x, "id") == c).map(|x| s(x, "label")).unwrap_or_else(|| c.clone());
                                                    #[cfg(target_arch = "wasm32")]
                                                    let did_rm = did.clone();
                                                    #[cfg(target_arch = "wasm32")]
                                                    let remaining: Vec<String> = cats.iter().filter(|x| **x != c).cloned().collect();
                                                    rsx! {
                                                        span { style: "{CHIP}",
                                                            "{label} "
                                                            button {
                                                                style: "background: none; border: none; color: #f87171; cursor: pointer; font-size: 11px; padding: 0 0 0 2px;",
                                                                onclick: move |_| {
                                                                    #[cfg(target_arch = "wasm32")]
                                                                    {
                                                                        let (did_rm, remaining, query, facets, mut result) = (did_rm.clone(), remaining.clone(), query, facets, result);
                                                                        spawn(async move {
                                                                            let _ = invoke_json::<serde_json::Value>("set_directory_entry_categories", json!({ "did": did_rm, "categories": remaining })).await;
                                                                            let fj = serde_json::to_string(&facets()).unwrap_or_else(|_| "{}".into());
                                                                            if let Ok(v) = invoke_json::<serde_json::Value>("search_directory", json!({ "query": query(), "facetsJson": fj })).await { result.set(v); }
                                                                        });
                                                                    }
                                                                },
                                                                "✕"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            select {
                                                style: "background: #0f172a; color: #cbd5e1; border: 1px solid #334155; border-radius: 8px; padding: 3px 6px; font-size: 11px; margin-left: 4px;",
                                                onchange: move |e| {
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    let _ = &e;
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        let chosen = e.value();
                                                        if chosen.is_empty() { return; }
                                                        let (did_add, cats_add, query, facets, mut result) = (did_add.clone(), cats_add.clone(), query, facets, result);
                                                        spawn(async move {
                                                            let mut next = cats_add.clone();
                                                            if !next.contains(&chosen) { next.push(chosen); }
                                                            let _ = invoke_json::<serde_json::Value>("set_directory_entry_categories", json!({ "did": did_add, "categories": next })).await;
                                                            let fj = serde_json::to_string(&facets()).unwrap_or_else(|_| "{}".into());
                                                            if let Ok(v) = invoke_json::<serde_json::Value>("search_directory", json!({ "query": query(), "facetsJson": fj })).await { result.set(v); }
                                                        });
                                                    }
                                                },
                                                option { value: "", "＋ category…" }
                                                for c in all_cats.clone() {
                                                    option { value: "{s(&c, \"id\")}", "{s(&c, \"label\")}" }
                                                }
                                            }
                                        }
                                        div { style: "border-top: 1px solid #374151; padding-top: 6px; margin-top: 4px; font-size: 12px; color: #9ca3af;",
                                            if agreements.is_empty() {
                                                span { "Agreements: none yet — the terms of this relationship will live here." }
                                            } else {
                                                span { "Agreements: {agreements.len()}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
