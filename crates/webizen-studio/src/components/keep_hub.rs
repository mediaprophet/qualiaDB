//! Keep hub — shelter, recent/sayable volume browse, life-domain directory.
//!
//! Frame D: Keep shelters. Commit only when the write is real.
//! Missing volume → held / not yet. Never "unavailable". No Host invent.
//! Continuity handle ≠ human is left to Identity chrome (untouched).

use dioxus::prelude::*;
use webizen_studio::keep_volume::{
    held_outcome, interpret_commit, interpret_open, merge_browse_list, remember_recent,
    sayable_name, KeepOutcome, RecentVolume, VaultVolume, VolumeLook, COMMIT_ID, HELD_WHY,
    OPEN_ID,
};
#[cfg(target_arch = "wasm32")]
use webizen_studio::keep_volume::{parse_recents_json, recents_json, RECENT_STORAGE_KEY};

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::qapp_engine::invoke_json;
use crate::Route;

#[component]
pub fn KeepHub() -> Element {
    let mut recents = use_signal(load_recents);
    let mut browse = use_signal(Vec::<RecentVolume>::new);
    let mut selected = use_signal(String::new);
    let outcome = use_signal(|| KeepOutcome::Held {
        why: HELD_WHY.to_string(),
    });
    let busy = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            let vault = match invoke_json("list_q42_volumes", serde_json::json!({})).await {
                Ok(value) => value
                    .get("volumes")
                    .cloned()
                    .and_then(|v| serde_json::from_value::<Vec<VaultVolume>>(v).ok())
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            let merged = merge_browse_list(&load_recents(), &vault);
            if selected.peek().is_empty() {
                if let Some(first) = merged.first() {
                    selected.set(first.path.clone());
                }
            }
            recents.set(load_recents());
            browse.set(merged);
        });
    });

    let current = outcome();
    let look = current.look();
    let celebrate = current.celebrates();
    let selected_path = selected();
    let selected_sayable = if selected_path.is_empty() {
        String::new()
    } else {
        sayable_name(&selected_path)
    };
    let cards = browse();
    let is_busy = busy();

    rsx! {
        div {
            style: "flex:1; min-height:0; overflow-y:auto; padding:2rem 2rem 3rem; max-width:720px; margin:0 auto; color:var(--qualia-text); box-sizing:border-box; width:100%;",
            "data-keep-hub": "1",
            "data-volume-state": "{look.as_str()}",
            "data-beat": "{look.named_beat()}",
            "data-keep-celebrate": "{celebrate}",

            section {
                "data-keep-volume-browse": "1",
                style: "margin-bottom:2rem;padding:1.15rem 1.2rem 1.25rem;border-radius:16px;border:1px solid var(--qualia-border,#1f2937);background:rgba(15,23,42,0.55);",
                div { style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.35rem;",
                    span {
                        style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#94a3b8;",
                        "Keep"
                    }
                    span {
                        style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid #475569;background:rgba(71,85,105,0.2);color:#cbd5e1;font-weight:700;",
                        "Shelter · reopen"
                    }
                }
                h1 { style: "margin:0 0 0.35rem; font-size:1.6rem; font-weight:700;", "Keep" }
                p { style: "margin:0 0 1rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.95rem;",
                    "Keep shelters. Commit only when the write is real. Reopen what you saved — browse recent volumes, not a typed path."
                }

                div {
                    class: "keep-volume-chip",
                    "data-volume-state": "{look.as_str()}",
                    "data-beat": "{look.named_beat()}",
                    style: keep_chip_style(look, celebrate),
                    if celebrate {
                        "Keep · committed"
                    } else {
                        "{look.label()}"
                    }
                }

                match &current {
                    KeepOutcome::Held { why } => rsx! {
                        p {
                            class: "keep-held-why",
                            "data-gate": "held",
                            style: "margin:0.75rem 0 0;font-size:0.85rem;color:#94a3b8;line-height:1.45;",
                            "{why}"
                        }
                    },
                    KeepOutcome::Open { sayable, .. } => rsx! {
                        p { style: "margin:0.75rem 0 0;font-size:0.85rem;color:#cbd5e1;",
                            "Open · {sayable}"
                        }
                    },
                    KeepOutcome::Committed { sayable, written, .. } => rsx! {
                        p {
                            "data-keep-commit-beat": "1",
                            style: "margin:0.75rem 0 0;font-size:0.85rem;color:#86efac;",
                            "Committed · {sayable} · {written} written"
                        }
                    },
                }

                if cards.is_empty() {
                    p {
                        "data-keep-empty": "1",
                        style: "margin:1rem 0 0;font-size:0.88rem;color:#94a3b8;line-height:1.45;",
                        "held / not yet — no recent volume. Browse to reopen a keep."
                    }
                } else {
                    div {
                        role: "list",
                        "aria-label": "Recent volumes",
                        style: "margin-top:1rem;display:flex;flex-direction:column;gap:0.45rem;",
                        for item in cards.iter() {
                            {
                                let path = item.path.clone();
                                let active = selected_path == item.path;
                                rsx! {
                                    button {
                                        r#type: "button",
                                        role: "listitem",
                                        "data-keep-sayable": "{item.sayable}",
                                        disabled: is_busy,
                                        onclick: move |_| selected.set(path.clone()),
                                        style: keep_card_style(active),
                                        strong { style: "display:block;font-size:0.95rem;", "{item.sayable}" }
                                        span { style: "font-size:0.72rem;color:#64748b;", "volume · reopen" }
                                    }
                                }
                            }
                        }
                    }
                }

                if !selected_sayable.is_empty() {
                    p { style: "margin:0.85rem 0 0;font-size:0.8rem;color:#94a3b8;",
                        "Selected · {selected_sayable}"
                    }
                }

                div { style: "margin-top:1rem;display:flex;flex-wrap:wrap;gap:0.5rem;",
                    button {
                        r#type: "button",
                        "data-keep-browse": "1",
                        disabled: is_busy,
                        onclick: move |_| browse_existing(selected, recents, browse, outcome, busy),
                        style: keep_btn_style(false),
                        "Browse"
                    }
                    button {
                        r#type: "button",
                        "data-keep-start": "1",
                        disabled: is_busy,
                        onclick: move |_| start_keep(selected, recents, browse, outcome, busy),
                        style: keep_btn_style(false),
                        "Start keep"
                    }
                    button {
                        r#type: "button",
                        "data-keep-open": "1",
                        "data-capability": OPEN_ID,
                        disabled: is_busy || selected_path.is_empty(),
                        onclick: move |_| open_selected(selected(), recents, browse, outcome, busy, false),
                        style: keep_btn_style(false),
                        if is_busy { "Opening…" } else { "Reopen" }
                    }
                    button {
                        r#type: "button",
                        "data-keep-commit": "1",
                        "data-capability": COMMIT_ID,
                        disabled: is_busy || selected_path.is_empty(),
                        onclick: move |_| commit_selected(selected(), recents, browse, outcome, busy),
                        style: keep_btn_style(true),
                        "Commit"
                    }
                }
                p { style: "margin:0.7rem 0 0;font-size:0.75rem;color:#64748b;line-height:1.4;",
                    "Browse uses the system picker. Commit celebrates only after GraphDatabase.volume_commit writes."
                }
            }

            KeepDirectory {}
        }
    }
}

