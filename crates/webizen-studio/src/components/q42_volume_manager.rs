//! Vault console for unified Q42 volumes — list, inspect, verify, magnet, compact.

use dioxus::prelude::*;
use serde::Deserialize;

use crate::components::qapp_engine::invoke_json;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct VolumeItem {
    path: String,
    relative: String,
    display_name: String,
    file_bytes: u64,
    version: u16,
    flags: u16,
    flag_names: Vec<String>,
    block_count: u64,
    lexicon_entries: Option<u64>,
    has_bidx: bool,
    has_field_ranges: bool,
    has_field_postings: bool,
    is_volume_root: bool,
    publication_class: String,
    publication_transport: String,
    may_public_magnet: bool,
    open_error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct Workspace {
    storage_path: String,
    volumes: Vec<VolumeItem>,
    total_bytes: u64,
    volume_count: usize,
    unreadable: usize,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct Section {
    name: String,
    offset: u64,
    length: u64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct InspectReport {
    path: String,
    file_bytes: u64,
    version: u16,
    flags: u16,
    flag_names: Vec<String>,
    block_count: u64,
    block_size: u32,
    quins_per_block: u32,
    lexicon_bytes: u64,
    lexicon_entries: Option<u64>,
    lexicon_has_no_terms: bool,
    has_bidx: bool,
    has_field_ranges: bool,
    has_field_postings: bool,
    is_volume_root: bool,
    publication_class: String,
    publication_transport: String,
    may_public_magnet: bool,
    publication_reason: String,
    sections: Vec<Section>,
    honesty: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct VerifyCheck {
    name: String,
    status: String,
    detail: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct VerifyReceipt {
    path: String,
    level: String,
    overall: String,
    checks: Vec<VerifyCheck>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct VerifySet {
    root: String,
    overall: String,
    members: Vec<VerifyReceipt>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct Magnet {
    path: String,
    display_name: String,
    info_hash_sha1: String,
    byte_length: u64,
    magnet_uri: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct MagnetResult {
    root: Magnet,
    children: Vec<Magnet>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct CompactResult {
    source: String,
    output: String,
}

fn bytes_label(n: u64) -> String {
    const KIB: f64 = 1024.0;
    let n = n as f64;
    if n >= KIB * KIB * KIB {
        format!("{:.2} GiB", n / (KIB * KIB * KIB))
    } else if n >= KIB * KIB {
        format!("{:.1} MiB", n / (KIB * KIB))
    } else if n >= KIB {
        format!("{:.1} KiB", n / KIB)
    } else {
        format!("{n:.0} B")
    }
}

fn status_color(status: &str) -> &'static str {
    match status {
        "pass" => "#4ade80",
        "fail" => "#fb7185",
        "incomplete" => "#fbbf24",
        "not_applicable" => "#94a3b8",
        _ => "#67e8f9",
    }
}

fn class_color(class: &str) -> &'static str {
    if class.contains("commons") {
        "#4ade80"
    } else if class.contains("sanctuary") || class.contains("medical") || class.contains("bilateral")
    {
        "#fb7185"
    } else if class.contains("unreadable") {
        "#fb7185"
    } else {
        "#fbbf24"
    }
}

#[component]
pub fn Q42VolumeManager() -> Element {
    let mut workspace = use_signal(Workspace::default);
    let mut status = use_signal(|| "Loading vault volumes…".to_string());
    let mut selected = use_signal(|| None::<String>);
    let mut inspect = use_signal(|| None::<InspectReport>);
    let mut verify = use_signal(|| None::<VerifySet>);
    let mut magnet = use_signal(|| None::<MagnetResult>);
    let mut action = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut open_path = use_signal(String::new);

    let mut refresh = move || {
        if busy() {
            return;
        }
        busy.set(true);
        status.set("Scanning vault for .q42 volumes…".into());
        spawn(async move {
            match invoke_json("list_q42_volumes", serde_json::json!({})).await {
                Ok(value) => match serde_json::from_value::<Workspace>(value) {
                    Ok(listed) => {
                        let count = listed.volume_count;
                        let bytes = bytes_label(listed.total_bytes);
                        let unread = listed.unreadable;
                        status.set(format!(
                            "{count} volume(s), {bytes} under {}. {unread} unreadable.",
                            listed.storage_path
                        ));
                        workspace.set(listed);
                    }
                    Err(err) => status.set(format!("List parse failed: {err}")),
                },
                Err(err) => status.set(format!("{err}")),
            }
            busy.set(false);
        });
    };

    use_effect(move || {
        refresh();
    });

    let mut load_inspect = move |path: String| {
        selected.set(Some(path.clone()));
        inspect.set(None);
        verify.set(None);
        magnet.set(None);
        action.set(String::new());
        busy.set(true);
        spawn(async move {
            match invoke_json("inspect_q42_volume", serde_json::json!({ "path": path })).await {
                Ok(value) => match serde_json::from_value::<InspectReport>(value) {
                    Ok(report) => inspect.set(Some(report)),
                    Err(err) => action.set(format!("Inspect parse failed: {err}")),
                },
                Err(err) => action.set(err),
            }
            busy.set(false);
        });
    };

    let run_verify = move |_| {
        let Some(path) = selected() else {
            return;
        };
        busy.set(true);
        action.set("Verifying every named member (Full). Large volume sets take time.".into());
        spawn(async move {
            match invoke_json(
                "verify_q42_volume",
                serde_json::json!({ "path": path, "level": "full" }),
            )
            .await
            {
                Ok(value) => match serde_json::from_value::<VerifySet>(value) {
                    Ok(report) => {
                        action.set(format!(
                            "Verify {} — {} member(s)",
                            report.overall,
                            report.members.len()
                        ));
                        verify.set(Some(report));
                    }
                    Err(err) => action.set(format!("Verify parse failed: {err}")),
                },
                Err(err) => action.set(err),
            }
            busy.set(false);
        });
    };

    let run_magnet = move |_| {
        let Some(path) = selected() else {
            return;
        };
        busy.set(true);
        action.set("Minting magnet only if this volume is Permissive Commons…".into());
        spawn(async move {
            match invoke_json("magnet_q42_volume", serde_json::json!({ "path": path })).await {
                Ok(value) => match serde_json::from_value::<MagnetResult>(value) {
                    Ok(result) => {
                        action.set(format!(
                            "Magnet minted for {} ({} child segment(s))",
                            result.root.display_name,
                            result.children.len()
                        ));
                        magnet.set(Some(result));
                    }
                    Err(err) => action.set(format!("Magnet parse failed: {err}")),
                },
                Err(err) => action.set(err),
            }
            busy.set(false);
        });
    };

    let run_compact = move |_| {
        let Some(path) = selected() else {
            return;
        };
        busy.set(true);
        action.set("Rewriting through the current v3 writer (PIDX/FIDX/ECC)…".into());
        spawn(async move {
            match invoke_json("compact_q42_volume", serde_json::json!({ "path": path })).await {
                Ok(value) => match serde_json::from_value::<CompactResult>(value) {
                    Ok(result) => {
                        action.set(format!("Compacted to {}", result.output));
                        load_inspect(result.output);
                    }
                    Err(err) => {
                        action.set(format!("Compact parse failed: {err}"));
                        busy.set(false);
                    }
                },
                Err(err) => {
                    action.set(err);
                    busy.set(false);
                }
            }
        });
    };

    let open_typed = move |_| {
        let path = open_path.read().trim().to_string();
        if path.is_empty() {
            action.set("Enter a .q42 path to inspect.".into());
            return;
        }
        load_inspect(path);
    };

    let rows = workspace.read().volumes.clone();
    let selected_path = selected();
    let current_inspect = inspect();
    let current_verify = verify();
    let current_magnet = magnet();
    let is_busy = busy();

    rsx! {
        section { style: "height:100%;overflow:auto;padding:1.5rem;box-sizing:border-box;background:linear-gradient(145deg,#101827,#07111d);color:#e5edf7;font-family:Inter,system-ui,sans-serif;",
            div { style: "max-width:1180px;margin:0 auto;display:flex;flex-direction:column;gap:1rem;",
                header { style: "display:flex;justify-content:space-between;gap:1rem;flex-wrap:wrap;align-items:flex-start;",
                    div {
                        div { style: "font-size:.75rem;letter-spacing:.12em;text-transform:uppercase;color:#67e8f9;font-weight:700;", "Qualia graph vault" }
                        h1 { style: "font-size:1.55rem;margin:.25rem 0;", "Q42 volumes" }
                        p { style: "margin:0;color:#a5b4c7;max-width:46rem;line-height:1.5;",
                            "Unified v3 files in this vault (Index, Chats, WellFair, runtime). Inspect and verify on the current reader. Public magnets stay fail-closed."
                        }
                    }
                    button {
                        disabled: is_busy,
                        onclick: move |_| refresh(),
                        style: "padding:.7rem 1rem;border:0;border-radius:8px;background:#2563eb;color:white;font-weight:700;cursor:pointer;",
                        if is_busy { "Working…" } else { "Refresh vault" }
                    }
                }

                p { style: "margin:0;color:#cbd5e1;font-size:.88rem;", "{status}" }

                div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:.65rem;",
                    Stat { label: "Volumes".to_string(), value: workspace().volume_count.to_string() }
                    Stat { label: "Bytes".to_string(), value: bytes_label(workspace().total_bytes) }
                    Stat { label: "Unreadable".to_string(), value: workspace().unreadable.to_string() }
                    Stat { label: "Storage".to_string(), value: workspace().storage_path.clone() }
                }

                div { style: "display:flex;gap:.6rem;flex-wrap:wrap;",
                    input {
                        value: "{open_path}",
                        disabled: is_busy,
                        placeholder: "Open a .q42 path not in the vault…",
                        oninput: move |event| open_path.set(event.value()),
                        style: "flex:1;min-width:16rem;padding:.65rem .75rem;border-radius:8px;border:1px solid rgba(148,163,184,.35);background:#07111d;color:#f8fafc;font-family:ui-monospace,monospace;"
                    }
                    button {
                        disabled: is_busy,
                        onclick: open_typed,
                        style: "padding:.65rem 1rem;border-radius:8px;border:1px solid #67e8f9;background:transparent;color:#67e8f9;font-weight:700;cursor:pointer;",
                        "Inspect path"
                    }
                }

                if rows.is_empty() {
                    div { style: "padding:1rem;border:1px dashed rgba(148,163,184,.35);border-radius:10px;color:#94a3b8;",
                        "No .q42 files under the vault yet. Import a catalog, compact a chat, or inspect a path."
                    }
                } else {
                    div { style: "overflow:auto;border:1px solid rgba(148,163,184,.2);border-radius:12px;background:rgba(15,23,42,.78);",
                        table { style: "width:100%;border-collapse:collapse;font-size:.78rem;",
                            thead { style: "background:rgba(34,211,238,.08);",
                                tr {
                                    th { style: "text-align:left;padding:.65rem .7rem;", "File" }
                                    th { style: "text-align:left;padding:.65rem .7rem;", "Size" }
                                    th { style: "text-align:left;padding:.65rem .7rem;", "v" }
                                    th { style: "text-align:left;padding:.65rem .7rem;", "Blocks" }
                                    th { style: "text-align:left;padding:.65rem .7rem;", "Lex" }
                                    th { style: "text-align:left;padding:.65rem .7rem;", "Indexes" }
                                    th { style: "text-align:left;padding:.65rem .7rem;", "Publication" }
                                }
                            }
                            tbody {
                                for item in rows.iter() {
                                    {
                                        let path = item.path.clone();
                                        let active = selected_path.as_deref() == Some(item.path.as_str());
                                        let row_bg = if active { "rgba(37,99,235,.22)" } else { "transparent" };
                                        let pub_color = class_color(&item.publication_class);
                                        let indexes = format!(
                                            "{}{}{}",
                                            if item.has_bidx { "BIDX " } else { "" },
                                            if item.has_field_ranges { "FIDX " } else { "" },
                                            if item.has_field_postings { "PIDX" } else { "" },
                                        );
                                        let lex = item
                                            .lexicon_entries
                                            .map(|n| n.to_string())
                                            .unwrap_or_else(|| "—".into());
                                        rsx! {
                                            tr {
                                                style: "border-top:1px solid rgba(148,163,184,.12);cursor:pointer;background:{row_bg};",
                                                onclick: move |_| load_inspect(path.clone()),
                                                td { style: "padding:.6rem .7rem;font-family:ui-monospace,monospace;",
                                                    div { "{item.display_name}" }
                                                    div { style: "color:#64748b;font-size:.7rem;", "{item.relative}" }
                                                    if let Some(err) = item.open_error.as_ref() {
                                                        div { style: "color:#fb7185;font-size:.7rem;", "{err}" }
                                                    }
                                                }
                                                td { style: "padding:.6rem .7rem;", "{bytes_label(item.file_bytes)}" }
                                                td { style: "padding:.6rem .7rem;", "{item.version}" }
                                                td { style: "padding:.6rem .7rem;", "{item.block_count}" }
                                                td { style: "padding:.6rem .7rem;", "{lex}" }
                                                td { style: "padding:.6rem .7rem;color:#94a3b8;", "{indexes}" }
                                                td { style: "padding:.6rem .7rem;color:{pub_color};", "{item.publication_class}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(report) = current_inspect {
                    div { style: "background:rgba(15,23,42,.78);border:1px solid rgba(148,163,184,.2);border-radius:14px;padding:1.1rem;display:flex;flex-direction:column;gap:.75rem;",
                        h2 { style: "margin:0;font-size:1rem;", "Inspect" }
                        div { style: "font-family:ui-monospace,monospace;font-size:.78rem;color:#cbd5e1;word-break:break-all;", "{report.path}" }
                        div { style: "display:flex;flex-wrap:wrap;gap:.45rem;",
                            Chip { text: format!("v{}", report.version) }
                            Chip { text: format!("{} blocks × {} B", report.block_count, report.block_size) }
                            Chip { text: format!("{} flags {:#06x}", report.flag_names.join(" | "), report.flags) }
                            Chip { text: format!("lex {} B / {:?}", report.lexicon_bytes, report.lexicon_entries) }
                        }
                        p { style: "margin:0;color:{class_color(&report.publication_class)};font-size:.85rem;",
                            "{report.publication_class} — {report.publication_transport}"
                        }
                        p { style: "margin:0;color:#94a3b8;font-size:.8rem;line-height:1.45;", "{report.publication_reason}" }

                        div { style: "display:flex;gap:.5rem;flex-wrap:wrap;",
                            button { disabled: is_busy, onclick: run_verify, style: "padding:.55rem .85rem;border:0;border-radius:8px;background:#2563eb;color:white;font-weight:700;cursor:pointer;", "Verify full" }
                            button { disabled: is_busy, onclick: run_magnet, style: "padding:.55rem .85rem;border:1px solid #67e8f9;border-radius:8px;background:transparent;color:#67e8f9;font-weight:700;cursor:pointer;", "Public magnet" }
                            button { disabled: is_busy, onclick: run_compact, style: "padding:.55rem .85rem;border:1px solid #fbbf24;border-radius:8px;background:transparent;color:#fbbf24;font-weight:700;cursor:pointer;", "Compact to v3" }
                        }

                        if !action().is_empty() {
                            p { style: "margin:0;color:#dbeafe;font-size:.82rem;line-height:1.45;", "{action}" }
                        }

                        if !report.honesty.is_empty() {
                            ul { style: "margin:0;padding-left:1.1rem;color:#fde68a;font-size:.8rem;",
                                for note in report.honesty.iter() {
                                    li { "{note}" }
                                }
                            }
                        }

                        table { style: "width:100%;border-collapse:collapse;font-size:.75rem;",
                            thead { tr {
                                th { style: "text-align:left;padding:.35rem 0;color:#94a3b8;", "Section" }
                                th { style: "text-align:left;padding:.35rem 0;color:#94a3b8;", "Offset" }
                                th { style: "text-align:left;padding:.35rem 0;color:#94a3b8;", "Length" }
                            } }
                            tbody {
                                for section in report.sections.iter() {
                                    tr {
                                        td { style: "padding:.28rem 0;font-family:ui-monospace,monospace;", "{section.name}" }
                                        td { style: "padding:.28rem 0;", "{section.offset}" }
                                        td { style: "padding:.28rem 0;", "{bytes_label(section.length)}" }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(report) = current_verify {
                    div { style: "background:rgba(15,23,42,.78);border:1px solid rgba(148,163,184,.2);border-radius:14px;padding:1.1rem;",
                        h2 { style: "margin:0 0 .6rem;font-size:1rem;color:{status_color(&report.overall)};",
                            "Verify {report.overall} — {report.members.len()} member(s)"
                        }
                        for member in report.members.iter() {
                            div { style: "margin-bottom:.8rem;",
                                div { style: "font-family:ui-monospace,monospace;font-size:.72rem;color:#94a3b8;margin-bottom:.3rem;", "{member.path}" }
                                for check in member.checks.iter() {
                                    div { style: "display:grid;grid-template-columns:11rem 6rem 1fr;gap:.4rem;font-size:.75rem;padding:.18rem 0;border-top:1px solid rgba(148,163,184,.08);",
                                        span { style: "font-family:ui-monospace,monospace;", "{check.name}" }
                                        span { style: "color:{status_color(&check.status)};font-weight:700;", "{check.status}" }
                                        span { style: "color:#cbd5e1;", "{check.detail}" }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(result) = current_magnet {
                    div { style: "background:rgba(15,23,42,.78);border:1px solid rgba(34,211,238,.28);border-radius:14px;padding:1.1rem;",
                        h2 { style: "margin:0 0 .5rem;font-size:1rem;", "Permissive Commons magnet" }
                        MagnetRow { magnet: result.root.clone() }
                        for child in result.children.iter() {
                            MagnetRow { magnet: child.clone() }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Stat(label: String, value: String) -> Element {
    rsx! {
        div { style: "padding:14px;background:rgba(15,23,42,.78);border:1px solid rgba(148,163,184,.2);border-radius:10px;",
            div { style: "font-size:11px;color:#94a3b8;text-transform:uppercase;", "{label}" }
            div { style: "font-size:15px;font-weight:700;word-break:break-all;margin-top:.25rem;", "{value}" }
        }
    }
}

#[component]
fn Chip(text: String) -> Element {
    rsx! {
        span { style: "font-size:.72rem;padding:.25rem .5rem;border-radius:999px;border:1px solid rgba(148,163,184,.35);color:#cbd5e1;", "{text}" }
    }
}

#[component]
fn MagnetRow(magnet: Magnet) -> Element {
    rsx! {
        div { style: "margin-bottom:.7rem;",
            div { style: "font-size:.8rem;color:#67e8f9;", "{magnet.display_name} · {bytes_label(magnet.byte_length)} · {magnet.info_hash_sha1}" }
            pre { style: "margin:.3rem 0 0;white-space:pre-wrap;word-break:break-all;font-size:.72rem;color:#e2e8f0;background:#07111d;padding:.6rem;border-radius:8px;",
                "{magnet.magnet_uri}"
            }
        }
    }
}
