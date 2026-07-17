//! **Library** — personal hypermedia knowledge shelf.
//!
//! Ingest notes, receipts, photos (meaning derived: topics, EXIF time/place, purpose/project you attach).
//! Browse by **List · Timeline · Map**, free-text or facet search. Not a folder tree — a graph of meaning.

use super::host_client::{
    export_library_graph, ingest_document, ingest_file_hex, library_commons_share_card,
    ingest_legislation_text, library_stats, list_library_section, query_library_faceted,
    remove_library_entry, search_library, search_library_time, seed_studio_qapps,
    set_library_commons, IngestFacets,
};
use crate::Route;
use dioxus::prelude::*;

const SECTIONS: &[(&str, &str, &str)] = &[
    ("all", "All", "Everything you can see"),
    ("secret", "Secret", "Sanctuary · private · classified"),
    ("wellfair", "Wellfair", "Health & welfare"),
    ("personal", "Personal", "Private life shelf"),
    ("work", "Work", "Projects & labour"),
    ("tools", "Tools", "Logs · telemetry · agent/tool output"),
    ("software", "Software", "QApps · websites · packages"),
    ("commons", "Commons", "Peers & permissive share"),
];

// ── styles (Talk-aligned dark product chrome) ────────────────────────────────

const ROOT: &str = "display:flex;flex-direction:column;height:100%;min-height:0;background:#0b1220;color:#e5e7eb;box-sizing:border-box;font-family:inherit;";
const HEADER: &str = "padding:1.1rem 1.35rem 0.85rem;border-bottom:1px solid #1f2937;background:linear-gradient(180deg,#111827 0%,#0b1220 100%);flex-shrink:0;";
const STATS: &str = "display:flex;flex-wrap:wrap;gap:0.45rem;margin-top:0.75rem;";
const STAT_CHIP: &str = "display:inline-flex;align-items:center;gap:0.35rem;padding:0.28rem 0.65rem;border-radius:999px;background:#0f172a;border:1px solid #334155;font-size:0.72rem;color:#94a3b8;";
const STAT_NUM: &str = "color:#a78bfa;font-weight:700;font-variant-numeric:tabular-nums;";
const BODY: &str = "flex:1;min-height:0;display:grid;grid-template-columns:minmax(280px,340px) 1fr;gap:0;";
const SIDE: &str = "border-right:1px solid #1f2937;overflow-y:auto;padding:1rem;background:#0f172a;min-height:0;";
const MAIN: &str = "overflow-y:auto;padding:1rem 1.25rem;min-height:0;";
const CARD: &str = "background:#111827;border:1px solid #1f2937;border-radius:14px;padding:0.95rem 1.05rem;margin-bottom:0.85rem;";
const H3: &str = "margin:0 0 0.4rem;font-size:0.82rem;font-weight:700;color:#c4b5fd;letter-spacing:0.04em;text-transform:uppercase;";
const MUTED: &str = "margin:0 0 0.75rem;font-size:0.78rem;color:#94a3b8;line-height:1.45;";
const INPUT: &str = "width:100%;box-sizing:border-box;padding:0.5rem 0.65rem;margin-bottom:0.45rem;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:9px;font-family:inherit;font-size:0.8rem;";
const LABEL: &str = "display:block;font-size:0.68rem;color:#64748b;margin:0 0 0.2rem;font-weight:600;";
const BTN: &str = "background:#8b5cf6;color:#fff;padding:0.5rem 0.9rem;border:none;border-radius:9px;font-weight:600;cursor:pointer;font-size:0.8rem;";
const BTN2: &str = "background:#1e293b;color:#e2e8f0;padding:0.45rem 0.75rem;border:1px solid #334155;border-radius:9px;font-weight:600;cursor:pointer;font-size:0.75rem;";
const TAB: &str = "padding:0.4rem 0.85rem;border-radius:9px;border:1px solid #334155;background:transparent;color:#94a3b8;font-size:0.78rem;font-weight:600;cursor:pointer;";
const TAB_ON: &str = "padding:0.4rem 0.85rem;border-radius:9px;border:1px solid #8b5cf6;background:rgba(139,92,246,0.18);color:#e9d5ff;font-size:0.78rem;font-weight:600;cursor:pointer;";
const TOPIC: &str = "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0.1rem 0;border-radius:999px;background:rgba(139,92,246,0.12);border:1px solid #4c1d95;color:#c4b5fd;font-size:0.68rem;cursor:pointer;";
const ENTRY: &str = "padding:0.85rem 1rem;border-radius:12px;border:1px solid #1f2937;background:#111827;margin-bottom:0.55rem;transition:border-color 0.15s;";
const STATUS_OK: &str = "padding:0.55rem 0.85rem;border-radius:10px;background:#052e1c;border:1px solid #10b981;color:#a7f3d0;font-size:0.78rem;margin-bottom:0.75rem;";
const STATUS_ERR: &str = "padding:0.55rem 0.85rem;border-radius:10px;background:#3b0b0b;border:1px solid #ef4444;color:#fecaca;font-size:0.78rem;margin-bottom:0.75rem;";

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}
fn arr_str(v: &serde_json::Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
fn i64_field(v: &serde_json::Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64())
}
fn f64_field(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}
fn u64_field(v: &serde_json::Value, key: &str) -> u64 {
    v.get(key).and_then(|x| x.as_u64()).unwrap_or(0)
}

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
fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (date, time) = match s.split_once(['T', ' ']) {
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
fn fmt_date(unix: i64) -> String {
    let (y, m, d) = civil_from_days(unix.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}
fn parse_latlon(s: &str) -> Option<(f32, f32)> {
    let (a, b) = s.trim().split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
fn display_title(uri: &str) -> String {
    let t = uri.rsplit(['/', ':']).next().unwrap_or(uri);
    if t.is_empty() {
        uri.to_string()
    } else {
        t.replace('-', " ").replace('_', " ")
    }
}
fn media_icon(media: &str) -> &'static str {
    if media.contains("qapp") || media.contains("webizen-qapp") {
        "▦"
    } else if media.starts_with("image/") {
        "🖼"
    } else if media.starts_with("audio/") {
        "🔊"
    } else if media.contains("html") || media.contains("website") {
        "🌐"
    } else {
        "📄"
    }
}

fn category_label(slug: &str) -> String {
    match slug {
        "social-sciences" => "Social Sciences".into(),
        "humanities" => "Humanities".into(),
        "natural-sciences" => "Natural Sciences".into(),
        "formal-sciences" => "Formal Sciences".into(),
        "area-studies" => "Area Studies".into(),
        "applied-liberal-arts" => "Applied Liberal Arts".into(),
        "emerging-interdisciplinary" => "Emerging Interdisciplinary".into(),
        "specialized-sciences" => "Specialized Sciences".into(),
        "pre-professional" => "Pre-Professional".into(),
        "language-regional" => "Language & Regional".into(),
        "arts-performance" => "Arts & Performance".into(),
        "advanced-subdisciplines" => "Advanced Sub-disciplines".into(),
        "niche-sciences" => "Niche Sciences".into(),
        "philosophy-theory" => "Philosophy & Theory".into(),
        "religious-studies" => "Religious Studies".into(),
        "literary-media" => "Literary & Media".into(),
        "linguistics-semiotics" => "Linguistics & Semiotics".into(),
        "intersectional-applied" => "Intersectional & Applied".into(),
        "historical-textual" => "Historical & Textual".into(),
        "critical-cultural" => "Critical & Cultural".into(),
        "interdisciplinary-stem" => "Interdisciplinary STEM".into(),
        "design-spatial" => "Design & Spatial".into(),
        "critical-theory" => "Critical Theory".into(),
        other => other.replace('-', " "),
    }
}

#[component]
pub fn WellfairLibraryPanel() -> Element {
    let mut results = use_signal(Vec::<serde_json::Value>::new);
    let mut stats = use_signal(|| serde_json::json!({}));
    let mut facet_counts = use_signal(|| serde_json::json!({}));
    let mut status = use_signal(String::new);
    let mut status_err = use_signal(|| false);
    let mut view = use_signal(|| "list".to_string());
    let mut show_ingest = use_signal(|| true);
    let mut q = use_signal(String::new);
    let mut section = use_signal(|| "all".to_string());
    let mut sort_mode = use_signal(|| "newest".to_string());
    let mut category = use_signal(String::new);
    let mut secret_unlocked = use_signal(|| false);
    let mut share_card = use_signal(String::new);

    let mut ing_uri = use_signal(|| {
        format!(
            "urn:doc:note-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() % 100_000)
                .unwrap_or(0)
        )
    });
    let mut ing_media = use_signal(|| "text/markdown".to_string());
    let mut ing_text = use_signal(String::new);
    let mut ing_binary = use_signal(|| false);
    let mut ing_guardian = use_signal(String::new);
    let mut ing_sensitivity = use_signal(|| "public".to_string());
    let mut ing_date = use_signal(String::new);
    let mut ing_place = use_signal(String::new);
    let mut ing_place_label = use_signal(String::new);
    let mut ing_project = use_signal(String::new);
    let mut ing_purpose = use_signal(String::new);
    let mut ing_section = use_signal(|| "personal".to_string());
    let mut ing_commons = use_signal(|| "none".to_string());
    let mut legis_text = use_signal(String::new);
    let mut legis_id = use_signal(String::new);
    let mut legis_title = use_signal(String::new);

    let mut facet = use_signal(|| "topic".to_string());
    let mut value = use_signal(String::new);
    let mut tl_from = use_signal(String::new);
    let mut tl_to = use_signal(String::new);

    let refresh_all = move || {
        let sec = section();
        let cat = category();
        let sort = sort_mode();
        let text = q();
        spawn(async move {
            if sec == "secret" && !secret_unlocked() {
                results.set(Vec::new());
                status_err.set(false);
                if let Ok(s) = library_stats().await {
                    stats.set(s);
                }
                return;
            }
            let mut filter = serde_json::Map::new();
            if sec != "all" {
                filter.insert("section".into(), serde_json::Value::String(sec.clone()));
            }
            if !cat.trim().is_empty() {
                filter.insert(
                    "categories".into(),
                    serde_json::json!([cat.trim()]),
                );
            }
            if !text.trim().is_empty() {
                filter.insert("text".into(), serde_json::Value::String(text));
            }
            let filter = serde_json::Value::Object(filter);
            match query_library_faceted(&filter, &sort).await {
                Ok(v) => {
                    let rows = v
                        .get("entries")
                        .and_then(|e| e.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let n = rows.len();
                    results.set(rows);
                    if let Some(f) = v.get("facets") {
                        facet_counts.set(f.clone());
                    }
                    status_err.set(false);
                    status.set(format!("{n} item(s) · sort {sort}."));
                }
                Err(e) => {
                    // Fallback to plain section list if faceted path unavailable.
                    let want = if sec == "all" { None } else { Some(sec.as_str()) };
                    match list_library_section(want).await {
                        Ok(serde_json::Value::Array(rows)) => {
                            results.set(rows);
                            status_err.set(false);
                        }
                        Ok(_) => results.set(Vec::new()),
                        Err(e2) => {
                            status_err.set(true);
                            status.set(format!(
                                "{e} / {e2} — unlock Sanctuary (Keep → Sanctuary) so the vault host can open the library."
                            ));
                        }
                    }
                }
            }
            if let Ok(s) = library_stats().await {
                stats.set(s);
            }
        });
    };

    let mut loaded = use_signal(|| false);
    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        refresh_all();
    });

    let do_ingest = move |_| {
        let (uri, media, text, binary, g, sensitivity) = (
            ing_uri(),
            ing_media(),
            ing_text(),
            ing_binary(),
            ing_guardian(),
            ing_sensitivity(),
        );
        let (date, place, place_label, project, purpose, sec, commons) = (
            ing_date(),
            ing_place(),
            ing_place_label(),
            ing_project(),
            ing_purpose(),
            ing_section(),
            ing_commons(),
        );
        spawn(async move {
            if text.trim().is_empty() {
                status_err.set(true);
                status.set("Add some content (or hex bytes for a photo) before ingesting.".into());
                return;
            }
            let guardian = if g.trim().is_empty() {
                None
            } else {
                Some(g)
            };
            // Section secret forces classified sensitivity.
            let sens = if sec == "secret" || sensitivity == "classified" || sensitivity == "restricted"
            {
                if sensitivity == "public" {
                    "classified".to_string()
                } else {
                    sensitivity.clone()
                }
            } else {
                sensitivity.clone()
            };
            let commons = if sec == "secret" || sens != "public" {
                "none".to_string()
            } else {
                commons
            };
            let res = if binary {
                let hex = if text
                    .trim()
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || c.is_whitespace())
                {
                    text.split_whitespace().collect::<String>()
                } else {
                    to_hex(text.as_bytes())
                };
                // Binary path still uses host sensitivity string where available.
                ingest_file_hex(&uri, &media, &hex, &uri, guardian, &sens).await
            } else {
                let (lat, lon) = parse_latlon(&place)
                    .map(|(a, b)| (Some(a), Some(b)))
                    .unwrap_or((None, None));
                let facets = IngestFacets {
                    occurred_at: parse_date(&date),
                    place_label: if place_label.trim().is_empty() {
                        None
                    } else {
                        Some(place_label)
                    },
                    lat,
                    lon,
                    project: if project.trim().is_empty() {
                        None
                    } else {
                        Some(project)
                    },
                    purpose: if purpose.trim().is_empty() {
                        None
                    } else {
                        Some(purpose)
                    },
                    sensitivity: Some(sens.clone()),
                    section: Some(sec.clone()),
                    commons_visibility: Some(commons),
                };
                ingest_document(&uri, &media, &text, guardian, &facets, &sens).await
            };
            match res {
                Ok(v) => {
                    let topics = arr_str(&v, "topics").join(", ");
                    let sect = str_field(&v, "section");
                    status_err.set(false);
                    status.set(format!(
                        "Saved to {sect} · topics [{topics}] — findable by meaning, not by folder."
                    ));
                    ing_text.set(String::new());
                    if !sect.is_empty() {
                        section.set(sect);
                    }
                    refresh_all();
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(format!("Ingest failed: {e}"));
                }
            }
        });
    };

    let do_quick_search = move |_| {
        // Faceted path respects section + category + sort + free text.
        refresh_all();
    };

    let do_facet_search = move |_| {
        let (f, v) = (facet(), value());
        spawn(async move {
            match search_library(&f, &v).await {
                Ok(serde_json::Value::Array(rows)) => {
                    let n = rows.len();
                    results.set(rows);
                    status_err.set(false);
                    status.set(format!("{n} · {f} = “{v}”"));
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(format!("Facet search failed: {e}"));
                }
                _ => results.set(Vec::new()),
            }
        });
    };

    let do_timeline = move |_| {
        let (from, to) = (tl_from(), tl_to());
        spawn(async move {
            let start = parse_date(&from).unwrap_or(i64::MIN / 2);
            let end = parse_date(&to).unwrap_or(i64::MAX / 2);
            match search_library_time(start, end).await {
                Ok(serde_json::Value::Array(rows)) => {
                    let n = rows.len();
                    results.set(rows);
                    view.set("timeline".into());
                    status_err.set(false);
                    status.set(format!("{n} dated item(s) in range."));
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(format!("Timeline failed: {e}"));
                }
                _ => results.set(Vec::new()),
            }
        });
    };

    let st = stats();
    let total = u64_field(&st, "total");
    let with_date = u64_field(&st, "with_date");
    let with_place = u64_field(&st, "with_place");
    let quins = u64_field(&st, "quins");
    let topic_map = st
        .get("topics")
        .and_then(|t| t.as_object())
        .cloned()
        .unwrap_or_default();
    let mut topic_list: Vec<(String, u64)> = topic_map
        .iter()
        .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n)))
        .collect();
    topic_list.sort_by(|a, b| b.1.cmp(&a.1));

    let rows = results();
    let body = match view().as_str() {
        "timeline" => rsx! { TimelineView { rows: rows.clone() } },
        "map" => rsx! { MapView { rows: rows.clone() } },
        _ => rsx! {
            ListView {
                rows: rows.clone(),
                on_topic: move |t: String| {
                    value.set(t.clone());
                    facet.set("topic".into());
                    spawn(async move {
                        if let Ok(serde_json::Value::Array(r)) = search_library("topic", &t).await {
                            results.set(r);
                        }
                    });
                },
                on_remove: move |uri: String| {
                    spawn(async move {
                        match remove_library_entry(&uri).await {
                            Ok(_) => {
                                status_err.set(false);
                                status.set("Removed from library.".into());
                                refresh_all();
                            }
                            Err(e) => {
                                status_err.set(true);
                                status.set(format!("Remove failed: {e}"));
                            }
                        }
                    });
                },
                on_commons: move |(uri, vis): (String, String)| {
                    spawn(async move {
                        match set_library_commons(&uri, &vis).await {
                            Ok(_) => {
                                status_err.set(false);
                                status.set(format!("Commons visibility → {vis}."));
                                refresh_all();
                            }
                            Err(e) => {
                                status_err.set(true);
                                status.set(format!("Commons update failed: {e}"));
                            }
                        }
                    });
                },
                on_share_card: move |uri: String| {
                    spawn(async move {
                        match library_commons_share_card(&uri).await {
                            Ok(v) => {
                                let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
                                share_card.set(text);
                                status_err.set(false);
                                status.set("Commons share card ready — copy from below; send via Talk → People.".into());
                            }
                            Err(e) => {
                                status_err.set(true);
                                status.set(format!("Share card failed: {e}"));
                            }
                        }
                    });
                },
            }
        },
    };

    rsx! {
        div { style: "{ROOT}",
            // ── Header ──
            div { style: "{HEADER}",
                div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:1rem;flex-wrap:wrap;",
                    div {
                        h1 { style: "margin:0;font-size:1.35rem;font-weight:700;color:#e9d5ff;letter-spacing:-0.02em;",
                            "Library"
                        }
                        p { style: "margin:0.35rem 0 0;font-size:0.82rem;color:#94a3b8;max-width:36rem;line-height:1.45;",
                            "Your files as meaning — topics, places, projects, time. Ingest once; find without digging folders."
                        }
                    }
                    div { style: "display:flex;gap:0.35rem;flex-wrap:wrap;align-items:center;",
                        button {
                            style: if view() == "list" { TAB_ON } else { TAB },
                            onclick: move |_| view.set("list".into()),
                            "List"
                        }
                        button {
                            style: if view() == "timeline" { TAB_ON } else { TAB },
                            onclick: move |_| view.set("timeline".into()),
                            "Timeline"
                        }
                        button {
                            style: if view() == "map" { TAB_ON } else { TAB },
                            onclick: move |_| view.set("map".into()),
                            "Map"
                        }
                        button { style: "{BTN2}", onclick: move |_| refresh_all(), "Refresh" }
                    }
                }
                div { style: "{STATS}",
                    span { style: "{STAT_CHIP}", span { style: "{STAT_NUM}", "{total}" } " items" }
                    span { style: "{STAT_CHIP}", span { style: "{STAT_NUM}", "{with_date}" } " dated" }
                    span { style: "{STAT_CHIP}", span { style: "{STAT_NUM}", "{with_place}" } " on map" }
                    span { style: "{STAT_CHIP}", span { style: "{STAT_NUM}", "{quins}" } " semantic edges" }
                    button {
                        style: "{BTN2} margin-left:auto;",
                        onclick: move |_| {
                            spawn(async move {
                                match export_library_graph().await {
                                    Ok(v) => {
                                        let n = u64_field(&v, "quin_count");
                                        status_err.set(false);
                                        status.set(format!(
                                            "Graph export ready · {n} NQuins (hypermedia edge mass for inject / query)."
                                        ));
                                    }
                                    Err(e) => {
                                        status_err.set(true);
                                        status.set(format!("Export failed: {e}"));
                                    }
                                }
                            });
                        },
                        "Export graph mass"
                    }
                }
                // Section rail — purpose lanes + Secret
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.4rem;margin-top:0.85rem;align-items:center;",
                    for (id, label, blurb) in SECTIONS {
                        {
                            let id = (*id).to_string();
                            let label = *label;
                            let blurb = *blurb;
                            let on = section() == id;
                            let count = st
                                .get("sections")
                                .and_then(|s| s.get(&id))
                                .and_then(|x| x.as_u64())
                                .unwrap_or(0);
                            rsx! {
                                button {
                                    title: "{blurb}",
                                    style: if on {
                                        if id == "secret" {
                                            "padding:0.4rem 0.75rem;border-radius:10px;border:1px solid #f59e0b;background:rgba(245,158,11,0.15);color:#fde68a;font-size:0.75rem;font-weight:700;cursor:pointer;"
                                        } else {
                                            TAB_ON
                                        }
                                    } else {
                                        TAB
                                    },
                                    onclick: move |_| {
                                        section.set(id.clone());
                                        if id == "secret" && !secret_unlocked() {
                                            results.set(Vec::new());
                                            status_err.set(false);
                                            status.set(
                                                "Secret section locked — click Unlock secret shelf to view sanctuary / classified items."
                                                    .into(),
                                            );
                                        } else {
                                            refresh_all();
                                        }
                                    },
                                    "{label}"
                                    if id != "all" {
                                        span { style: "opacity:0.75;margin-left:0.25rem;", "({count})" }
                                    }
                                }
                            }
                        }
                    }
                    if section() == "secret" {
                        button {
                            style: if secret_unlocked() {
                                "padding:0.4rem 0.75rem;border-radius:10px;border:1px solid #10b981;background:rgba(16,185,129,0.15);color:#a7f3d0;font-size:0.75rem;font-weight:700;cursor:pointer;"
                            } else {
                                "padding:0.4rem 0.75rem;border-radius:10px;border:1px solid #f59e0b;background:#78350f;color:#fde68a;font-size:0.75rem;font-weight:700;cursor:pointer;"
                            },
                            onclick: move |_| {
                                let next = !secret_unlocked();
                                secret_unlocked.set(next);
                                if next {
                                    status.set("Secret shelf unlocked for this session.".into());
                                    refresh_all();
                                } else {
                                    results.set(Vec::new());
                                    status.set("Secret shelf locked again.".into());
                                }
                            },
                            if secret_unlocked() { "Lock secret shelf" } else { "Unlock secret shelf" }
                        }
                    }
                }
                if section() == "commons" {
                    p { style: "margin:0.65rem 0 0;font-size:0.75rem;color:#94a3b8;line-height:1.4;",
                        "Commons is the permissive share surface — metadata cards for Talk peers / micro-commons. Secret and classified items never appear here. Connect people under Talk → People first."
                    }
                }
                if section() == "tools" {
                    p { style: "margin:0.65rem 0 0;font-size:0.75rem;color:#94a3b8;line-height:1.4;",
                        "Tools holds logs, telemetry, agent/tool output, and technical diagnostics — the machine paper trail, separate from personal notes and secret health."
                    }
                }
                if section() == "work" {
                    p { style: "margin:0.65rem 0 0;font-size:0.75rem;color:#94a3b8;line-height:1.4;",
                        "Work holds project labour and legislation. Paste Act text (or use native PDF ingest via host) — every section is stored with full body text for faceted search."
                    }
                }
                if section() == "software" {
                    p { style: "margin:0.65rem 0 0;font-size:0.75rem;color:#94a3b8;line-height:1.4;",
                        "Software holds QApps, websites, packages — including the early academic studio QApp inventory (catalogued & categorised; many are stubs). Use category facets and sort below. Runtime logs stay under Tools."
                    }
                    div { style: "display:flex;flex-wrap:wrap;gap:0.4rem;margin-top:0.55rem;align-items:center;",
                        button {
                            style: "{BTN}",
                            onclick: move |_| {
                                spawn(async move {
                                    match seed_studio_qapps().await {
                                        Ok(v) => {
                                            let added = u64_field(&v, "added");
                                            let updated = u64_field(&v, "updated");
                                            let total_c = u64_field(&v, "total_catalog");
                                            let cats = u64_field(&v, "categories");
                                            status_err.set(false);
                                            status.set(format!(
                                                "QApps seeded into Software · catalog {total_c} · +{added} new · {updated} refreshed · {cats} categories."
                                            ));
                                            section.set("software".into());
                                            refresh_all();
                                        }
                                        Err(e) => {
                                            status_err.set(true);
                                            status.set(format!("Seed QApps failed: {e}"));
                                        }
                                    }
                                });
                            },
                            "Seed academic QApps → Software"
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                category.set(String::new());
                                sort_mode.set("category".into());
                                refresh_all();
                            },
                            "Sort by category"
                        }
                    }
                }
                if section() == "secret" {
                    p { style: "margin:0.65rem 0 0;font-size:0.75rem;color:#fde68a;line-height:1.4;",
                        "Secret is for Wellfair-private health and other high-sensitivity material. It never exports to Commons. Unlock is session-local UI gate — Sanctuary vault still holds the enclave."
                    }
                }
            }

            div { style: "{BODY}",
                // ── Sidebar: ingest + search ──
                div { style: "{SIDE}",
                    // Free text search + faceted sort
                    div { style: "{CARD}",
                        div { style: "{H3}", "Search & sort" }
                        p { style: "{MUTED}", "Faceted browse: free text × section × category × sort. Anything you remember — topic, discipline, QApp name." }
                        input {
                            style: "{INPUT}",
                            placeholder: "Search library…",
                            value: "{q}",
                            oninput: move |e| q.set(e.value()),
                        }
                        label { style: "{LABEL}", "Sort" }
                        select {
                            style: "{INPUT}",
                            value: "{sort_mode}",
                            onchange: move |e| {
                                sort_mode.set(e.value());
                                refresh_all();
                            },
                            option { value: "newest", "Newest first" }
                            option { value: "oldest", "Oldest first" }
                            option { value: "title_asc", "Title A–Z" }
                            option { value: "title_desc", "Title Z–A" }
                            option { value: "category", "Category" }
                            option { value: "media_type", "Media type" }
                        }
                        button { style: "{BTN} width:100%;", onclick: do_quick_search, "Search" }
                        // Category facets (from faceted query counts — strong for Software/QApps)
                        {
                            let cat_map = facet_counts()
                                .get("categories")
                                .and_then(|t| t.as_object())
                                .cloned()
                                .unwrap_or_default();
                            let mut cat_list: Vec<(String, u64)> = cat_map
                                .iter()
                                .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n)))
                                .collect();
                            cat_list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                            rsx! {
                                if !cat_list.is_empty() || section() == "software" {
                                    div { style: "margin-top:0.75rem;",
                                        div { style: "{LABEL}", "Categories" }
                                        button {
                                            style: if category().is_empty() {
                                                "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0.1rem 0;border-radius:999px;background:rgba(139,92,246,0.35);border:1px solid #a78bfa;color:#f5f3ff;font-size:0.68rem;cursor:pointer;font-weight:700;"
                                            } else {
                                                TOPIC
                                            },
                                            onclick: move |_| {
                                                category.set(String::new());
                                                refresh_all();
                                            },
                                            "All categories"
                                        }
                                        for (c, n) in cat_list.iter() {
                                            {
                                                let c = c.clone();
                                                let n = *n;
                                                let on = category() == c;
                                                let label = category_label(&c);
                                                rsx! {
                                                    button {
                                                        style: if on {
                                                            "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0.1rem 0;border-radius:999px;background:rgba(139,92,246,0.35);border:1px solid #a78bfa;color:#f5f3ff;font-size:0.68rem;cursor:pointer;font-weight:700;"
                                                        } else {
                                                            TOPIC
                                                        },
                                                        onclick: move |_| {
                                                            category.set(c.clone());
                                                            if section() != "software" && section() != "all" {
                                                                // keep current section
                                                            }
                                                            refresh_all();
                                                            status.set(format!("Category “{label}” · {n}"));
                                                        },
                                                        "{label} · {n}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if !topic_list.is_empty() {
                            div { style: "margin-top:0.75rem;",
                                div { style: "{LABEL}", "Popular topics" }
                                for (t, n) in topic_list.iter().take(12) {
                                    {
                                        let t = t.clone();
                                        let n = *n;
                                        rsx! {
                                            button {
                                                style: "{TOPIC}",
                                                onclick: move |_| {
                                                    let t = t.clone();
                                                    value.set(t.clone());
                                                    facet.set("topic".into());
                                                    q.set(t.clone());
                                                    spawn(async move {
                                                        if let Ok(serde_json::Value::Array(r)) =
                                                            search_library("topic", &t).await
                                                        {
                                                            results.set(r);
                                                            status.set(format!("Topic “{t}”."));
                                                        }
                                                    });
                                                },
                                                "{t} · {n}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Native legislation ingest (structure parse — full section bodies)
                    div { style: "{CARD}",
                        div { style: "{H3}", "Legislation" }
                        p { style: "{MUTED}",
                            "Paste Act text (after the enacting formula). Native parse splits Part/Section/Subsection and stores every body under Work — not the old N3-without-text path."
                        }
                        label { style: "{LABEL}", "Register id (optional)" }
                        input {
                            style: "{INPUT}",
                            placeholder: "C2004A00601",
                            value: "{legis_id}",
                            oninput: move |e| legis_id.set(e.value()),
                        }
                        label { style: "{LABEL}", "Title hint (optional)" }
                        input {
                            style: "{INPUT}",
                            placeholder: "Privacy Act 1988",
                            value: "{legis_title}",
                            oninput: move |e| legis_title.set(e.value()),
                        }
                        label { style: "{LABEL}", "Instrument text" }
                        textarea {
                            style: "{INPUT} min-height:7rem;font-family:ui-monospace,monospace;font-size:0.72rem;",
                            placeholder: "The Parliament of Australia enacts:\n1  Short title\nThis Act may be cited as…",
                            value: "{legis_text}",
                            oninput: move |e| legis_text.set(e.value()),
                        }
                        button {
                            style: "{BTN} width:100%;",
                            onclick: move |_| {
                                let (text, id, title) = (legis_text(), legis_id(), legis_title());
                                spawn(async move {
                                    if text.trim().is_empty() {
                                        status_err.set(true);
                                        status.set("Paste legislation text first.".into());
                                        return;
                                    }
                                    let reg = if id.trim().is_empty() { None } else { Some(id.as_str()) };
                                    let th = if title.trim().is_empty() { None } else { Some(title.as_str()) };
                                    match ingest_legislation_text(&text, reg, Some("AU"), th).await {
                                        Ok(v) => {
                                            let secs = u64_field(&v, "sections");
                                            let with_t = u64_field(&v, "concepts_with_text");
                                            let empty = u64_field(&v, "empty_text");
                                            let written = u64_field(&v, "library_entries_written");
                                            status_err.set(false);
                                            status.set(format!(
                                                "Legislation ingested · {secs} sections · {with_t} with text · {empty} empty · {written} library rows → Work."
                                            ));
                                            section.set("work".into());
                                            legis_text.set(String::new());
                                            refresh_all();
                                        }
                                        Err(e) => {
                                            status_err.set(true);
                                            status.set(format!("Legislation ingest failed: {e}"));
                                        }
                                    }
                                });
                            },
                            "Ingest legislation → Work"
                        }
                    }

                    // Facet + timeline
                    div { style: "{CARD}",
                        div { style: "{H3}", "Facet & time" }
                        label { style: "{LABEL}", "Facet" }
                        select {
                            style: "{INPUT}",
                            value: "{facet}",
                            onchange: move |e| facet.set(e.value()),
                            option { value: "topic", "Topic" }
                            option { value: "project", "Project" }
                            option { value: "purpose", "Purpose" }
                            option { value: "place", "Place" }
                            option { value: "depicts", "Depicts" }
                        }
                        label { style: "{LABEL}", "Value" }
                        input {
                            style: "{INPUT}",
                            placeholder: "biology · house-move · tax-return",
                            value: "{value}",
                            oninput: move |e| value.set(e.value()),
                        }
                        button { style: "{BTN2} width:100%;margin-bottom:0.65rem;", onclick: do_facet_search, "Run facet search" }
                        div { style: "display:grid;grid-template-columns:1fr 1fr;gap:0.4rem;",
                            div {
                                label { style: "{LABEL}", "From" }
                                input { style: "{INPUT}", placeholder: "2025-01-01", value: "{tl_from}", oninput: move |e| tl_from.set(e.value()) }
                            }
                            div {
                                label { style: "{LABEL}", "To" }
                                input { style: "{INPUT}", placeholder: "2025-12-31", value: "{tl_to}", oninput: move |e| tl_to.set(e.value()) }
                            }
                        }
                        button { style: "{BTN2} width:100%;", onclick: do_timeline, "Filter timeline range" }
                    }

                    // Ingest
                    div { style: "{CARD}",
                        div { style: "display:flex;justify-content:space-between;align-items:center;margin-bottom:0.35rem;",
                            div { style: "{H3} margin:0;", "Add to library" }
                            button {
                                style: "{BTN2}",
                                onclick: move |_| {
                                    let n = !show_ingest();
                                    show_ingest.set(n);
                                },
                                if show_ingest() { "Hide" } else { "Show" }
                            }
                        }
                        if show_ingest() {
                            p { style: "{MUTED}",
                                "Paste a note, receipt, or research blurb. Topics are derived automatically. Attach date/place/project when you care."
                            }
                            label { style: "{LABEL}", "Title id" }
                            input {
                                style: "{INPUT}",
                                value: "{ing_uri}",
                                oninput: move |e| ing_uri.set(e.value()),
                            }
                            label { style: "{LABEL}", "Type" }
                            select {
                                style: "{INPUT}",
                                value: "{ing_media}",
                                onchange: move |e| ing_media.set(e.value()),
                                option { value: "text/markdown", "Text / markdown" }
                                option { value: "image/jpeg", "Image JPEG" }
                                option { value: "image/png", "Image PNG" }
                                option { value: "audio/wav", "Audio WAV" }
                            }
                            label { style: "{LABEL}", "Section" }
                            select {
                                style: "{INPUT}",
                                value: "{ing_section}",
                                onchange: move |e| {
                                    let v = e.value();
                                    ing_section.set(v.clone());
                                    if v == "secret" {
                                        ing_sensitivity.set("classified".into());
                                        ing_commons.set("none".into());
                                    } else if v == "wellfair" {
                                        // Wellfair default restricted unless user opens Secret.
                                        if ing_sensitivity() == "public" {
                                            ing_sensitivity.set("restricted".into());
                                        }
                                    } else if v == "commons" {
                                        ing_sensitivity.set("public".into());
                                        ing_commons.set("commons".into());
                                    }
                                },
                                option { value: "personal", "Personal" }
                                option { value: "wellfair", "Wellfair (health / care)" }
                                option { value: "work", "Work / project" }
                                option { value: "tools", "Tools / logs / technical" }
                                option { value: "software", "Software (QApps / websites)" }
                                option { value: "commons", "Commons (shareable)" }
                                option { value: "secret", "Secret (sanctuary)" }
                            }
                            label { style: "{LABEL}", "Sensitivity" }
                            select {
                                style: "{INPUT}",
                                value: "{ing_sensitivity}",
                                onchange: move |e| ing_sensitivity.set(e.value()),
                                option { value: "public", "Public" }
                                option { value: "restricted", "Restricted (enclave)" }
                                option { value: "classified", "Classified / sanctuary" }
                            }
                            label { style: "{LABEL}", "Social / commons reach" }
                            select {
                                style: "{INPUT}",
                                value: "{ing_commons}",
                                onchange: move |e| ing_commons.set(e.value()),
                                option { value: "none", "Device only" }
                                option { value: "peers", "Talk peers (bilateral)" }
                                option { value: "commons", "Permissive commons" }
                            }
                            label { style: "{LABEL}", if ing_binary() { "Bytes (hex)" } else { "Content" } }
                            textarea {
                                style: "{INPUT} min-height:7rem;resize:vertical;line-height:1.4;",
                                placeholder: "The liver is an organ… keep this receipt for the tax deduction…",
                                value: "{ing_text}",
                                oninput: move |e| ing_text.set(e.value()),
                            }
                            label { style: "display:flex;gap:0.4rem;align-items:center;font-size:0.72rem;color:#94a3b8;margin-bottom:0.5rem;",
                                input {
                                    r#type: "checkbox",
                                    checked: ing_binary(),
                                    onchange: move |e| ing_binary.set(e.checked()),
                                }
                                "Binary (hex) — photo EXIF fills timeline & map"
                            }
                            label { style: "{LABEL}", "Date → timeline" }
                            input { style: "{INPUT}", placeholder: "YYYY-MM-DD", value: "{ing_date}", oninput: move |e| ing_date.set(e.value()) }
                            label { style: "{LABEL}", "Place lat,lon → map" }
                            input { style: "{INPUT}", placeholder: "-33.87,151.21", value: "{ing_place}", oninput: move |e| ing_place.set(e.value()) }
                            label { style: "{LABEL}", "Place label" }
                            input { style: "{INPUT}", value: "{ing_place_label}", oninput: move |e| ing_place_label.set(e.value()) }
                            label { style: "{LABEL}", "Project" }
                            input { style: "{INPUT}", placeholder: "house-move-2025", value: "{ing_project}", oninput: move |e| ing_project.set(e.value()) }
                            label { style: "{LABEL}", "Purpose" }
                            input { style: "{INPUT}", placeholder: "tax-return-2025", value: "{ing_purpose}", oninput: move |e| ing_purpose.set(e.value()) }
                            label { style: "{LABEL}", "Guardian DID (optional)" }
                            input { style: "{INPUT}", value: "{ing_guardian}", oninput: move |e| ing_guardian.set(e.value()) }
                            button { style: "{BTN} width:100%;margin-top:0.35rem;", onclick: do_ingest, "Ingest into library" }
                        }
                    }

                    div { style: "font-size:0.72rem;color:#64748b;line-height:1.4;padding:0.25rem;",
                        Link {
                            to: Route::SanctuaryRoute {},
                            style: "color:#a78bfa;",
                            "Sanctuary"
                        }
                        " unlocks the vault host · "
                        Link {
                            to: Route::TalkRoute {},
                            style: "color:#a78bfa;",
                            "Talk → Projects"
                        }
                        " for cooperative work"
                    }
                }

                // ── Main results ──
                div { style: "{MAIN}",
                    if !status().is_empty() {
                        div {
                            style: if status_err() { STATUS_ERR } else { STATUS_OK },
                            "{status}"
                        }
                    }
                    if !share_card().is_empty() {
                        div { style: "{CARD}",
                            div { style: "{H3}", "Commons share card (metadata only)" }
                            pre {
                                style: "margin:0;white-space:pre-wrap;word-break:break-all;font-size:0.68rem;color:#a7f3d0;max-height:160px;overflow:auto;",
                                "{share_card}"
                            }
                        }
                    }
                    if rows.is_empty() && !status_err() {
                        div {
                            style: "padding:2.5rem 1.5rem;text-align:center;border:1px dashed #334155;border-radius:16px;background:#0f172a;",
                            div { style: "font-size:2rem;margin-bottom:0.5rem;", "📚" }
                            h2 { style: "margin:0 0 0.4rem;color:#e9d5ff;font-size:1.1rem;", "Your library is empty" }
                            p { style: "margin:0 auto;max-width:28rem;color:#94a3b8;font-size:0.85rem;line-height:1.5;",
                                "Add a note on the left — research, a receipt, a caption. We derive topics and keep the original addressable by meaning. Photos with EXIF land on the timeline and map automatically."
                            }
                        }
                    } else {
                        {body}
                    }
                }
            }
        }
    }
}

#[component]
fn ListView(
    rows: Vec<serde_json::Value>,
    on_topic: EventHandler<String>,
    on_remove: EventHandler<String>,
    on_commons: EventHandler<(String, String)>,
    on_share_card: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            for r in rows {
                {
                    let uri = str_field(&r, "asset_uri");
                    let media = str_field(&r, "media_type");
                    let excerpt = str_field(&r, "excerpt");
                    let topics = arr_str(&r, "topics");
                    let projects = arr_str(&r, "projects");
                    let purposes = arr_str(&r, "purposes");
                    let place = str_field(&r, "place");
                    let section = str_field(&r, "section");
                    let sens = str_field(&r, "sensitivity");
                    let title = display_title(&uri);
                    let icon = media_icon(&media);
                    let uri_rm = uri.clone();
                    let uri_peers = uri.clone();
                    let uri_com = uri.clone();
                    let uri_share = uri.clone();
                    let uri_none = uri.clone();
                    let is_secret = r.get("is_secret").and_then(|x| x.as_bool()).unwrap_or(false)
                        || section == "secret";
                    let flagged = r
                        .get("flags")
                        .and_then(|x| x.as_array())
                        .map(|a| !a.is_empty())
                        .unwrap_or(false);
                    let border = if is_secret {
                        "padding:0.85rem 1rem;border-radius:12px;border:1px solid #b45309;background:#1c1917;margin-bottom:0.55rem;"
                    } else if section == "commons" {
                        "padding:0.85rem 1rem;border-radius:12px;border:1px solid #0e7490;background:#0c1a24;margin-bottom:0.55rem;"
                    } else {
                        ENTRY
                    };
                    rsx! {
                        article { style: "{border}",
                            div { style: "display:flex;gap:0.75rem;align-items:flex-start;",
                                div { style: "font-size:1.4rem;line-height:1;flex-shrink:0;", "{icon}" }
                                div { style: "flex:1;min-width:0;",
                                    div { style: "display:flex;justify-content:space-between;gap:0.5rem;align-items:baseline;flex-wrap:wrap;",
                                        h3 { style: "margin:0;font-size:0.95rem;color:#f3f4f6;font-weight:650;", "{title}" }
                                        div { style: "display:flex;gap:0.35rem;align-items:center;",
                                            if !section.is_empty() {
                                                span {
                                                    style: "font-size:0.65rem;padding:0.12rem 0.45rem;border-radius:999px;background:#1e293b;color:#cbd5e1;border:1px solid #334155;",
                                                    "{section}"
                                                }
                                            }
                                            if !sens.is_empty() && sens != "public" {
                                                span {
                                                    style: "font-size:0.65rem;padding:0.12rem 0.45rem;border-radius:999px;background:rgba(245,158,11,0.15);color:#fde68a;border:1px solid #b45309;",
                                                    "{sens}"
                                                }
                                            }
                                            if let Some(t) = i64_field(&r, "occurred_at") {
                                                span { style: "font-size:0.7rem;color:#a78bfa;white-space:nowrap;", "{fmt_date(t)}" }
                                            }
                                        }
                                    }
                                    div { style: "font-size:0.65rem;color:#64748b;font-family:ui-monospace,monospace;margin:0.15rem 0 0.35rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                                        "{uri}"
                                    }
                                    if !excerpt.is_empty() {
                                        p { style: "margin:0 0 0.45rem;font-size:0.8rem;color:#94a3b8;line-height:1.45;",
                                            "{excerpt}"
                                        }
                                    }
                                    div {
                                        for t in topics {
                                            {
                                                let t2 = t.clone();
                                                rsx! {
                                                    button {
                                                        style: "{TOPIC}",
                                                        onclick: move |_| on_topic.call(t2.clone()),
                                                        "{t}"
                                                    }
                                                }
                                            }
                                        }
                                        for p in projects {
                                            span {
                                                style: "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0;border-radius:999px;background:rgba(16,185,129,0.12);border:1px solid #065f46;color:#6ee7b7;font-size:0.68rem;",
                                                "📁 {p}"
                                            }
                                        }
                                        for p in purposes {
                                            span {
                                                style: "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0;border-radius:999px;background:rgba(167,139,250,0.12);border:1px solid #5b21b6;color:#ddd6fe;font-size:0.68rem;",
                                                "🎯 {p}"
                                            }
                                        }
                                        if !place.is_empty() {
                                            span {
                                                style: "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0;border-radius:999px;background:rgba(56,189,248,0.1);border:1px solid #0e7490;color:#7dd3fc;font-size:0.68rem;",
                                                "📍 {place}"
                                            }
                                        }
                                        if flagged {
                                            span {
                                                style: "display:inline-block;padding:0.15rem 0.5rem;margin:0.1rem 0.2rem 0;border-radius:999px;background:rgba(239,68,68,0.12);border:1px solid #991b1b;color:#fca5a5;font-size:0.68rem;",
                                                "flagged"
                                            }
                                        }
                                    }
                                    if !is_secret {
                                        div { style: "display:flex;flex-wrap:wrap;gap:0.35rem;margin-top:0.55rem;",
                                            button {
                                                style: "{BTN2}",
                                                onclick: move |_| on_commons.call((uri_peers.clone(), "peers".into())),
                                                "Share → peers"
                                            }
                                            button {
                                                style: "{BTN2}",
                                                onclick: move |_| on_commons.call((uri_com.clone(), "commons".into())),
                                                "Share → commons"
                                            }
                                            button {
                                                style: "{BTN2}",
                                                onclick: move |_| on_share_card.call(uri_share.clone()),
                                                "Commons card"
                                            }
                                            button {
                                                style: "{BTN2}",
                                                onclick: move |_| on_commons.call((uri_none.clone(), "none".into())),
                                                "Unshare"
                                            }
                                        }
                                    }
                                }
                                button {
                                    style: "{BTN2} flex-shrink:0;color:#fca5a5;border-color:#7f1d1d;",
                                    onclick: move |_| on_remove.call(uri_rm.clone()),
                                    "Remove"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TimelineView(rows: Vec<serde_json::Value>) -> Element {
    let mut dated: Vec<(i64, serde_json::Value)> = rows
        .into_iter()
        .filter_map(|r| i64_field(&r, "occurred_at").map(|t| (t, r)))
        .collect();
    dated.sort_by_key(|(t, _)| *t);
    if dated.is_empty() {
        return rsx! {
            div { style: "padding:2rem;text-align:center;color:#94a3b8;font-size:0.85rem;",
                "No dated assets yet. Add a date when ingesting, or drop in a photo with EXIF."
            }
        };
    }
    rsx! {
        div { style: "border-left:2px solid #7c3aed;margin-left:0.5rem;padding-left:0;",
            for (t, r) in dated {
                div {
                    key: "{str_field(&r, \"asset_uri\")}",
                    style: "position:relative;padding:0.55rem 0 1rem 1.15rem;",
                    span {
                        style: "position:absolute;left:-7px;top:0.7rem;width:12px;height:12px;border-radius:50%;background:#a78bfa;box-shadow:0 0 0 3px #0b1220;",
                    }
                    div { style: "font-size:0.72rem;font-weight:700;color:#a78bfa;margin-bottom:0.2rem;",
                        "{fmt_date(t)}"
                    }
                    div { style: "font-size:0.9rem;color:#f3f4f6;font-weight:600;",
                        "{display_title(&str_field(&r, \"asset_uri\"))}"
                    }
                    div { style: "font-size:0.78rem;color:#94a3b8;margin-top:0.2rem;",
                        "{str_field(&r, \"excerpt\")}"
                    }
                }
            }
        }
    }
}

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
        return rsx! {
            div { style: "padding:2rem;text-align:center;color:#94a3b8;font-size:0.85rem;",
                "No located assets. Attach lat,lon when ingesting, or use a geotagged photo."
            }
        };
    }
    rsx! {
        div {
            svg {
                view_box: "0 0 360 180",
                width: "100%",
                style: "background:linear-gradient(180deg,#0f172a,#1e1b4b);border:1px solid #334155;border-radius:14px;max-height:380px;",
                for gx in [60, 120, 180, 240, 300] {
                    line { x1: "{gx}", y1: "0", x2: "{gx}", y2: "180", stroke: "#334155", stroke_width: "0.5" }
                }
                for gy in [45, 90, 135] {
                    line { x1: "0", y1: "{gy}", x2: "360", y2: "{gy}", stroke: "#334155", stroke_width: "0.5" }
                }
                for (lat, lon, r) in placed.clone() {
                    circle {
                        cx: "{lon + 180.0}",
                        cy: "{90.0 - lat}",
                        r: "3.2",
                        fill: "#a78bfa",
                        fill_opacity: "0.9",
                        stroke: "#ede9fe",
                        stroke_width: "0.6",
                        title { "{str_field(&r, \"asset_uri\")} ({lat:.3},{lon:.3})" }
                    }
                }
            }
            ul { style: "margin:0.75rem 0 0;padding:0;list-style:none;",
                for (lat, lon, r) in placed {
                    li {
                        key: "{str_field(&r, \"asset_uri\")}",
                        style: "font-size:0.78rem;display:flex;gap:0.65rem;padding:0.35rem 0;border-bottom:1px solid #1f2937;",
                        span { style: "color:#7dd3fc;font-variant-numeric:tabular-nums;", "{lat:.3}, {lon:.3}" }
                        span { style: "color:#e5e7eb;", "{display_title(&str_field(&r, \"asset_uri\"))}" }
                    }
                }
            }
        }
    }
}