#[component]
fn KeepDirectory() -> Element {
    rsx! {
        div { style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.35rem;",
            span {
                style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#94a3b8;",
                "Directory"
            }
            span {
                style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid #475569;background:rgba(71,85,105,0.2);color:#cbd5e1;font-weight:700;",
                "Secondary · prefer life-domain nav"
            }
        }
        h2 { style: "margin:0 0 0.35rem; font-size:1.15rem; font-weight:700;", "All destinations" }
        p { style: "margin:0 0 1.5rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.95rem;",
            "Private on this machine. Primary shell uses life domains: Memory · Relations · Care · Practice · World · Instruments. This page is a full index for deep links."
        }
        div { style: "display:flex; flex-direction:column; gap:0.65rem;",
            KeepTalkTabLink { tab: "chat", title: "Relations — Chat", blurb: "Private local agent. Nothing leaves this machine unless you send it. Instruments are not peers." }
            KeepTalkTabLink { tab: "people", title: "Relations — People", blurb: "Invites, contacts, magic links, groups — natural persons, not identity assets." }
            KeepTalkTabLink { tab: "reception", title: "Relations — Reception", blurb: "Domain front door + DNS TXT so peers can find you without seeing your vault." }
            KeepTalkTabLink { tab: "mail", title: "Relations — Mail", blurb: "Purpose inboxes, relationship addresses, catchall, SMTP/IMAP after domain setup." }
            KeepTalkTabLink { tab: "projects", title: "Practice — Projects", blurb: "Cooperative projects and QualiaDB Development Cooperative seed · Remember → Memory." }
            KeepLink { to: Route::WellfairRoute {}, title: "Care — Wellfair shell", blurb: "Body, rights, welfare, labour under principal control. Unlock vault for private records." }
            KeepLink { to: Route::SanctuaryRoute {}, title: "Care — Sanctuary (vault)", blurb: "Unlock when cooperative projects or work board need the host API." }
            KeepLink { to: Route::WorkRoute {}, title: "Practice — Work board", blurb: "Kanban — project id fills from Relations → Projects." }
            KeepLink { to: Route::AnatomyRoute {}, title: "Care — Anatomy", blurb: "See systems and conditions on a reference body." }
            KeepLink { to: Route::HealthRoute {}, title: "Care — Health vault", blurb: "Vitals, sleep, medication, wellbeing — local journal, not cloud." }
            KeepLink { to: Route::LibraryRoute {}, title: "Memory — Lived Memory", blurb: "Hypermedia shelf — notes, photos, receipts found by meaning, time, and place." }
            KeepLink { to: Route::PoetCatalogRoute {}, title: "Catalog · Lexicon", blurb: "Open a lexicon pack — or honest held / not yet. Live GraphDatabase.lexicon_manifest." }
            KeepLink { to: Route::VisionRoute {}, title: "Instruments — Vision", blurb: "Local detect/overlay — not a peer person. Synthetic scenes, reject/correct without erasing claims." }
            KeepLink { to: Route::ListenRoute {}, title: "Instruments — Listen", blurb: "Local ears — features, reference events (not full ASR). Not social." }
            KeepLink { to: Route::IdentityRoute {}, title: "You — Identity", blurb: "Personal profile, social book, consent. Identifiers ≠ the natural person." }
            KeepLink { to: Route::SanctuaryRoute {}, title: "Care — Sanctuary", blurb: "Vault lock and protected spaces." }
            KeepLink { to: Route::AgencyRoute {}, title: "Care — Agency", blurb: "Guardianship, accountability, safeguards." }
            KeepLink { to: Route::ChoraRoute {}, title: "World — Chora commons", blurb: "Spatio-temporal commons manifold — attributed public layers." }
            KeepLink { to: Route::BrowserRoute {}, title: "World — Browser", blurb: "Web pages project into the same entity session as Memory." }
        }
    }
}

