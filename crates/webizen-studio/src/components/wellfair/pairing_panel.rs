//! Desktop pairing QR — scan from phone companion to open WS ingest.

use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
struct CompanionPairingInfo {
    ws_url: String,
    lan_ip: String,
    port: u16,
    qr_path: String,
}

#[component]
pub fn CompanionPairingPanel() -> Element {
    let mut pairing = use_signal(CompanionPairingInfo::default);
    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        spawn(async move {
            match super::host_client::fetch_companion_pairing().await {
                Ok(json) => {
                    if let Ok(info) = serde_json::from_str::<CompanionPairingInfo>(&json) {
                        pairing.set(info);
                        loaded.set(true);
                    }
                }
                Err(_) => {}
            }
        });
    });

    let port = pairing.read().port;
    let qr_src = if port > 0 {
        format!("http://127.0.0.1:{port}/mobile/qr")
    } else {
        "http://127.0.0.1:8080/mobile/qr".into()
    };

    rsx! {
        section {
            aria_label: "Companion pairing",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            super::shared::DomainChrome { domain: "Instruments", chip: "Phone remote · installable controller", show_memory: false }
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Pair your phone" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Open the WellFair companion on your phone, scan this QR, then export Samsung Health CSVs on the device."
            }
            if *loaded.read() && !pairing.read().ws_url.is_empty() {
                div {
                    style: "display:flex;flex-wrap:wrap;gap:1rem;align-items:flex-start;",
                    img {
                        src: "{qr_src}",
                        alt: "Companion WebSocket pairing QR",
                        style: "width:160px;height:160px;background:#fff;border-radius:8px;padding:6px;",
                    }
                    div {
                        style: "flex:1;min-width:200px;",
                        p { style: "margin:0 0 0.35rem;font-size:0.75rem;color:var(--qualia-text-muted,#666);", "WebSocket URL (LAN)" }
                        code {
                            style: "display:block;padding:0.45rem;font-size:0.72rem;word-break:break-all;background:#111;color:#e8e8e8;border-radius:6px;",
                            "{pairing.read().ws_url}"
                        }
                        p {
                            style: "margin:0.5rem 0 0;font-size:0.72rem;color:var(--qualia-text-muted,#888);",
                            "LAN IP {pairing.read().lan_ip} · port {pairing.read().port}"
                        }
                    }
                }
            } else {
                p { style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);", "Loading pairing info…" }
            }
        }
    }
}
