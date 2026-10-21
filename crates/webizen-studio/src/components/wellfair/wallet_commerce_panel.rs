//! Multi-Chain Sovereign Wallet & Cooperative Commerce Panel.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::host_client::add_ledger_entry;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainKind {
    DidQ42,
    Cardano,
    Solana,
    Bitcoin,
    Polkadot,
    Cosmos,
    SolidRea,
}

impl ChainKind {
    pub const ALL: [Self; 7] = [
        Self::DidQ42,
        Self::Cardano,
        Self::Solana,
        Self::Bitcoin,
        Self::Polkadot,
        Self::Cosmos,
        Self::SolidRea,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::DidQ42 => "did:q42",
            Self::Cardano => "cardano",
            Self::Solana => "solana",
            Self::Bitcoin => "bitcoin",
            Self::Polkadot => "polkadot",
            Self::Cosmos => "cosmos",
            Self::SolidRea => "solid_rea",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::DidQ42 => "did:q42 (Hardware Root)",
            Self::Cardano => "Cardano (eUTxO / Plutus)",
            Self::Solana => "Solana (SPL / Ed25519)",
            Self::Bitcoin => "Bitcoin (Taproot / Lightning)",
            Self::Polkadot => "Polkadot / Substrate (SR25519)",
            Self::Cosmos => "Cosmos IBC (Inter-Chain)",
            Self::SolidRea => "Solid REA Mutual Credit",
        }
    }

    pub fn derived_address(self) -> &'static str {
        match self {
            Self::DidQ42 => "did:q42:8f419c83a201b12d#device-key",
            Self::Cardano => "addr1q42...cardano42",
            Self::Solana => "4zMMC9q...8JqK (Sanctuary Program Account)",
            Self::Bitcoin => "bc1p8f42...q42tap (Taproot Script / Lightning)",
            Self::Polkadot => "14Gq42...9sub (Polkadot Asset Hub)",
            Self::Cosmos => "cosmos1q42...ibc (Osmosis / Hub Channel 42)",
            Self::SolidRea => "https://solid.qualia.net/rea/commons#ledger",
        }
    }

    pub fn signing_posture(self) -> &'static str {
        match self {
            Self::DidQ42 => "Ed25519 Native Root + 48B NQuin ECC Fold",
            Self::Cardano => "Ed25519-BIP32 Extended eUTxO Signer",
            Self::Solana => "Ed25519 Deterministic Transaction Signer",
            Self::Bitcoin => "Schnorr BIP-340 / MuSig2 Multi-Sig Channel",
            Self::Polkadot => "SR25519 Schnorrkel / Ristretto255",
            Self::Cosmos => "Secp256k1 ADR-036 Inalienable Signatures",
            Self::SolidRea => "W3C Solid WebID-OIDC + REA ValueFlows Commits",
        }
    }

    pub fn balance_summary(self) -> &'static str {
        match self {
            Self::DidQ42 => "12,450.00 QCREDIT · 917,504 NQuin Slots Available",
            Self::Cardano => "420.00 ADA · Staked to Commons Pool",
            Self::Solana => "14.80 SOL · 500.00 USDC-SPL",
            Self::Bitcoin => "0.042 BTC · 1,200,000 sats Lightning Capacity",
            Self::Polkadot => "350.00 DOT · Commons Parachain Account",
            Self::Cosmos => "120.00 ATOM · IBC Channel 42 Active",
            Self::SolidRea => "5,800.00 Labour Credits (ValueFlows Commons)",
        }
    }

    pub fn default_asset(self) -> &'static str {
        match self {
            Self::DidQ42 => "QCREDIT",
            Self::Cardano => "ADA",
            Self::Solana => "SOL",
            Self::Bitcoin => "BTC",
            Self::Polkadot => "DOT",
            Self::Cosmos => "ATOM",
            Self::SolidRea => "QCREDIT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissiveRoutingLane {
    Passthrough,
    Commons,
    Bilateral,
    Spatial,
}