#[component]
fn KeepLink(to: Route, title: &'static str, blurb: &'static str) -> Element {
    rsx! {
        Link {
            to: to,
            style: "display:block; text-decoration:none; color:inherit; padding:1rem 1.15rem; border-radius:12px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.22); transition:border-color 0.15s;",
            strong { style: "display:block; font-size:1rem; margin-bottom:0.25rem;", "{title}" }
            span { style: "font-size:0.85rem; color:var(--qualia-text-muted); line-height:1.4;", "{blurb}" }
        }
    }
}

#[component]
fn KeepTalkTabLink(tab: &'static str, title: &'static str, blurb: &'static str) -> Element {
    rsx! {
        Link {
            to: Route::TalkRoute {},
            style: "display:block; text-decoration:none; color:inherit; padding:1rem 1.15rem; border-radius:12px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.22); transition:border-color 0.15s;",
            onclick: move |_| {
                #[cfg(target_arch = "wasm32")]
                if let Some(win) = web_sys::window() {
                    if let Ok(Some(storage)) = win.session_storage() {
                        let _ = storage.set_item("webizen_talk_tab", tab);
                    }
                }
            },
            strong { style: "display:block; font-size:1rem; margin-bottom:0.25rem;", "{title}" }
            span { style: "font-size:0.85rem; color:var(--qualia-text-muted); line-height:1.4;", "{blurb}" }
        }
    }
}

fn keep_chip_style(look: VolumeLook, celebrate: bool) -> &'static str {
    if celebrate {
        "display:inline-block;margin-top:0.15rem;padding:0.2rem 0.55rem;border-radius:999px;border:1px solid #4ade80;background:rgba(74,222,128,0.12);color:#86efac;font-size:0.72rem;font-weight:700;"
    } else {
        match look {
            VolumeLook::Open => {
                "display:inline-block;margin-top:0.15rem;padding:0.2rem 0.55rem;border-radius:999px;border:1px solid #67e8f9;background:rgba(103,232,249,0.1);color:#a5f3fc;font-size:0.72rem;font-weight:700;"
            }
            _ => {
                "display:inline-block;margin-top:0.15rem;padding:0.2rem 0.55rem;border-radius:999px;border:1px solid #475569;background:rgba(71,85,105,0.2);color:#cbd5e1;font-size:0.72rem;font-weight:700;"
            }
        }
    }
}

fn keep_card_style(active: bool) -> &'static str {
    if active {
        "text-align:left;cursor:pointer;padding:0.75rem 0.9rem;border-radius:12px;border:1px solid #67e8f9;background:rgba(103,232,249,0.08);color:inherit;"
    } else {
        "text-align:left;cursor:pointer;padding:0.75rem 0.9rem;border-radius:12px;border:1px solid var(--qualia-border,#1f2937);background:rgba(0,0,0,0.22);color:inherit;"
    }
}

