//! Cooperative economics body — SDN permissive lanes & true-cost personal modeler.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::Route;
use dioxus::prelude::*;

const ECON_RATE: &str = r#"requires [ capability("capability.invoke") ];
effect fn peer_rate() {
    return capability.invoke("Econ.evaluate_peer", {
        peer: "did:qualia:0x3f8a...",
        entity_type: "human_commons"
    });
}
"#;

const FINANCE_NPV: &str = r#"requires [ capability("capability.invoke") ];
effect fn npv() {
    return capability.invoke("Finance.npv", {
        rate: 0.05,
        cash_flows: [-1000.0, 300.0, 420.0, 500.0]
    });
}
"#;

const FINANCE_VAR: &str = r#"requires [ capability("capability.invoke") ];
effect fn portfolio_var() {
    return capability.invoke("Finance.portfolio_var", {
        weights: [0.4, 0.6],
        cov_matrix: [[0.04, 0.01], [0.01, 0.09]],
        confidence: 0.95
    });
}
"#;

const COMMERCE_SETTLE: &str = r#"requires [ capability("capability.invoke") ];
effect fn settle() {
    return capability.invoke("commerce_settle", {
        chain: "did:q42",
        from: "did:q42:sovereign:alpha",
        to: "did:q42:coop:commons",
        amount_cents: 12500,
        currency: "USD",
        settlement_mode: "AtomicP2P"
    });
}
"#;

#[derive(Clone, Copy)]
struct ChainMeta {
    name: &'static str,
    id: &'static str,
    token: &'static str,
    posture: &'static str,
    address: &'static str,
    balance: &'static str,
}

const CHAINS: &[ChainMeta] = &[
    ChainMeta { name: "Qualia Sovereign", id: "did:q42", token: "QUALS", posture: "Ed25519", address: "did:q42:427a9c8e10b42f", balance: "42,000.00" },
    ChainMeta { name: "Cardano (eUTxO)", id: "cardano", token: "ADA", posture: "Ed25519-BIP32", address: "addr1q42...cardano", balance: "420.00" },
    ChainMeta { name: "Solana", id: "solana", token: "SOL", posture: "Ed25519", address: "8xLt7P...42mQ", balance: "48.50" },
    ChainMeta { name: "Bitcoin", id: "bitcoin", token: "BTC", posture: "Schnorr / Taproot", address: "bc1p...42q", balance: "0.245" },
    ChainMeta { name: "Polkadot", id: "polkadot", token: "DOT", posture: "SR25519", address: "14xQ...dot42", balance: "185.00" },
    ChainMeta { name: "Cosmos", id: "cosmos", token: "ATOM", posture: "Secp256k1", address: "cosmos1...42a", balance: "62.40" },
    ChainMeta { name: "Solid REA", id: "solid-rea", token: "CREDIT", posture: "W3C DID", address: "https://pod.open.coop/profile#me", balance: "1,200.00" },
];

#[component]
/// Dioxus `EconomicsBody` component.
pub fn EconomicsBody() -> Element {
    let nav = navigator();
    let mut selected_chain = use_signal(|| 0usize);
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    let active = CHAINS[selected_chain().min(CHAINS.len() - 1)];

    rsx! {
        div { style: "display:grid;gap:12px;padding:8px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px;",
                div {
                    p { style: muted(), "Socially Defined Networking & Cooperative Economics: Sovereign multi-chain assets, SDN permissive lanes & true-cost personal modeler." }
                }
                button {
                    r#type: "button",
                    style: "background:#3b82f6;color:#ffffff;border:none;border-radius:4px;padding:4px 10px;font-size:.72rem;font-weight:600;cursor:pointer;",
                    onclick: move |_| { nav.push(Route::WalletRoute {}); },
                    "↗ Open Full Multi-Chain Wallet in OS-Shell (/wallet)"
                }
            }

            // Chain Selector Bar
            div { style: "display:flex;gap:4px;flex-wrap:wrap;background:#0f172a;padding:4px;border-radius:6px;border:1px solid #1e293b;",
                for (idx, ch) in CHAINS.iter().enumerate() {
                    button {
                        key: "{ch.id}",
                        r#type: "button",
                        style: if selected_chain() == idx {
                            "padding:3px 8px;border-radius:4px;background:#38bdf8;color:#0f172a;font-weight:700;font-size:.68rem;border:none;cursor:pointer;"
                        } else {
                            "padding:3px 8px;border-radius:4px;background:transparent;color:#94a3b8;font-size:.68rem;border:none;cursor:pointer;"
                        },
                        onclick: move |_| selected_chain.set(idx),
                        "{ch.name}"
                    }
                }
            }

            // Active Chain Ledger & Posture Summary Card
            div { style: "display:flex;gap:16px;background:#131c2e;border:1px solid #1e293b;border-radius:6px;padding:10px 14px;align-items:center;flex-wrap:wrap;",
                div { style: "display:grid;gap:2px;",
                    span { style: "color:#64748b;font-size:.65rem;text-transform:uppercase;", "Chain & Token" }
                    strong { style: "color:#38bdf8;font-size:.85rem;", "{active.name} ({active.token})" }
                }
                div { style: "display:grid;gap:2px;",
                    span { style: "color:#64748b;font-size:.65rem;text-transform:uppercase;", "Signing Key Posture" }
                    span { style: "background:#0f172a;color:#a78bfa;border:1px solid #334155;padding:2px 6px;border-radius:3px;font-size:.70rem;font-family:monospace;", "{active.posture}" }
                }
                div { style: "display:grid;gap:2px;",
                    span { style: "color:#64748b;font-size:.65rem;text-transform:uppercase;", "Identifier (did:q42 / Derived Address)" }
                    span { style: "color:#e2e8f0;font-size:.72rem;font-family:monospace;", "{active.address}" }
                }
                div { style: "display:grid;gap:2px;margin-left:auto;",
                    span { style: "color:#64748b;font-size:.65rem;text-transform:uppercase;", "Sovereign Balance" }
                    strong { style: "color:#34d399;font-size:.9rem;font-family:monospace;", "{active.balance} {active.token}" }
                }
            }

            // POET Kernel Evaluation Actions
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(ECON_RATE, "peer_rate", &mut busy, &mut out),
                    "Econ.evaluate_peer"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(FINANCE_NPV, "npv", &mut busy, &mut out),
                    "Finance.npv"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(FINANCE_VAR, "portfolio_var", &mut busy, &mut out),
                    "Finance.portfolio_var"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(COMMERCE_SETTLE, "settle", &mut busy, &mut out),
                    "commerce_settle"
                }
            }

            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#34d399;background:#090d16;padding:8px;border-radius:4px;border:1px solid #1e293b;", "{out}" }
            }
        }
    }
}

fn run(src: &'static str, name: &'static str, busy: &mut Signal<bool>, out: &mut Signal<String>) {
    busy.set(true);
    let mut busy = *busy;
    let mut out = *out;
    spawn(async move {
        match engine::eval(src.into(), false, Some(name.into())).await {
            Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
            Err(e) => out.set(e),
        }
        busy.set(false);
    });
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
