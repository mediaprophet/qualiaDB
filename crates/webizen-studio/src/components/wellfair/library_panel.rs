//! **Library** panel — the personal hypermedia asset library. Ingest a document (it's *processed* to derive
//! topics + a searchable text representation), then find your files **by meaning** — topic, depiction, place,
//! project, purpose — never by folder. A flagged ingest under a guardianship relation notifies the guardian.
//!
//! Three views over the same library of meaning:
//! - **List** — everything / a facet search.
//! - **Timeline** — assets with a date, in time order (a photo's EXIF capture time, or a date you attach).
//! - **Map** — assets with coordinates, plotted (a photo's GPS, or a place you attach).
//!
//! Ingest derives what it can from the content (a text doc → topics; a photo → EXIF time/GPS; a WAV →
//! duration/pitch). You can also **author facets yourself** — a date, a place, a project, a purpose — because
//! the software provides the means; the person defines the meaning.

use super::host_client::{
    ingest_document, ingest_file_hex, list_library, search_library, search_library_time, IngestFacets,
};
use dioxus::prelude::*;
use crate::Route;

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn arr_join(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}
fn i64_field(v: &serde_json::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64())
}
fn f64_field(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

// ── date helpers (no chrono dependency) ──────────────────────────────────────

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - (m <= 2) as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + (m <= 2) as i64, m, d)
}

/// Parse `YYYY-MM-DD` (optionally with a `HH:MM` time) to unix seconds (UTC).
fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (date, time) = match s.split_once(&['T', ' '][..]) {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut dp = date.split('-');
    let y: i64 = dp.next()?.trim().parse().ok()?;
    let mo: i64 = dp.next()?.trim().parse().ok()?;
    let d: i64 = dp.next()?.trim().parse().ok()?;
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let (mut h, mut mi) = (0i64, 0i64);
    if let Some(t) = time {
        let mut tp = t.split(':');
        h = tp.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
        mi = tp.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    }
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60)
}