fn keep_btn_style(primary: bool) -> &'static str {
    if primary {
        "padding:0.55rem 0.9rem;border-radius:10px;border:1px solid #67e8f9;background:#0891b2;color:#082f49;font-weight:700;cursor:pointer;"
    } else {
        "padding:0.55rem 0.9rem;border-radius:10px;border:1px solid #475569;background:transparent;color:#e2e8f0;font-weight:600;cursor:pointer;"
    }
}

fn load_recents() -> Vec<RecentVolume> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                if let Ok(Some(raw)) = storage.get_item(RECENT_STORAGE_KEY) {
                    return parse_recents_json(&raw);
                }
            }
        }
    }
    Vec::new()
}

fn persist_recents(items: &[RecentVolume]) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                let _ = storage.set_item(RECENT_STORAGE_KEY, &recents_json(items));
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = items;
    }
}

fn remember_path(path: &str, recents: &mut Signal<Vec<RecentVolume>>, browse: &mut Signal<Vec<RecentVolume>>) {
    let next = remember_recent(&recents(), path);
    persist_recents(&next);
    recents.set(next.clone());
    let vault_only: Vec<VaultVolume> = browse()
        .into_iter()
        .filter(|item| !next.iter().any(|r| r.path == item.path))
        .map(|item| VaultVolume {
            path: item.path,
            display_name: item.sayable,
            relative: String::new(),
            open_error: None,
        })
        .collect();
    browse.set(merge_browse_list(&next, &vault_only));
}

fn browse_existing(
    mut selected: Signal<String>,
    mut recents: Signal<Vec<RecentVolume>>,
    mut browse: Signal<Vec<RecentVolume>>,
    mut outcome: Signal<KeepOutcome>,
    mut busy: Signal<bool>,
) {
    if busy() {
        return;
    }
    busy.set(true);
    spawn(async move {
        match engine::browse_q42_volume().await {
            Ok(Some(path)) if !path.trim().is_empty() => {
                selected.set(path.clone());
                remember_path(&path, &mut recents, &mut browse);
                outcome.set(held_outcome(HELD_WHY));
            }
            Ok(_) => outcome.set(held_outcome(HELD_WHY)),
            Err(_) => outcome.set(held_outcome(HELD_WHY)),
        }
        busy.set(false);
    });
}

fn start_keep(
    mut selected: Signal<String>,
    mut recents: Signal<Vec<RecentVolume>>,
    mut browse: Signal<Vec<RecentVolume>>,
    mut outcome: Signal<KeepOutcome>,
    mut busy: Signal<bool>,
) {
    if busy() {
        return;
    }
    busy.set(true);
    spawn(async move {
        match engine::start_q42_volume().await {
            Ok(Some(path)) if !path.trim().is_empty() => {
                selected.set(path.clone());
                remember_path(&path, &mut recents, &mut browse);
                open_selected(path, recents, browse, outcome, busy, true);
            }
            _ => {
                outcome.set(held_outcome(HELD_WHY));
                busy.set(false);
            }
        }
    });
}

fn open_selected(
    path: String,
    mut recents: Signal<Vec<RecentVolume>>,
    mut browse: Signal<Vec<RecentVolume>>,
    mut outcome: Signal<KeepOutcome>,
    mut busy: Signal<bool>,
    create: bool,
) {
    let path = path.trim().to_string();
    if path.is_empty() {
        outcome.set(held_outcome(HELD_WHY));
        return;
    }
    busy.set(true);
    spawn(async move {
        let next = match engine::volume_open(path.clone(), create).await {
            Ok(PoetEvalResult {
                ok,
                value,
                diagnostic,
                ..
            }) => interpret_open(ok, &value, diagnostic.as_deref(), &path),
            Err(_) => held_outcome(HELD_WHY),
        };
        if matches!(next, KeepOutcome::Open { .. }) {
            remember_path(&path, &mut recents, &mut browse);
        }
        outcome.set(next);
        busy.set(false);
    });
}

fn commit_selected(
    path: String,
    mut recents: Signal<Vec<RecentVolume>>,
    mut browse: Signal<Vec<RecentVolume>>,
    mut outcome: Signal<KeepOutcome>,
    mut busy: Signal<bool>,
) {
    let path = path.trim().to_string();
    if path.is_empty() {
        outcome.set(held_outcome(HELD_WHY));
        return;
    }
    busy.set(true);
    spawn(async move {
        let next = match engine::volume_commit(path.clone()).await {
            Ok(PoetEvalResult {
                ok,
                value,
                diagnostic,
                ..
            }) => interpret_commit(ok, &value, diagnostic.as_deref(), &path),
            Err(_) => held_outcome(HELD_WHY),
        };
        if next.celebrates() {
            remember_path(&path, &mut recents, &mut browse);
        }
        outcome.set(next);
        busy.set(false);
    });
}
