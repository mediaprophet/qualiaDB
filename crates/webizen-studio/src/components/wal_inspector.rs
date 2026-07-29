use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
struct DeployRecord {
    revision: u64,
    unix_ts: u32,
    pane_count: u16,
    manifest_hash: u64,
}

#[component]
pub fn WalInspector() -> Element {
    let mut records = use_signal(Vec::<DeployRecord>::new);
    let mut status = use_signal(|| "Loading deploy history…".to_string());
    let replay_status = use_signal(|| String::new());
    let replaying = use_signal(|| None::<u64>);

    let mut refresh_history = move || {
        if !crate::endpoints::is_native_host() {
            status.set("WAL history requires the desktop host.".to_string());
            return;
        }
        spawn(async move {
            match reqwest::get(crate::endpoints::manifest_history_url()).await {
                Ok(res) if res.status().is_success() => {
                    match res.json::<Vec<DeployRecord>>().await {
                        Ok(rows) => {
                            let count = rows.len();
                            records.set(rows);
                            status.set(format!(
                                "{count} deploy checkpoint(s) in studio-workspace.wal"
                            ));
                        }
                        Err(err) => status.set(format!("History parse failed: {err}")),
                    }
                }
                Ok(res) => status.set(format!("History fetch failed ({})", res.status())),
                Err(err) => status.set(format!("History unreachable: {err}")),
            }
        });
    };

    use_effect(move || {
        refresh_history();
    });

    let rows = records.read();
    let latest_revision = rows.last().map(|r| r.revision).unwrap_or(0);
    let total_panes: u16 = rows.iter().map(|r| r.pane_count).sum();

    rsx! {
        div {
            style: "padding: 24px; background: var(--qualia-bg, #050510); color: var(--qualia-text, #e5e5e5); height: 100%; font-family: 'Inter', sans-serif; overflow-y: auto;",
            h1 { style: "margin-top: 0; color: var(--qualia-accent, #f59e0b); font-size: 1.1rem;",
                "Studio Deploy WAL"
            }
            p { style: "font-size: 0.8rem; color: var(--qualia-text-muted, #888); margin-top: 0;",
                "{status()}"
            }
            if !replay_status.read().is_empty() {
                p {
                    style: "font-size: 0.75rem; color: var(--qualia-accent); margin: 0.35rem 0 0;",
                    "{replay_status.read()}"
                }
            }

            div { style: "display: flex; gap: 12px; margin: 1rem 0 1.5rem;",
                div { style: "padding: 14px; background: var(--qualia-surface, #111); border-radius: 8px; border: 1px solid var(--qualia-border, #333); flex: 1;",
                    div { style: "font-size: 11px; color: var(--qualia-text-muted); text-transform: uppercase;", "Latest revision" }
                    div { style: "font-size: 22px; font-weight: 700;", "{latest_revision}" }
                }
                div { style: "padding: 14px; background: var(--qualia-surface, #111); border-radius: 8px; border: 1px solid var(--qualia-border, #333); flex: 1;",
                    div { style: "font-size: 11px; color: var(--qualia-text-muted); text-transform: uppercase;", "Deploy events" }
                    div { style: "font-size: 22px; font-weight: 700; color: var(--qualia-accent);", "{rows.len()}" }
                }
                div { style: "padding: 14px; background: var(--qualia-surface, #111); border-radius: 8px; border: 1px solid var(--qualia-border, #333); flex: 1;",
                    div { style: "font-size: 11px; color: var(--qualia-text-muted); text-transform: uppercase;", "Pane quins (sum)" }
                    div { style: "font-size: 22px; font-weight: 700; color: #34d399;", "{total_panes}" }
                }
            }

            h3 { style: "font-size: 0.85rem; margin-bottom: 0.5rem;", "Deploy checkpoints" }
            if rows.is_empty() {
                div {
                    style: "padding: 1rem; border: 1px dashed var(--qualia-border); border-radius: 8px; color: var(--qualia-text-muted); font-size: 0.8rem;",
                    "Save a workspace from QApp Studio to append the first Quin checkpoint."
                }
            } else {
                table {
                    style: "width: 100%; border-collapse: collapse; background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 8px; overflow: hidden; font-size: 0.78rem;",
                    thead { style: "background: rgba(245,158,11,0.08); border-bottom: 1px solid var(--qualia-border);",
                        tr {
                            th { style: "padding: 10px 12px; text-align: left;", "Rev" }
                            th { style: "padding: 10px 12px; text-align: left;", "Unix ts" }
                            th { style: "padding: 10px 12px; text-align: left;", "Panes" }
                            th { style: "padding: 10px 12px; text-align: left;", "Manifest hash" }
                            th { style: "padding: 10px 12px; text-align: left;", "Restore" }
                        }
                    }
                    tbody {
                        for row in rows.iter().rev() {
                            tr { style: "border-bottom: 1px solid var(--qualia-border);",
                                td { style: "padding: 10px 12px; font-family: monospace;", "#{row.revision}" }
                                td { style: "padding: 10px 12px; font-family: monospace;", "{row.unix_ts}" }
                                td { style: "padding: 10px 12px;", "{row.pane_count}" }
                                td { style: "padding: 10px 12px; font-family: monospace; font-size: 0.7rem;",
                                    "0x{row.manifest_hash:012x}"
                                }
                                td { style: "padding: 10px 12px;",
                                    if crate::endpoints::is_native_host() {
                                        button {
                                            style: "padding: 0.2rem 0.5rem; font-size: 0.65rem; border-radius: 5px; border: 1px solid var(--qualia-accent); background: rgba(245,158,11,0.1); color: var(--qualia-text); cursor: pointer;",
                                            disabled: replaying() == Some(row.revision),
                                            onclick: {
                                                let rev = row.revision;
                                                let mut replay_status = replay_status.clone();
                                                let mut replaying = replaying.clone();
                                                move |_| {
                                                    replaying.set(Some(rev));
                                                    replay_status.set(format!("Restoring rev #{rev}…"));
                                                    spawn(async move {
                                                        let client = reqwest::Client::new();
                                                        let url = crate::endpoints::manifest_replay_url(rev);
                                                        match client.post(&url).send().await {
                                                            Ok(res) if res.status().is_success() => {
                                                                replay_status.set(format!(
                                                                    "Restored rev #{rev} — reload QApp Studio to edit the workspace."
                                                                ));
                                                            }
                                                            Ok(res) => {
                                                                replay_status.set(format!(
                                                                    "Restore failed ({}) for rev #{rev}",
                                                                    res.status()
                                                                ));
                                                            }
                                                            Err(err) => {
                                                                replay_status.set(format!(
                                                                    "Restore unreachable: {err}"
                                                                ));
                                                            }
                                                        }
                                                        replaying.set(None);
                                                    });
                                                }
                                            },
                                            if replaying() == Some(row.revision) { "…" } else { "Restore" }
                                        }
                                    } else {
                                        span { style: "color: var(--qualia-text-muted); font-size: 0.65rem;", "—" }
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