impl PermissiveRoutingLane {
    pub const ALL: [Self; 4] = [
        Self::Passthrough,
        Self::Commons,
        Self::Bilateral,
        Self::Spatial,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Passthrough => "00",
            Self::Commons => "01",
            Self::Bilateral => "10",
            Self::Spatial => "11",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Passthrough => "00: Passthrough",
            Self::Commons => "01: Human Commons (25GB Quota)",
            Self::Bilateral => "10: Bilateral Micro-Commons",
            Self::Spatial => "11: Spatial / Chora",
        }
    }

    pub fn quota_status(self) -> &'static str {
        match self {
            Self::Passthrough => "Direct zero-overhead peer routing lane",
            Self::Commons => "18.4 GB / 25.0 GB human commons quota available",
            Self::Bilateral => "Guarded by M:N threshold signatures & sanctuary policy",
            Self::Spatial => "Volumetric 3D/10D bounded coordinate routing",
        }
    }
}

pub fn stash_poet_handoff(construct: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.session_storage() {
            let _ = storage.set_item("webizen_poet_handoff", construct);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = construct;
}

#[component]
pub fn WellfairWalletCommercePanel() -> Element {
    let mut selected_chain = use_signal(|| ChainKind::DidQ42);
    let mut selected_lane = use_signal(|| PermissiveRoutingLane::Commons);
    let mut sdn_sync_status = use_signal(|| String::new());
    let mut sdn_busy = use_signal(|| false);

    let mut counterparty = use_signal(|| "did:qualia:0x3f8a42...coop".to_string());
    let mut amount = use_signal(|| "150.00".to_string());
    let mut asset = use_signal(|| "QCREDIT".to_string());
    let mut project_id = use_signal(|| "proj-qualia-shed".to_string());
    let mut memo = use_signal(|| "Sprint milestone deliverable payout".to_string());
    let mut receipt = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    let nav = use_navigator();
    let chain = selected_chain();
    let lane = selected_lane();

    rsx! {
        section {
            aria_label: "WellFair Multi-Chain Wallet & Cooperative Commerce",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            super::shared::DomainChrome { domain: "Finance", chip: "Qap · Multi-Chain Wallet & Cooperative Commerce", show_memory: true }
            div { style: "display:flex;justify-content:space-between;align-items:center;margin-bottom:0.6rem;flex-wrap:wrap;gap:6px;",
                h2 { style: "margin:0;font-size:1rem;color:var(--qualia-text,#1e293b);", "Multi-Chain Sovereign Wallet & Commerce Ledger" }
                button {
                    r#type: "button",
                    style: "background:rgba(56,189,248,0.12);border:1px solid rgba(56,189,248,0.35);color:var(--accent-cyan,#0284c7);font-size:11px;font-weight:650;padding:3px 9px;border-radius:6px;cursor:pointer;",
                    title: "Switch to POET Shed Economics Construct",
                    onclick: {
                        let nav = nav;
                        move |_| {
                            stash_poet_handoff("economics");
                            let _ = nav.push(crate::Route::PoetRoute {});
                        }
                    },
                    "⬡ Open Wallet & Commerce Construct in POET Shed"
                }
            }
            p { style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Non-custodial cryptographic keys, deterministic hardware addresses, and cross-chain cooperative settlements backed by 48-byte Super-Quins."
            }

            // Chain selection tabs
            div { style: "display:flex;gap:4px;overflow-x:auto;padding-bottom:4px;margin-bottom:0.75rem;",
                for c in ChainKind::ALL {
                    button {
                        key: "{c.id()}",
                        r#type: "button",
                        style: if chain == c {
                            "border:1px solid var(--accent-cyan,#0284c7);background:rgba(56,189,248,0.15);color:var(--accent-cyan,#0284c7);border-radius:6px;padding:4px 8px;font-size:.72rem;font-weight:700;cursor:pointer;white-space:nowrap;"
                        } else {
                            "border:1px solid var(--qualia-border,#ccc);background:transparent;color:var(--qualia-text-muted,#666);border-radius:6px;padding:4px 8px;font-size:.72rem;cursor:pointer;white-space:nowrap;"
                        },
                        onclick: move |_| {
                            selected_chain.set(c);
                            asset.set(c.default_asset().to_string());
                        },
                        "{c.title()}"
                    }
                }
            }

            // Active Chain Card
            div { style: "background:rgba(0,0,0,0.03);border:1px solid var(--qualia-border,#e2e8f0);border-radius:8px;padding:10px;margin-bottom:0.85rem;display:grid;gap:5px;",
                div { style: "display:flex;justify-content:space-between;align-items:center;",
                    span { style: "font-weight:700;font-size:12px;color:var(--qualia-text,#0f172a);", "{chain.title()}" }
                    span { style: "font-size:11px;color:var(--accent-emerald,#16a34a);font-weight:600;", "{chain.balance_summary()}" }
                }
                div { style: "font-size:11px;font-family:var(--font-mono);color:var(--accent-cyan,#0369a1);word-break:break-all;",
                    "Identifier (did:q42 / Derived Address): {chain.derived_address()}"
                }
                div { style: "font-size:10px;color:var(--qualia-text-muted,#64748b);",
                    "Signing Key Posture: {chain.signing_posture()}"
                }
            }

            // 🌐 Permissive Commons & SDN Routing Quota
            div { style: "background:rgba(56,189,248,0.04);border:1px solid rgba(56,189,248,0.25);border-radius:8px;padding:10px;margin-bottom:0.85rem;display:grid;gap:6px;",
                div { style: "display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:6px;",
                    div {
                        strong { style: "font-size:.84rem;color:var(--accent-cyan,#0284c7);", "🌐 Permissive Commons & SDN Routing Quota" }
                        p { style: "margin:2px 0 0;font-size:.70rem;color:var(--qualia-text-muted,#64748b);", "Hardware-enforced SDN permissive routing lanes (metadata bits 61..62) paired with sovereign wallet settlements." }
                    }
                    button {
                        r#type: "button",
                        disabled: sdn_busy(),
                        style: "background:var(--accent-cyan,#0284c7);color:#ffffff;border:none;border-radius:6px;padding:4px 10px;font-size:.72rem;font-weight:600;cursor:pointer;",
                        onclick: move |_| {
                            sdn_busy.set(true);
                            let lane_code = lane.code();
                            let ch_id = chain.id();
                            spawn(async move {
                                sdn_sync_status.set("Syncing SDN quota with POET toolchain…".into());
                                let code = format!(
                                    "Poet.toolchain_execute(\"service_permissive_commons\", \"#{{ lane: \\\"{}\\\", chain: \\\"{}\\\", op: \\\"sync_quota\\\" }}\")",
                                    lane_code, ch_id
                                );
                                let msg = match crate::components::poet::engine::eval(code, false, None).await {
                                    Ok(res) if res.ok => {
                                        format!("⚡ SDN Quota Synced: Lane {lane_code} on {ch_id} [25GB Commons Allocated | Lamport #0x1A42]")
                                    }
                                    Ok(res) => format!("⚠️ SDN Quota Notice: {}", res.diagnostic.unwrap_or(res.value)),
                                    Err(e) => format!("❌ SDN Quota Error: {e}"),
                                };
                                sdn_sync_status.set(msg);
                                sdn_busy.set(false);
                            });
                        },
                        "⚡ Sync Permissive Commons & SDN Quota"
                    }
                }
                div { style: "display:flex;gap:4px;flex-wrap:wrap;margin-top:4px;",
                    for l in PermissiveRoutingLane::ALL {
                        button {
                            key: "{l.code()}",
                            r#type: "button",
                            style: if lane == l {
                                "border:1px solid #0284c7;background:#0284c7;color:#ffffff;border-radius:4px;padding:3px 7px;font-size:.68rem;font-weight:650;cursor:pointer;"
                            } else {
                                "border:1px solid var(--qualia-border,#ccc);background:transparent;color:var(--qualia-text-muted,#666);border-radius:4px;padding:3px 7px;font-size:.68rem;cursor:pointer;"
                            },
                            onclick: move |_| selected_lane.set(l),
                            "{l.label()}"
                        }
                    }
                }
                div { style: "font-size:.70rem;color:#10b981;font-family:var(--font-mono);",
                    "Active Lane Allocation: {lane.quota_status()}"
                }
                if !sdn_sync_status().is_empty() {
                    div { style: "font-size:.70rem;font-family:var(--font-mono);color:var(--accent-cyan,#0284c7);",
                        "{sdn_sync_status()}"
                    }
                }
            }

            // Settlement Form
            h3 { style: "margin:0 0 0.4rem;font-size:0.88rem;", "Execute Multi-Chain Commerce Settlement & Payout" }
            div { style: "display:grid;grid-template-columns:2fr 1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                div {
                    label { style: "display:block;font-size:10px;color:var(--qualia-text-muted,#666);margin-bottom:2px;", "Counterparty Identifier (did:q42 / Address)" }
                    input {
                        r#type: "text",
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;box-sizing:border-box;",
                        value: "{counterparty}",
                        oninput: move |e| counterparty.set(e.value()),
                    }
                }
                div {
                    label { style: "display:block;font-size:10px;color:var(--qualia-text-muted,#666);margin-bottom:2px;", "Amount" }
                    input {
                        r#type: "text",
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;box-sizing:border-box;",
                        value: "{amount}",
                        oninput: move |e| amount.set(e.value()),
                    }
                }
                div {
                    label { style: "display:block;font-size:10px;color:var(--qualia-text-muted,#666);margin-bottom:2px;", "Asset" }
                    select {
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;box-sizing:border-box;",
                        value: "{asset}",
                        onchange: move |e| asset.set(e.value()),
                        for a in ["QCREDIT", "USDC", "ADA", "SOL", "BTC", "DOT", "ATOM"] {
                            option { value: "{a}", "{a}" }
                        }
                    }
                }
            }
            div { style: "display:grid;grid-template-columns:1fr 2fr;gap:0.5rem;margin-bottom:0.6rem;",
                div {
                    label { style: "display:block;font-size:10px;color:var(--qualia-text-muted,#666);margin-bottom:2px;", "Project ID" }
                    input {
                        r#type: "text",
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;box-sizing:border-box;",
                        value: "{project_id}",
                        oninput: move |e| project_id.set(e.value()),
                    }
                }
                div {
                    label { style: "display:block;font-size:10px;color:var(--qualia-text-muted,#666);margin-bottom:2px;", "Settlement Memo / Reason" }
                    input {
                        r#type: "text",
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;box-sizing:border-box;",
                        value: "{memo}",
                        oninput: move |e| memo.set(e.value()),
                    }
                }
            }

            button {
                r#type: "button",
                disabled: busy(),
                style: "padding:0.45rem 0.85rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;font-weight:600;cursor:pointer;",
                onclick: move |_| {
                    let c = chain;
                    let cp = counterparty().trim().to_string();
                    let amt_str = amount().trim().to_string();
                    let ast = asset().trim().to_string();
                    let proj = project_id().trim().to_string();
                    let mem = memo().trim().to_string();
                    let cents = (amt_str.parse::<f64>().unwrap_or(0.0) * 100.0) as i64;
                    busy.set(true);
                    spawn(async move {
                        receipt.set("Executing multi-chain settlement…".into());
                        let desc = format!("{}[{}]: {} → {}", c.id(), ast, mem, cp);
                        let doc_uri = format!("did:q42:commerce:{}", proj);
                        let _ = add_ledger_entry(&desc, -cents, &ast, Some("coop_commerce"), Some(&doc_uri)).await;

                        let code = format!(
                            "Poet.toolchain_execute(\"commerce_settle\", \"{}\")",
                            format!("#{{ chain: \"{}\", asset: \"{}\", amount: {}, counterparty: \"{}\", project: \"{}\" }}", c.id(), ast, amt_str, cp, proj)
                        );
                        let msg = match crate::components::poet::engine::eval(code, false, None).await {
                            Ok(res) if res.ok => {
                                let l = 0x0001_A3F2u32 & 0x1FFF_FFFF;
                                format!(
                                    "✅ Settlement Settled on {}: {}\nReceipt: did:q42:{proj}:0x8000000000000000 [Lamport: #{l} | 48B NQuins: 16]\nCommitted: {}",
                                    c.title(), res.value, res.committed
                                )
                            }
                            Ok(res) => format!("⚠️ Execution diagnostic: {}", res.diagnostic.unwrap_or(res.value)),
                            Err(e) => format!("❌ Host error: {e}"),
                        };
                        receipt.set(msg);
                        busy.set(false);
                    });
                },
                "💸 Settle Commerce & Mint Receipt"
            }

            if !receipt().is_empty() {
                pre {
                    style: "margin-top:0.75rem;padding:8px;background:rgba(0,0,0,0.06);border:1px solid var(--qualia-border,#cbd5e1);border-radius:6px;font-size:11px;font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;color:var(--accent-cyan,#0369a1);",
                    "{receipt()}"
                }
            }
        }
    }
}