/// Format unix seconds as `YYYY-MM-DD`.
fn fmt_date(unix: i64) -> String {
    let (y, m, d) = civil_from_days(unix.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Parse a `lat,lon` string to a coordinate pair.
fn parse_latlon(s: &str) -> Option<(f32, f32)> {
    let (a, b) = s.trim().split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// Hex-encode bytes (for the binary ingest path).
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[component]
pub fn WellfairLibraryPanel() -> Element {
    let mut results = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);
    let mut view = use_signal(|| "list".to_string());

    let mut ing_uri = use_signal(|| "urn:doc:my-note".to_string());
    let mut ing_media = use_signal(|| "text/markdown".to_string());
    let mut ing_text = use_signal(|| "Notes: the liver is an organ; keep this receipt for the tax deduction.".to_string());
    let mut ing_binary = use_signal(|| false);
    let mut ing_guardian = use_signal(String::new);
    let mut ing_sensitivity = use_signal(|| "public".to_string());
    // person-authored facets
    let mut ing_date = use_signal(String::new);
    let mut ing_place = use_signal(String::new); // "lat,lon"
    let mut ing_place_label = use_signal(String::new);
    let mut ing_project = use_signal(String::new);
    let mut ing_purpose = use_signal(String::new);

    let mut facet = use_signal(|| "topic".to_string());
    let mut value = use_signal(|| "biology".to_string());
    let mut tl_from = use_signal(String::new);
    let mut tl_to = use_signal(String::new);

    let reload = move || {
        spawn(async move {
            if let Ok(serde_json::Value::Array(rows)) = list_library().await {
                results.set(rows);
            }
        });
    };
    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    let do_ingest = move |_| {
        let (uri, media, text, binary, g, sensitivity) =
            (ing_uri(), ing_media(), ing_text(), ing_binary(), ing_guardian(), ing_sensitivity());
        let (date, place, place_label, project, purpose) =
            (ing_date(), ing_place(), ing_place_label(), ing_project(), ing_purpose());
        spawn(async move {
            let guardian = if g.trim().is_empty() { None } else { Some(g) };
            let res = if binary {
                // The "text" field carries hex bytes of a photo/audio; a photo's EXIF auto-populates
                // the timeline + map. (Native file-picker → bytes is the remaining UI affordance.)
                let hex = if text.trim().chars().all(|c| c.is_ascii_hexdigit() || c.is_whitespace()) {
                    text.split_whitespace().collect::<String>()
                } else {
                    to_hex(text.as_bytes())
                };
                ingest_file_hex(&uri, &media, &hex, &uri, guardian, &sensitivity).await
            } else {
                let (lat, lon) = parse_latlon(&place).map(|(a, b)| (Some(a), Some(b))).unwrap_or((None, None));
                let facets = IngestFacets {
                    occurred_at: parse_date(&date),
                    place_label: if place_label.trim().is_empty() { None } else { Some(place_label) },
                    lat,
                    lon,
                    project: if project.trim().is_empty() { None } else { Some(project) },
                    purpose: if purpose.trim().is_empty() { None } else { Some(purpose) },
                };
                ingest_document(&uri, &media, &text, guardian, &facets, &sensitivity).await
            };
            match res {
                Ok(v) => {
                    let topics = arr_join(&v, "topics");
                    let n_flags = v.get("flags").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
                    let n_notif = v.get("guardian_notifications").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
                    let placed = v.get("occurred_at").and_then(|x| x.as_i64()).map(|t| format!(" · dated {}", fmt_date(t))).unwrap_or_default();
                    let mapped = v.get("lat").and_then(|x| x.as_f64()).map(|_| " · on the map".to_string()).unwrap_or_default();
                    status.set(format!(
                        "Ingested {uri} — topics: [{topics}]{placed}{mapped}{}{}",
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
    let do_timeline_search = move |_| {
        let (from, to) = (tl_from(), tl_to());
        spawn(async move {
            let start = parse_date(&from).unwrap_or(i64::MIN / 2);
            let end = parse_date(&to).unwrap_or(i64::MAX / 2);
            match search_library_time(start, end).await {
                Ok(serde_json::Value::Array(rows)) => {
                    let n = rows.len();
                    results.set(rows);
                    view.set("timeline".to_string());
                    status.set(format!("{n} dated asset(s) in range."));
                }
                Ok(_) => results.set(Vec::new()),
                Err(e) => status.set(format!("Timeline search failed: {e}")),
            }
        });
    };

    let field_style = "padding:0.3rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;background:var(--qualia-surface-2,#fff);";
    let label_style = "display:flex;flex-direction:column;gap:0.15rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);";

    // ── the active view's result rendering ──
    let rows: Vec<serde_json::Value> = results.read().clone();
    let body = match view().as_str() {
        "timeline" => rsx! { TimelineView { rows: rows.clone() } },
        "map" => rsx! { MapView { rows: rows.clone() } },
        _ => rsx! { ListView { rows: rows.clone() } },
    };

    let tab_style = |active: bool| {
        if active {
            "padding:0.25rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.75rem;cursor:pointer;"
        } else {
            "padding:0.25rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;"
        }
    };

    rsx! {
        section {
            aria_label: "WellFair hypermedia library",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);display:flex;flex-direction:column;gap:0.8rem;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;flex-wrap:wrap;",
                h2 { style: "margin:0;font-size:1rem;", "Library — find your files by meaning" }
                div {
                    style: "display:flex;gap:0.3rem;",
                    button { style: "{tab_style(view() == \"list\")}", onclick: move |_| view.set("list".to_string()), "List" }
                    button { style: "{tab_style(view() == \"timeline\")}", onclick: move |_| view.set("timeline".to_string()), "Timeline" }
                    button { style: "{tab_style(view() == \"map\")}", onclick: move |_| view.set("map".to_string()), "Map" }
                    button {
                        style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                        onclick: move |_| reload(),
                        "Show all"
                    }
                }
            }
            p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                "Ingest an asset — it's processed to derive its meaning (a doc → topics; a photo → its EXIF date + GPS; a WAV → duration + pitch). Attach a date or place yourself to put anything on the timeline or map. Not a folder of files; a graph of meaning."
            }

            // ── Operational Contexts (Quick Links) ──
            div {
                style: "display:flex; gap:0.5rem; flex-wrap:wrap; padding: 0.6rem 0; border-top: 1px dashed var(--qualia-border,#eee); border-bottom: 1px dashed var(--qualia-border,#eee);",
                Link { to: Route::SanctuaryRoute {}, style: "text-decoration:none; display:flex; align-items:center; gap:0.3rem; padding:0.3rem 0.6rem; border-radius:6px; background:var(--qualia-surface-2,#fff); border:1px solid var(--qualia-border,#ddd); font-size:0.75rem; color:var(--qualia-text,#333);",
                    sl-icon { "name": "safe" }
                    "Secure Enclave"
                }
                Link { to: Route::WorkRoute {}, style: "text-decoration:none; display:flex; align-items:center; gap:0.3rem; padding:0.3rem 0.6rem; border-radius:6px; background:var(--qualia-surface-2,#fff); border:1px solid var(--qualia-border,#ddd); font-size:0.75rem; color:var(--qualia-text,#333);",
                    sl-icon { "name": "wallet2" }
                    "Wallet & Finance"
                }
                Link { to: Route::WorkRoute {}, style: "text-decoration:none; display:flex; align-items:center; gap:0.3rem; padding:0.3rem 0.6rem; border-radius:6px; background:var(--qualia-surface-2,#fff); border:1px solid var(--qualia-border,#ddd); font-size:0.75rem; color:var(--qualia-text,#333);",
                    sl-icon { "name": "diagram-3" }
                    "Cooperative Projects"
                }
                Link { to: Route::IdentityRoute {}, style: "text-decoration:none; display:flex; align-items:center; gap:0.3rem; padding:0.3rem 0.6rem; border-radius:6px; background:var(--qualia-surface-2,#fff); border:1px solid var(--qualia-border,#ddd); font-size:0.75rem; color:var(--qualia-text,#333);",
                    sl-icon { "name": "journal-bookmark-fill" }
                    "Social Directory"
                }
            }

            if !status().is_empty() {
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-accent,#2b6);", "{status()}" }
            }

            // ── Ingest ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.5rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.8rem;font-weight:600;", "Ingest an asset" }
                div {
                    style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    label { style: "{label_style}flex:1;min-width:12rem;", "Asset id (uri)"
                        input { style: "{field_style}", value: "{ing_uri}", oninput: move |e| ing_uri.set(e.value()) } }
                    label { style: "{label_style}", "Type"
                        select {
                            style: "{field_style}",
                            value: "{ing_media}",
                            oninput: move |e| ing_media.set(e.value()),
                            option { value: "text/markdown", "text" }
                            option { value: "image/jpeg", "image/jpeg" }
                            option { value: "image/png", "image/png" }
                            option { value: "audio/wav", "audio/wav" }
                        }
                    }
                    label { style: "{label_style}", "Sensitivity (Sanctuary Vault)"
                        select {
                            style: "{field_style}",
                            value: "{ing_sensitivity}",
                            oninput: move |e| ing_sensitivity.set(e.value()),
                            option { value: "public", "Public (Cleartext)" }
                            option { value: "restricted", "Restricted (Encrypted Enclave)" }
                            option { value: "classified", "Classified (M-of-N Guardianship)" }
                        }
                    }
                }
                label { style: "{label_style}",
                    if ing_binary() { "Bytes (hex — a photo/audio file's contents)" } else { "Text" }
                    textarea { style: "{field_style}min-height:3rem;font-family:inherit;", value: "{ing_text}", oninput: move |e| ing_text.set(e.value()) } }
                label { style: "display:flex;gap:0.35rem;align-items:center;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                    input { r#type: "checkbox", checked: ing_binary(), onchange: move |e| ing_binary.set(e.checked()) }
                    "Binary asset — the field above is hex bytes (a photo's EXIF date/GPS auto-fill the timeline & map)"
                }
                // person-authored facets (the "means, not definition" path)
                div {
                    style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    label { style: "{label_style}", "Date (YYYY-MM-DD) → timeline"
                        input { style: "{field_style}", placeholder: "2025-04-01", value: "{ing_date}", oninput: move |e| ing_date.set(e.value()) } }
                    label { style: "{label_style}", "Place (lat,lon) → map"
                        input { style: "{field_style}", placeholder: "-33.87,151.21", value: "{ing_place}", oninput: move |e| ing_place.set(e.value()) } }
                    label { style: "{label_style}", "Place label"
                        input { style: "{field_style}", value: "{ing_place_label}", oninput: move |e| ing_place_label.set(e.value()) } }
                }
                div {
                    style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    label { style: "{label_style}", "Project"
                        input { style: "{field_style}", placeholder: "house-move-2025", value: "{ing_project}", oninput: move |e| ing_project.set(e.value()) } }
                    label { style: "{label_style}", "Purpose"
                        input { style: "{field_style}", placeholder: "tax-return-2025", value: "{ing_purpose}", oninput: move |e| ing_purpose.set(e.value()) } }
                    label { style: "{label_style}flex:1;min-width:10rem;", "Guardian DID (optional — flagged ingest notifies them)"
                        input { style: "{field_style}", value: "{ing_guardian}", oninput: move |e| ing_guardian.set(e.value()) } }
                }
                button {
                    style: "align-self:flex-start;padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.78rem;cursor:pointer;",
                    onclick: do_ingest,
                    "Ingest"
                }
            }

            // ── Search (facet + timeline range) ──
            div {
                style: "display:flex;gap:0.6rem;align-items:flex-end;flex-wrap:wrap;",
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
                span { style: "width:1px;height:1.5rem;background:var(--qualia-border,#ddd);" }
                label { style: "{label_style}", "From"
                    input { style: "{field_style}", placeholder: "2025-01-01", value: "{tl_from}", oninput: move |e| tl_from.set(e.value()) } }
                label { style: "{label_style}", "To"
                    input { style: "{field_style}", placeholder: "2025-12-31", value: "{tl_to}", oninput: move |e| tl_to.set(e.value()) } }
                button {
                    style: "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                    onclick: do_timeline_search,
                    "On timeline"
                }
            }

            {body}
        }
    }
}

/// The plain list view (any asset).
#[component]
fn ListView(rows: Vec<serde_json::Value>) -> Element {
    if rows.is_empty() {
        return rsx! { p { style: "font-size:0.75rem;color:var(--qualia-text-muted,#888);", "Nothing yet — ingest an asset above." } };
    }
    rsx! {
        ul {
            style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
            for r in rows {
                li {
                    key: "{str_field(&r, \"asset_uri\")}",
                    style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.74rem;display:flex;flex-direction:column;gap:0.15rem;",
                    strong { "{str_field(&r, \"asset_uri\")}" }
                    if !arr_join(&r, "topics").is_empty() {
                        div { style: "font-size:0.68rem;color:var(--qualia-accent,#2b6);", "topics: {arr_join(&r, \"topics\")}" }
                    }
                    if let Some(t) = i64_field(&r, "occurred_at") {
                        div { style: "font-size:0.68rem;color:var(--qualia-text-muted,#777);", "date: {fmt_date(t)}" }
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

/// The timeline view — dated assets in chronological order.
#[component]
fn TimelineView(rows: Vec<serde_json::Value>) -> Element {
    let mut dated: Vec<(i64, serde_json::Value)> = rows
        .into_iter()
        .filter_map(|r| i64_field(&r, "occurred_at").map(|t| (t, r)))
        .collect();
    dated.sort_by_key(|(t, _)| *t);
    if dated.is_empty() {
        return rsx! { p { style: "font-size:0.75rem;color:var(--qualia-text-muted,#888);", "No dated assets. Attach a date when you ingest (or ingest a photo with EXIF), then they appear here in order." } };
    }
    rsx! {
        ol {
            style: "margin:0;padding:0 0 0 0;list-style:none;display:flex;flex-direction:column;gap:0;border-left:2px solid var(--qualia-accent,#2b6);",
            for (t, r) in dated {
                li {
                    key: "{str_field(&r, \"asset_uri\")}",
                    style: "position:relative;padding:0.35rem 0.6rem 0.6rem 0.9rem;",
                    span { style: "position:absolute;left:-6px;top:0.5rem;width:9px;height:9px;border-radius:50%;background:var(--qualia-accent,#2b6);" }
                    div { style: "font-size:0.72rem;font-weight:600;color:var(--qualia-accent,#2b6);", "{fmt_date(t)}" }
                    div { style: "font-size:0.74rem;", "{str_field(&r, \"asset_uri\")}" }
                    if !arr_join(&r, "topics").is_empty() {
                        div { style: "font-size:0.66rem;color:var(--qualia-text-muted,#777);", "{arr_join(&r, \"topics\")}" }
                    }
                    div { style: "font-size:0.68rem;color:var(--qualia-text-muted,#888);", "{str_field(&r, \"excerpt\")}" }
                }
            }
        }
    }
}

/// The map view — assets with coordinates on an equirectangular world plot.
#[component]
fn MapView(rows: Vec<serde_json::Value>) -> Element {
    let placed: Vec<(f64, f64, serde_json::Value)> = rows
        .into_iter()
        .filter_map(|r| match (f64_field(&r, "lat"), f64_field(&r, "lon")) {
            (Some(lat), Some(lon)) => Some((lat, lon, r)),
            _ => None,
        })
        .collect();
    if placed.is_empty() {
        return rsx! { p { style: "font-size:0.75rem;color:var(--qualia-text-muted,#888);", "No located assets. Attach a place (lat,lon) when you ingest (or ingest a geotagged photo), then they appear on the map." } };
    }
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:0.5rem;",
            svg {
                view_box: "0 0 360 180",
                width: "100%",
                style: "background:var(--qualia-surface-2,#eef3f6);border:1px solid var(--qualia-border,#cdd);border-radius:8px;max-height:360px;",
                // graticule
                for gx in [60, 120, 180, 240, 300] {
                    line { x1: "{gx}", y1: "0", x2: "{gx}", y2: "180", stroke: "#c3d0d6", stroke_width: "0.4" }
                }
                for gy in [45, 90, 135] {
                    line { x1: "0", y1: "{gy}", x2: "360", y2: "{gy}", stroke: "#c3d0d6", stroke_width: "0.4" }
                }
                for (lat, lon, r) in placed.clone() {
                    circle {
                        cx: "{lon + 180.0}",
                        cy: "{90.0 - lat}",
                        r: "2.4",
                        fill: "var(--qualia-accent,#2b6)",
                        fill_opacity: "0.8",
                        stroke: "#fff",
                        stroke_width: "0.5",
                        title { "{str_field(&r, \"asset_uri\")} ({lat:.3},{lon:.3})" }
                    }
                }
            }
            ul {
                style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.25rem;",
                for (lat, lon, r) in placed {
                    li {
                        key: "{str_field(&r, \"asset_uri\")}",
                        style: "font-size:0.72rem;display:flex;gap:0.5rem;",
                        span { style: "color:var(--qualia-accent,#2b6);font-variant-numeric:tabular-nums;", "{lat:.3},{lon:.3}" }
                        span { "{str_field(&r, \"asset_uri\")}" }
                    }
                }
            }
        }
    }
}
