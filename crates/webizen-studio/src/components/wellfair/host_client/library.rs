//! Hypermedia asset library

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

/// The person-authored facets attached at ingest — an optional date (timeline), place (map), project and
/// purpose. All `None`/empty ⇒ ingest derives what it can from the content alone.
#[derive(Debug, Clone, Default)]
pub struct IngestFacets {
    pub occurred_at: Option<i64>,
    pub place_label: Option<String>,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub project: Option<String>,
    pub purpose: Option<String>,
    pub sensitivity: Option<String>,
    pub section: Option<String>,
    pub commons_visibility: Option<String>,
}

/// Ingest a text document (derive topics + searchable text; guardianship flag→notify), optionally placing it
/// on the timeline/map via person-authored `facets`. Returns a summary.
#[cfg(target_arch = "wasm32")]
pub async fn ingest_document(
    uri: &str,
    media_type: &str,
    text: &str,
    guardian_did: Option<String>,
    facets: &IngestFacets,
    sensitivity: &str,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (k, v) in [
        ("uri", uri),
        ("mediaType", media_type),
        ("text", text),
        ("sensitivity", sensitivity),
    ] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v))
            .map_err(|_| "args".to_string())?;
    }
    if let Some(g) = guardian_did {
        js_sys::Reflect::set(
            &args,
            &"guardianDid".into(),
            &wasm_bindgen::JsValue::from_str(&g),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(t) = facets.occurred_at {
        js_sys::Reflect::set(
            &args,
            &"occurredAt".into(),
            &wasm_bindgen::JsValue::from_f64(t as f64),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(l) = &facets.place_label {
        js_sys::Reflect::set(
            &args,
            &"placeLabel".into(),
            &wasm_bindgen::JsValue::from_str(l),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(v) = facets.lat {
        js_sys::Reflect::set(
            &args,
            &"lat".into(),
            &wasm_bindgen::JsValue::from_f64(v as f64),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(v) = facets.lon {
        js_sys::Reflect::set(
            &args,
            &"lon".into(),
            &wasm_bindgen::JsValue::from_f64(v as f64),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(p) = &facets.project {
        js_sys::Reflect::set(
            &args,
            &"project".into(),
            &wasm_bindgen::JsValue::from_str(p),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(p) = &facets.purpose {
        js_sys::Reflect::set(
            &args,
            &"purpose".into(),
            &wasm_bindgen::JsValue::from_str(p),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(s) = &facets.sensitivity {
        js_sys::Reflect::set(
            &args,
            &"sensitivity".into(),
            &wasm_bindgen::JsValue::from_str(s),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(s) = &facets.section {
        js_sys::Reflect::set(
            &args,
            &"section".into(),
            &wasm_bindgen::JsValue::from_str(s),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(s) = &facets.commons_visibility {
        js_sys::Reflect::set(
            &args,
            &"commonsVisibility".into(),
            &wasm_bindgen::JsValue::from_str(s),
        )
        .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_document", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "ingest response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_document(
    _u: &str,
    _m: &str,
    _t: &str,
    _g: Option<String>,
    _f: &IngestFacets,
    _s: &str,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Ingest a binary asset (photo/audio) from hex-encoded bytes — a photo's EXIF time/GPS auto-populate the
/// timeline/map. Returns a summary.
#[cfg(target_arch = "wasm32")]
pub async fn ingest_file_hex(
    uri: &str,
    media_type: &str,
    bytes_hex: &str,
    caption: &str,
    guardian_did: Option<String>,
    sensitivity: &str,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    for (k, v) in [
        ("uri", uri),
        ("mediaType", media_type),
        ("bytesHex", bytes_hex),
        ("caption", caption),
        ("sensitivity", sensitivity),
    ] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v))
            .map_err(|_| "args".to_string())?;
    }
    if let Some(g) = guardian_did {
        js_sys::Reflect::set(
            &args,
            &"guardianDid".into(),
            &wasm_bindgen::JsValue::from_str(&g),
        )
        .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_file_hex", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "ingest response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_file_hex(
    _u: &str,
    _m: &str,
    _b: &str,
    _c: &str,
    _g: Option<String>,
    _s: &str,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Prefer vault-free `library_*` commands (storage path), then HostApi `wellfair_*`.
/// Parses either a JSON string or a direct object/array from Tauri.
#[cfg(target_arch = "wasm32")]
async fn invoke_library_json(
    vault_free: &str,
    vault_free_args: serde_json::Value,
    wellfair: &str,
    wellfair_args: js_sys::Object,
) -> Result<serde_json::Value, String> {
    use crate::components::qapp_engine::invoke_json;
    match invoke_json(vault_free, vault_free_args).await {
        Ok(v) => return Ok(normalize_library_json(v)),
        Err(e1) => {
            let js = tauri_invoke(wellfair, wellfair_args.into())
                .await
                .map_err(|e2| {
                    format!("Library read failed. {vault_free}: {e1}; {wellfair}: {e2:?}")
                })?;
            if let Some(s) = js.as_string() {
                return serde_json::from_str(&s).map_err(|e| e.to_string());
            }
            serde_wasm_bindgen::from_value(js).map_err(|e| format!("library response: {e}"))
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn normalize_library_json(v: serde_json::Value) -> serde_json::Value {
    if let Some(s) = v.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            return parsed;
        }
    }
    v
}

/// Search the library by facet (topic/depicts/place/project/purpose). Returns entry summaries.
#[cfg(target_arch = "wasm32")]
pub async fn search_library(facet: &str, value: &str) -> Result<serde_json::Value, String> {
    let wellfair_args = js_sys::Object::new();
    js_sys::Reflect::set(
        &wellfair_args,
        &"facet".into(),
        &wasm_bindgen::JsValue::from_str(facet),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &wellfair_args,
        &"value".into(),
        &wasm_bindgen::JsValue::from_str(value),
    )
    .map_err(|_| "args".to_string())?;
    invoke_library_json(
        "library_search",
        serde_json::json!({ "facet": facet, "value": value }),
        "wellfair_search_library",
        wellfair_args,
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn search_library(_f: &str, _v: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Everything in the library (newest first). Optional section filter.
#[cfg(target_arch = "wasm32")]
pub async fn list_library() -> Result<serde_json::Value, String> {
    list_library_section(None).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_library() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_library_section(section: Option<&str>) -> Result<serde_json::Value, String> {
    let wellfair_args = js_sys::Object::new();
    if let Some(s) = section {
        js_sys::Reflect::set(
            &wellfair_args,
            &"section".into(),
            &wasm_bindgen::JsValue::from_str(s),
        )
        .map_err(|_| "args".to_string())?;
    }
    let vf = match section {
        Some(s) => serde_json::json!({ "section": s }),
        None => serde_json::json!({}),
    };
    invoke_library_json("library_list", vf, "wellfair_list_library", wellfair_args).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_library_section(_s: Option<&str>) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_library_commons(
    asset_uri: &str,
    visibility: &str,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"assetUri".into(),
        &wasm_bindgen::JsValue::from_str(asset_uri),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"visibility".into(),
        &wasm_bindgen::JsValue::from_str(visibility),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_set_library_commons", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "commons response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_library_commons(_u: &str, _v: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn library_commons_share_card(asset_uri: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"assetUri".into(),
        &wasm_bindgen::JsValue::from_str(asset_uri),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_library_commons_share_card", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "share card not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn library_commons_share_card(_u: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// The timeline query — entries whose event instant falls within `[start, end]` (unix seconds).
#[cfg(target_arch = "wasm32")]
pub async fn search_library_time(start: i64, end: i64) -> Result<serde_json::Value, String> {
    let wellfair_args = js_sys::Object::new();
    js_sys::Reflect::set(
        &wellfair_args,
        &"start".into(),
        &wasm_bindgen::JsValue::from_f64(start as f64),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &wellfair_args,
        &"end".into(),
        &wasm_bindgen::JsValue::from_f64(end as f64),
    )
    .map_err(|_| "args".to_string())?;
    invoke_library_json(
        "library_search_time",
        serde_json::json!({ "start": start, "end": end }),
        "wellfair_search_library_time",
        wellfair_args,
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn search_library_time(_s: i64, _e: i64) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn search_library_text(query: &str) -> Result<serde_json::Value, String> {
    let wellfair_args = js_sys::Object::new();
    js_sys::Reflect::set(
        &wellfair_args,
        &"query".into(),
        &wasm_bindgen::JsValue::from_str(query),
    )
    .map_err(|_| "args".to_string())?;
    invoke_library_json(
        "library_search_text",
        serde_json::json!({ "query": query }),
        "wellfair_search_library_text",
        wellfair_args,
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn search_library_text(_q: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Multi-facet library query. `filter` is a JSON object matching FacetFilter;
/// `sort` is newest|oldest|title_asc|title_desc|media_type|category.
#[cfg(target_arch = "wasm32")]
pub async fn query_library_faceted(
    filter: &serde_json::Value,
    sort: &str,
) -> Result<serde_json::Value, String> {
    let filter_json = serde_json::to_string(filter).map_err(|e| e.to_string())?;
    let wellfair_args = js_sys::Object::new();
    js_sys::Reflect::set(
        &wellfair_args,
        &"filterJson".into(),
        &wasm_bindgen::JsValue::from_str(&filter_json),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &wellfair_args,
        &"sort".into(),
        &wasm_bindgen::JsValue::from_str(sort),
    )
    .map_err(|_| "args".to_string())?;
    invoke_library_json(
        "library_query_faceted",
        serde_json::json!({ "filterJson": filter_json, "sort": sort }),
        "wellfair_query_library_faceted",
        wellfair_args,
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn query_library_faceted(
    _filter: &serde_json::Value,
    _sort: &str,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn seed_studio_qapps() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_seed_studio_qapps", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "seed qapps not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn seed_studio_qapps() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

/// Seed perception models + ontologies + computer_vision library rows into Library.
/// Prefers `library_seed_perception_assets` (storage path; no vault required), then
/// falls back to `wellfair_seed_perception_library` (vault host).
#[cfg(target_arch = "wasm32")]
pub async fn seed_perception_library() -> Result<serde_json::Value, String> {
    use crate::components::qapp_engine::invoke_json;
    match invoke_json("library_seed_perception_assets", serde_json::json!({})).await {
        Ok(v) => return Ok(normalize_seed_report(v)),
        Err(e1) => {
            let js = match tauri_invoke(
                "wellfair_seed_perception_library",
                wasm_bindgen::JsValue::NULL,
            )
            .await
            {
                Ok(js) => js,
                Err(e2) => {
                    return Err(format!(
                        "Seed perception failed. library_seed_perception_assets: {e1}; wellfair_seed_perception_library: {e2:?}"
                    ));
                }
            };
            if let Some(s) = js.as_string() {
                let v: serde_json::Value =
                    serde_json::from_str(&s).map_err(|e| format!("seed report parse: {e}"))?;
                return Ok(normalize_seed_report(v));
            }
            match serde_wasm_bindgen::from_value::<serde_json::Value>(js) {
                Ok(v) => Ok(normalize_seed_report(v)),
                Err(e) => Err(format!(
                    "Seed perception: unexpected host response ({e}); assets path: {e1}"
                )),
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn seed_perception_library() -> Result<serde_json::Value, String> {
    Err("Perception library seed requires the Tauri desktop host".into())
}

/// Coerce string-wrapped or object host reports into a plain JSON object.
#[cfg(target_arch = "wasm32")]
fn normalize_seed_report(v: serde_json::Value) -> serde_json::Value {
    if let Some(s) = v.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            return parsed;
        }
    }
    v
}

#[cfg(target_arch = "wasm32")]
pub async fn ingest_legislation_text(
    text: &str,
    register_id: Option<&str>,
    jurisdiction: Option<&str>,
    title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"text".into(),
        &wasm_bindgen::JsValue::from_str(text),
    )
    .map_err(|_| "args".to_string())?;
    if let Some(id) = register_id {
        js_sys::Reflect::set(
            &args,
            &"registerId".into(),
            &wasm_bindgen::JsValue::from_str(id),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(j) = jurisdiction {
        js_sys::Reflect::set(
            &args,
            &"jurisdiction".into(),
            &wasm_bindgen::JsValue::from_str(j),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(t) = title_hint {
        js_sys::Reflect::set(
            &args,
            &"titleHint".into(),
            &wasm_bindgen::JsValue::from_str(t),
        )
        .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_legislation_text", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "legislation ingest not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_legislation_text(
    _text: &str,
    _register_id: Option<&str>,
    _jurisdiction: Option<&str>,
    _title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn ingest_legislation_pdf_hex(
    hex_bytes: &str,
    register_id: Option<&str>,
    jurisdiction: Option<&str>,
    title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"hexBytes".into(),
        &wasm_bindgen::JsValue::from_str(hex_bytes),
    )
    .map_err(|_| "args".to_string())?;
    if let Some(id) = register_id {
        js_sys::Reflect::set(
            &args,
            &"registerId".into(),
            &wasm_bindgen::JsValue::from_str(id),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(j) = jurisdiction {
        js_sys::Reflect::set(
            &args,
            &"jurisdiction".into(),
            &wasm_bindgen::JsValue::from_str(j),
        )
        .map_err(|_| "args".to_string())?;
    }
    if let Some(t) = title_hint {
        js_sys::Reflect::set(
            &args,
            &"titleHint".into(),
            &wasm_bindgen::JsValue::from_str(t),
        )
        .map_err(|_| "args".to_string())?;
    }
    let js = tauri_invoke("wellfair_ingest_legislation_pdf_hex", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "legislation pdf ingest not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_legislation_pdf_hex(
    _hex: &str,
    _register_id: Option<&str>,
    _jurisdiction: Option<&str>,
    _title_hint: Option<&str>,
) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_qapp_catalog_categories() -> Result<serde_json::Value, String> {
    let js = tauri_invoke(
        "wellfair_list_qapp_catalog_categories",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "qapp categories not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_qapp_catalog_categories() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn library_stats() -> Result<serde_json::Value, String> {
    let wellfair_args = js_sys::Object::new();
    invoke_library_json(
        "library_stats",
        serde_json::json!({}),
        "wellfair_library_stats",
        wellfair_args,
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn library_stats() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn remove_library_entry(asset_uri: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"assetUri".into(),
        &wasm_bindgen::JsValue::from_str(asset_uri),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_remove_library_entry", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "remove not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_library_entry(_u: &str) -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn export_library_graph() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_export_library_graph", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "export not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn export_library_graph() -> Result<serde_json::Value, String> {
    Err("The library requires the Tauri desktop host".into())
}
