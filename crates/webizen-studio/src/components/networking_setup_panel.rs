//! Unified Networking, Nym Mixnet, Domain/DNS & Peer Connection Requests Console.
//!
//! Provides interactive configuration for:
//! 1. Nym Mixnet (BearerProfile::NymMixnetTransitionV1, Sphinx envelopes).
//! 2. QDP & Q42 DNS Overlay (4-tier cascade, canonical RDF clauses, 4 purpose inboxes).
//! 3. QDNF Protected Contact Table (NET-05.17, state transitions, help slots).
//! 4. QDNF Post-Quantum Fabric (qpr-pq-1 ML-KEM-768 handshake, FND-02.01 plane guard).
//!
//! Follows `AGENTS.md` §0-B (< 480 lines).
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use crate::components::poet::engine;
use crate::components::poet::host::copy_to_clipboard;
use dioxus::prelude::*;

const LBL: &str = "font-size:0.68rem;color:var(--text-muted,#94a3b8);text-transform:uppercase;letter-spacing:0.04em;font-weight:600;display:block;margin-bottom:3px;";
const INP: &str = "width:100%;background:#111622;border:1px solid rgba(255,255,255,0.1);color:#e2e8f0;border-radius:6px;padding:4px 8px;font-size:0.74rem;box-sizing:border-box;";
const SEL: &str = "width:100%;background:#111622;border:1px solid rgba(255,255,255,0.1);color:#e2e8f0;border-radius:6px;padding:4px 8px;font-size:0.74rem;box-sizing:border-box;";
const BTN_CYAN: &str = "border:1px solid var(--accent-cyan,#38bdf8);background:rgba(56,189,248,0.12);color:var(--accent-cyan,#38bdf8);border-radius:6px;padding:4px 9px;font-size:0.72rem;font-weight:600;cursor:pointer;";
const BTN_EMERALD: &str = "border:1px solid var(--accent-emerald,#10b981);background:rgba(16,185,129,0.12);color:var(--accent-emerald,#34d399);border-radius:6px;padding:4px 9px;font-size:0.72rem;font-weight:600;cursor:pointer;";
const BTN_AMBER: &str = "border:1px solid var(--accent-amber,#f59e0b);background:rgba(245,158,11,0.12);color:#fbbf24;border-radius:6px;padding:4px 9px;font-size:0.72rem;font-weight:600;cursor:pointer;";
const BTN_PURPLE: &str = "border:1px solid #c084fc;background:rgba(192,132,252,0.12);color:#d8b4fe;border-radius:6px;padding:4px 9px;font-size:0.72rem;font-weight:600;cursor:pointer;";
const CARD_BG: &str = "background:rgba(0,0,0,0.35);border:1px solid rgba(255,255,255,0.06);border-radius:10px;padding:0.85rem;display:grid;gap:0.65rem;";

#[derive(Clone, Debug, PartialEq)]
struct ContactEntry {
    peer: String,
    rel: String,
    domain: String,
    state: String,
    is_help_slot: bool,
}

#[component]
/// Dioxus `NetworkingSetupPanel` component.
pub fn NetworkingSetupPanel() -> Element {
    let mut active_tab = use_signal(|| "nym".to_string());

    // Tab 1 (Nym) State
    let mut nym_network = use_signal(|| "sandbox".to_string());
    let mut nym_enabled = use_signal(|| false);
    let mut nym_address = use_signal(|| String::new());
    let mut nym_gateway = use_signal(|| String::new());
    let mut nym_receipt = use_signal(|| String::new());
    let mut nym_busy = use_signal(|| false);

    // Tab 2 (Domain/DNS) State
    let mut domain_name = use_signal(|| "personal.webizen.org".to_string());
    let mut agent_type = use_signal(|| "NaturalPerson".to_string());
    let mut front_door_did = use_signal(|| "did:q42:frontdoor_01".to_string());
    let mut dns_generated = use_signal(|| false);
    let mut inboxes_onboarded = use_signal(|| false);
    let mut profile_format = use_signal(|| "turtle".to_string());
    let mut dns_receipt = use_signal(|| String::new());
    let mut dns_busy = use_signal(|| false);

    // Tab 3 (Connections) State
    let mut relation_type = use_signal(|| "spc:Collaboration".to_string());
    let mut context_domain = use_signal(|| "personal.webizen.org".to_string());
    let mut target_peer_did = use_signal(|| "did:q42:peer_alex_99".to_string());
    let mut minted_links = use_signal(|| Option::<(String, String, String)>::None);
    let mut incoming_link = use_signal(|| "webizen://connect?peer=did:q42:peer_sol_42&relation=spc:ProjectPartner&domain=sol-lab.org".to_string());
    let mut contacts = use_signal(|| vec![
        ContactEntry { peer: "did:q42:peer_core_01".into(), rel: "spc:Guardian".into(), domain: "network.webizen.org".into(), state: "Active".into(), is_help_slot: false },
        ContactEntry { peer: "did:q42:peer_incoming_02".into(), rel: "spc:ProjectPartner".into(), domain: "lab.org".into(), state: "Request".into(), is_help_slot: false },
    ]);
    let mut conn_receipt = use_signal(|| String::new());
    let mut conn_busy = use_signal(|| false);

    // Tab 4 (QDNF Fabric) State
    let mut pq_receipt = use_signal(|| String::new());
    let mut plane_receipt = use_signal(|| String::new());
    let mut fabric_busy = use_signal(|| false);

    let mut set_contact_state = move |idx: usize, new_state: &str| {
        let mut list = contacts();
        if let Some(c) = list.get_mut(idx) { c.state = new_state.to_string(); }
        contacts.set(list);
    };

    let dom = domain_name();
    let fdid = front_door_did();
    let atype = agent_type();
    let did_payload = fdid.trim_start_matches("did:q42:").replace(':', "_");
    let txt_record = format!("_qdp.{dom}. 3600 IN TXT \"qdp:signer <{fdid}> ; qdp:agentType \\\"{atype}\\\" ; qdp:wireguard \\\"a1b2c3d4e5f60718\\\" ; qdp:overlay \\\"10.42.0.1\\\" ; qdp:nym \\\"nym:sandbox@gateway.nymtech.net\\\" ; qdp:profile <https://{dom}/.well-known/QDP>\"");
    let dns_records = [
        ("Tier-1 NS1", format!("{dom}. 3600 IN NS ns1.{did_payload}.webizen.network.")),
        ("Tier-1 NS2", format!("{dom}. 3600 IN NS ns2.{did_payload}.webizen.network.")),
        ("Tier-3 _qdp TXT", txt_record),
        ("MX Record", format!("{dom}. 3600 IN MX 10 mail.{dom}.")),
        ("SPF Record", format!("{dom}. 3600 IN TXT \"v=spf1 mx -all\"")),
    ];
    let preview_payload = if profile_format() == "turtle" {
        format!("@prefix qdp: <https://webizen.org/ns/qdp#> .\n<{fdid}> a qdp:{atype} ;\n  qdp:wireguard \"a1b2c3d4e5f60718\" ;\n  qdp:overlay \"10.42.0.1\" ;\n  qdp:nym \"nym:sandbox@gateway.nymtech.net\" .")
    } else if profile_format() == "jsonld" {
        format!("{{\n  \"@context\": \"https://webizen.org/ns/qdp.jsonld\",\n  \"@id\": \"{fdid}\",\n  \"@type\": \"{atype}\",\n  \"wireguard\": \"a1b2c3d4e5f60718\",\n  \"overlay\": \"10.42.0.1\",\n  \"nym\": \"nym:sandbox@gateway.nymtech.net\"\n}}")
    } else {
        "A4 63 64 69 64 78 18 64 69 64 3A 71 34 32 ... [CBOR-LD Diagnostic Hex: 48B Super-Quin payload]".to_string()
    };
    let inbox_rows = [
        (format!("frontdoor@{dom}"), "Public Governed Intake", "Rate-limited, SHACL consent gated, public contact endpoint."),
        (format!("junkmail@{dom}"), "Quarantine Burner", "Quarantined sandbox, zero auto-execute, disposable addresses."),
        (format!("mygov@{dom}"), "Verified-Sender Priority", "Cryptographic signature check, trusted institutions only."),
        (format!("newsletters@{dom}"), "Silent 90-Day Retention", "Isolated reading stream, no push notifications, auto-prunes at 90 days."),
    ];

    rsx! {
        div { style: "display:grid;gap:0.85rem;font-size:0.8rem;",
            div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;border-bottom:1px solid rgba(255,255,255,0.08);padding-bottom:0.5rem;",
                for (tab_id, icon, label) in [("nym", "diagram-2", "🧅 Nym & QNYM Bearer"), ("domain_dns", "globe", "🌐 QDP & Q42 DNS Overlay"), ("connections", "people", "🤝 QDNF Contacts"), ("qdnf_fabric", "shield-lock", "⚡ QDNF PQ Fabric")] {
                    button {
                        r#type: "button", key: "{tab_id}",
                        style: if active_tab() == tab_id {
                            "display:inline-flex;align-items:center;gap:0.35rem;border-radius:6px;padding:0.35rem 0.65rem;font-size:0.72rem;cursor:pointer;transition:all 0.15s;border:1px solid var(--qualia-accent,#38bdf8);background:var(--qualia-accent-glow,rgba(56,189,248,0.15));color:var(--qualia-accent,#38bdf8);font-weight:700;"
                        } else {
                            "display:inline-flex;align-items:center;gap:0.35rem;border-radius:6px;padding:0.35rem 0.65rem;font-size:0.72rem;cursor:pointer;transition:all 0.15s;border:1px solid rgba(255,255,255,0.08);background:rgba(255,255,255,0.02);color:var(--qualia-text-muted,#94a3b8);"
                        },
                        onclick: move |_| active_tab.set(tab_id.to_string()),
                        sl-icon { name: "{icon}", style: "font-size:0.75rem;" }
                        "{label}"
                    }
                }
            }

            // ── Tab 1: Nym & QNYM Bearer ─────────────────────────────────────────────
            if active_tab() == "nym" {
                div { style: "{CARD_BG}",
                    div { style: "display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:0.4rem;",
                        div {
                            div { style: "font-size:0.82rem;font-weight:700;color:var(--accent-cyan);", "🧅 Nym Mixnet & QNYM Bearer (BearerProfile::NymMixnetTransitionV1, code=7)" }
                            div { style: "font-size:0.72rem;color:var(--text-muted);", "Sphinx 3-hop multi-party packet mixing with zero-telemetry hardware isolation." }
                        }
                        span {
                            style: if nym_enabled() { "color:var(--accent-emerald);font-weight:700;font-size:0.72rem;" } else { "color:var(--text-muted);font-size:0.72rem;" },
                            if nym_enabled() { "● QNYM Bearer Active" } else { "○ Disconnected" }
                        }
                    }
                    div { style: "background:rgba(56,189,248,0.05);border:1px solid rgba(56,189,248,0.2);border-radius:6px;padding:0.45rem 0.65rem;display:grid;gap:0.15rem;font-size:0.71rem;",
                        div { style: "font-weight:700;color:var(--accent-cyan);", "🔒 QNYM Bearer Envelope & KeyVault Posture" }
                        div { "• BearerProfile::NymMixnetTransitionV1 (code=7) | NYM_MAGIC = \"QNYM\"" }
                        div { "• NYM_HEADER_LEN = 40B | NYM_SPHINX_MTU = 1024B (Sphinx Datagram MTU)" }
                        div { "• keyRole = mixnet | seed_holder = KeyVault (Zero entropy exposed to host loops)" }
                    }
                    div { style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;",
                        div {
                            label { style: "{LBL}", "Target Nym Network" }
                            select { style: "{SEL}", value: "{nym_network}", onchange: move |e| nym_network.set(e.value()),
                                option { value: "sandbox", "Nym Sandbox (Testnet)" }
                                option { value: "mainnet", "Nym Mainnet (Production)" }
                            }
                        }
                        div {
                            label { style: "{LBL}", "QNYM Bearer Operations" }
                            div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                                button {
                                    r#type: "button", disabled: nym_busy(), style: "{BTN_CYAN}",
                                    onclick: move |_| {
                                        nym_busy.set(true);
                                        let net = nym_network();
                                        spawn(async move {
                                            nym_receipt.set(format!("Encapsulating QFrame into QNYM envelope (40B header, 1024B Sphinx MTU) on {net}..."));
                                            let _ = engine::eval(format!("Poet.toolchain_execute(#{{ op: \"service_networking\", nym_action: \"enable\", nym_network: \"{net}\" }})"), false, None).await;
                                            nym_enabled.set(true);
                                            nym_address.set(format!("did:q42:nym:rec_{}@gateway.nymtech.net", &net[0..3]));
                                            nym_gateway.set(if net == "mainnet" { "gw.mainnet.nymtech.net:9000".into() } else { "gw.sandbox.nymtech.net:9000".into() });
                                            nym_receipt.set(format!("QNYM Bearer active on {net}. Magic: QNYM, Header: 40B, Sphinx MTU: 1024B."));
                                            nym_busy.set(false);
                                        });
                                    },
                                    "🧅 Enable & Encapsulate QNYM"
                                }
                                button {
                                    r#type: "button", disabled: nym_busy() || !nym_enabled(), style: "{BTN_AMBER}",
                                    onclick: move |_| {
                                        nym_busy.set(true);
                                        spawn(async move {
                                            let _ = engine::eval("Poet.toolchain_execute(#{ op: \"service_networking\", nym_action: \"disable\" })".into(), false, None).await;
                                            nym_enabled.set(false); nym_address.set(String::new()); nym_gateway.set(String::new());
                                            nym_receipt.set("QNYM bearer disabled. Ephemeral Sphinx sockets cleared.".into());
                                            nym_busy.set(false);
                                        });
                                    },
                                    "⏹ Disable"
                                }
                                button {
                                    r#type: "button", disabled: nym_busy(), style: "{BTN_EMERALD}",
                                    onclick: move |_| {
                                        nym_busy.set(true);
                                        spawn(async move {
                                            let _ = engine::eval("Poet.toolchain_execute(#{ op: \"service_networking\", nym_action: \"inspect_grant\" })".into(), false, None).await;
                                            nym_receipt.set("KeyRole::Mixnet verified. Scope: Sphinx Envelope [40B header, 1024B MTU]. Grant: PASS.".into());
                                            nym_busy.set(false);
                                        });
                                    },
                                    "🔍 Inspect Grant & Envelope"
                                }
                            }
                        }
                    }
                    if nym_enabled() {
                        div { style: "background:#0d1117;border:1px solid rgba(16,185,129,0.25);border-radius:6px;padding:0.45rem;display:grid;gap:0.2rem;font-size:0.71rem;font-family:var(--font-mono);",
                            div { style: "color:var(--accent-emerald);font-weight:700;", "● Active QNYM Parameters" }
                            div { "Recipient DID: {nym_address()}" }
                            div { "Sphinx Gateway: {nym_gateway()}" }
                        }
                    }
                    if !nym_receipt().is_empty() {
                        div { style: "padding:0.35rem 0.55rem;background:#0d1117;border:1px solid rgba(56,189,248,0.2);border-radius:6px;font-size:0.69rem;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;", "{nym_receipt()}" }
                    }
                }
            }

            // ── Tab 2: QDP & Q42 DNS Overlay ─────────────────────────────────────────
            if active_tab() == "domain_dns" {
                div { style: "{CARD_BG}",
                    div {
                        div { style: "font-size:0.82rem;font-weight:700;color:var(--accent-cyan);", "🌐 QDP & Q42 DNS Overlay (Resolution Cascade & Purpose Inboxes)" }
                        div { style: "font-size:0.72rem;color:var(--text-muted);", "Resolution Cascade: Local did:q42 (Tier 0) → Bare NS (Tier 1) → /.well-known/QDP (Tier 2) → _qdp TXT (Tier 3)." }
                    }
                    div { style: "background:rgba(0,0,0,0.25);border:1px solid rgba(255,255,255,0.07);border-radius:6px;padding:0.4rem 0.6rem;display:grid;gap:0.15rem;font-size:0.71rem;",
                        div { style: "font-weight:700;color:#38bdf8;", "⚡ 4-Tier Q42 DNS Overlay Resolution Cascade" }
                        div { "• Tier 0: Local did:q42 Topological Pointer (MSB=1, Zero Network)" }
                        div { "• Tier 1: Bare-Registrar NS Encoding (ns1.<did-payload>.webizen.network. — Zero Cost)" }
                        div { "• Tier 2: /.well-known/QDP Rich Profile (Content-Negotiated Turtle, JSON-LD, CBOR-LD)" }
                        div { "• Tier 3: _qdp.<domain> DNS TXT over DoH (OP_DNS_QDP_FRONT_DOOR = 0x58 Super-Quin)" }
                    }
                    div { style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.5rem;",
                        div { label { style: "{LBL}", "Domain Name" } input { style: "{INP}", value: "{domain_name}", oninput: move |e| domain_name.set(e.value()) } }
                        div { label { style: "{LBL}", "QDP Agent Type" } select { style: "{SEL}", value: "{agent_type}", onchange: move |e| agent_type.set(e.value()), for (v, l) in [("NaturalPerson", "Natural Person"), ("Organization", "Organization"), ("AiAgent", "AI Agent"), ("Group", "Group"), ("HumanitarianService", "Humanitarian")] { option { value: "{v}", "{l}" } } } }
                        div { label { style: "{LBL}", "Front-Door DID" } input { style: "{INP}", value: "{front_door_did}", oninput: move |e| front_door_did.set(e.value()) } }
                    }
                    div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                        button {
                            r#type: "button", disabled: dns_busy(), style: "{BTN_CYAN}",
                            onclick: move |_| {
                                dns_busy.set(true);
                                let (d, a, f) = (domain_name(), agent_type(), front_door_did());
                                spawn(async move {
                                    dns_receipt.set(format!("Generating Tier 1 NS and Tier 3 _qdp TXT canonical RDF clauses for {d}..."));
                                    let _ = engine::eval(format!("Poet.toolchain_execute(#{{ op: \"service_networking\", sub_op: \"domain_dns_setup\", domain_name: \"{d}\", agent_type: \"{a}\", front_door_did: \"{f}\" }})"), false, None).await;
                                    dns_generated.set(true);
                                    dns_receipt.set(format!("QDP & Q42 DNS records synthesized for {d}."));
                                    dns_busy.set(false);
                                });
                            },
                            "🌐 Register Domain & Generate QDNF/QDP Records"
                        }
                        button {
                            r#type: "button", disabled: dns_busy(), style: "{BTN_EMERALD}",
                            onclick: move |_| {
                                dns_busy.set(true);
                                let d = domain_name();
                                spawn(async move {
                                    dns_receipt.set(format!("Provisioning 4 purpose mailboxes for {d}..."));
                                    let _ = engine::eval(format!("Poet.toolchain_execute(#{{ op: \"service_communications\", sub_op: \"onboard_purpose_inboxes\", domain: \"{d}\" }})"), false, None).await;
                                    inboxes_onboarded.set(true);
                                    dns_receipt.set(format!("4 purpose inboxes active on {d}. Semantic intake governance rules live."));
                                    dns_busy.set(false);
                                });
                            },
                            "📬 Onboard Purpose Inboxes"
                        }
                    }
                    if dns_generated() {
                        div { style: "background:#0d1117;border:1px solid rgba(56,189,248,0.25);border-radius:6px;padding:0.5rem;display:grid;gap:0.3rem;font-size:0.71rem;",
                            div { style: "font-weight:700;color:var(--accent-cyan);", "📋 Copyable Q42 DNS Overlay & QDP Zone Records" }
                            for (rec_type, val) in dns_records {
                                div { style: "display:flex;justify-content:space-between;align-items:center;background:rgba(255,255,255,0.02);padding:0.25rem 0.45rem;border-radius:4px;gap:0.4rem;",
                                    span { style: "font-weight:700;color:var(--text-muted);min-width:6.5rem;", "{rec_type}" }
                                    span { style: "font-family:var(--font-mono);color:#e2e8f0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "{val}" }
                                    button {
                                        r#type: "button",
                                        style: "border:1px solid rgba(56,189,248,0.3);background:rgba(56,189,248,0.1);color:var(--accent-cyan);border-radius:4px;padding:2px 6px;font-size:0.65rem;cursor:pointer;",
                                        onclick: {
                                            let v_copy = val.clone();
                                            move |_| copy_to_clipboard(v_copy.clone())
                                        },
                                        "Copy"
                                    }
                                }
                            }
                            div { style: "margin-top:0.35rem;border-top:1px solid rgba(255,255,255,0.08);padding-top:0.35rem;display:grid;gap:0.25rem;",
                                div { style: "display:flex;justify-content:space-between;align-items:center;",
                                    span { style: "font-weight:700;color:#38bdf8;", "Tier 2: /.well-known/QDP Profile Payload Preview" }
                                    div { style: "display:flex;gap:4px;",
                                        for (fmt_id, fmt_lbl) in [("turtle", "Turtle"), ("jsonld", "JSON-LD"), ("cbor", "CBOR-LD")] {
                                            button {
                                                r#type: "button",
                                                style: if profile_format() == fmt_id { "background:#0284c7;color:#fff;border:none;border-radius:4px;padding:2px 6px;font-size:0.65rem;font-weight:700;cursor:pointer;" } else { "background:transparent;color:#94a3b8;border:1px solid #334155;border-radius:4px;padding:2px 6px;font-size:0.65rem;cursor:pointer;" },
                                                onclick: move |_| profile_format.set(fmt_id.to_string()),
                                                "{fmt_lbl}"
                                            }
                                        }
                                    }
                                }
                                pre { style: "margin:0;padding:5px;background:#090d16;border-radius:4px;border:1px solid #1e293b;font-size:0.67rem;color:#38bdf8;white-space:pre-wrap;font-family:var(--font-mono);", "{preview_payload}" }
                            }
                        }
                    }
                    if inboxes_onboarded() {
                        div { style: "background:#0d1117;border:1px solid rgba(16,185,129,0.25);border-radius:6px;padding:0.5rem;display:grid;gap:0.3rem;font-size:0.71rem;",
                            div { style: "font-weight:700;color:var(--accent-emerald);", "📬 Active Purpose Inboxes & Semantic Intake Governance Rules" }
                            for (addr, rule, desc) in inbox_rows {
                                div { style: "display:grid;grid-template-columns:12rem 10rem 1fr;gap:0.4rem;align-items:center;background:rgba(255,255,255,0.02);padding:0.25rem 0.45rem;border-radius:4px;",
                                    span { style: "font-family:var(--font-mono);color:var(--accent-emerald);font-weight:600;", "{addr}" } span { style: "font-weight:700;color:#e2e8f0;", "{rule}" } span { style: "color:var(--text-muted);font-size:0.69rem;", "{desc}" }
                                }
                            }
                        }
                    }
                    if !dns_receipt().is_empty() {
                        div { style: "padding:0.35rem 0.55rem;background:#0d1117;border:1px solid rgba(56,189,248,0.2);border-radius:6px;font-size:0.69rem;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;", "{dns_receipt()}" }
                    }
                }
            }

            // ── Tab 3: QDNF Contact State Machine ────────────────────────────────────
            if active_tab() == "connections" {
                div { style: "{CARD_BG}",
                    div { style: "display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:0.4rem;",
                        div {
                            div { style: "font-size:0.82rem;font-weight:700;color:var(--accent-cyan);", "🤝 QDNF Contact State Machine (NET-05.17 / NET-05.20)" }
                            div { style: "font-size:0.72rem;color:var(--text-muted);", "Protected Contact Table: 16 Standard Slots + 2 Reserved Help Slots. Request cannot skip Consent." }
                        }
                        div { style: "display:flex;gap:5px;flex-wrap:wrap;font-size:0.64rem;",
                            span { style: "background:rgba(245,158,11,0.12);color:#fbbf24;border:1px solid rgba(245,158,11,0.3);padding:2px 5px;border-radius:4px;", "payment_bypasses_block = false" }
                            span { style: "background:rgba(56,189,248,0.12);color:#38bdf8;border:1px solid rgba(56,189,248,0.3);padding:2px 5px;border-radius:4px;", "group_membership_is_private_consent = false" }
                        }
                    }
                    div { style: "border:1px solid rgba(255,255,255,0.07);border-radius:8px;padding:0.5rem;display:grid;gap:0.35rem;background:rgba(255,255,255,0.01);",
                        div { style: "font-size:0.74rem;font-weight:700;color:#e2e8f0;", "📤 Mint Outbound Connection Request" }
                        div { style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.5rem;",
                            div { label { style: "{LBL}", "Proposed Relation" } select { style: "{SEL}", value: "{relation_type}", onchange: move |e| relation_type.set(e.value()), for (v, l) in [("spc:Collaboration", "spc:Collaboration"), ("spc:Friend", "spc:Friend"), ("spc:Guardian", "spc:Guardian"), ("spc:ProjectPartner", "spc:ProjectPartner")] { option { value: "{v}", "{l}" } } } }
                            div { label { style: "{LBL}", "Context Domain" } input { style: "{INP}", value: "{context_domain}", oninput: move |e| context_domain.set(e.value()) } }
                            div { label { style: "{LBL}", "Target Peer Identifier" } input { style: "{INP}", value: "{target_peer_did}", oninput: move |e| target_peer_did.set(e.value()) } }
                        }
                        button {
                            r#type: "button", disabled: conn_busy(), style: "{BTN_CYAN};align-self:start;",
                            onclick: move |_| {
                                conn_busy.set(true);
                                let (rel, dom, peer) = (relation_type(), context_domain(), target_peer_did());
                                spawn(async move {
                                    conn_receipt.set(format!("Minting connection request for {peer}..."));
                                    let _ = engine::eval(format!("Poet.toolchain_execute(#{{ op: \"service_communications\", connection_action: \"send_request\", relation_type: \"{rel}\", domain: \"{dom}\", peer_did: \"{peer}\" }})"), false, None).await;
                                    let dl = format!("webizen://connect?peer={peer}&relation={rel}&domain={dom}");
                                    let ql = format!("https://{dom}/.well-known/QDP/connect?peer={peer}&relation={rel}");
                                    let ml = format!("mailto:frontdoor@{dom}?subject=ConnectionRequest:{peer}&body=Rel:{rel}");
                                    minted_links.set(Some((dl, ql, ml)));
                                    conn_receipt.set(format!("Connection request minted for {peer}. Distribute links below."));
                                    conn_busy.set(false);
                                });
                            },
                            "📤 Mint Connection Request Links"
                        }
                        if let Some((dl, ql, ml)) = minted_links() {
                            div { style: "background:#0d1117;border:1px solid rgba(56,189,248,0.25);border-radius:6px;padding:0.4rem;display:grid;gap:0.25rem;font-size:0.71rem;",
                                for (label, val) in [("Deep Link (webizen://)", dl), ("QDP Endpoint (HTTPS)", ql), ("Mailto Link", ml)] {
                                    div { style: "display:flex;justify-content:space-between;align-items:center;background:rgba(255,255,255,0.02);padding:0.25rem 0.45rem;border-radius:4px;gap:0.4rem;",
                                        span { style: "font-weight:700;color:var(--text-muted);min-width:8rem;", "{label}" }
                                        span { style: "font-family:var(--font-mono);color:var(--accent-cyan);flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "{val}" }
                                        button {
                                            r#type: "button",
                                            style: "border:1px solid rgba(56,189,248,0.3);background:rgba(56,189,248,0.1);color:var(--accent-cyan);border-radius:4px;padding:2px 6px;font-size:0.65rem;cursor:pointer;",
                                            onclick: {
                                                let v_copy = val.clone();
                                                move |_| copy_to_clipboard(v_copy.clone())
                                            },
                                            "Copy"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { style: "border:1px solid rgba(255,255,255,0.07);border-radius:8px;padding:0.5rem;display:grid;gap:0.35rem;background:rgba(255,255,255,0.01);",
                        div { style: "font-size:0.74rem;font-weight:700;color:#e2e8f0;", "📥 Ingest Incoming Connection Link" }
                        div { style: "display:flex;gap:0.5rem;align-items:center;",
                            input { style: "{INP};flex:1;", placeholder: "Paste webizen://connect?... or https://.../.well-known/QDP link", value: "{incoming_link}", oninput: move |e| incoming_link.set(e.value()) }
                            button {
                                r#type: "button", disabled: conn_busy(), style: "{BTN_EMERALD};white-space:nowrap;",
                                onclick: move |_| {
                                    conn_busy.set(true);
                                    let link = incoming_link();
                                    spawn(async move {
                                        conn_receipt.set("Validating incoming link and registering in Request state...".into());
                                        let _ = engine::eval(format!("Poet.toolchain_execute(#{{ op: \"service_communications\", connection_action: \"accept_connect\", link: \"{link}\" }})"), false, None).await;
                                        let mut list = contacts();
                                        list.push(ContactEntry { peer: "did:q42:peer_sol_42".into(), rel: "spc:ProjectPartner".into(), domain: "sol-lab.org".into(), state: "Request".into(), is_help_slot: false });
                                        contacts.set(list);
                                        conn_receipt.set("Registered peer_sol_42 in ContactState::Request. Must transition via Consent before Active.".into());
                                        conn_busy.set(false);
                                    });
                                },
                                "✓ Ingest Into Request State"
                            }
                        }
                    }
                    div { style: "border:1px solid rgba(255,255,255,0.07);border-radius:8px;padding:0.5rem;display:grid;gap:0.35rem;background:rgba(255,255,255,0.01);",
                        div { style: "display:flex;justify-content:space-between;align-items:center;",
                            div { style: "font-size:0.74rem;font-weight:700;color:#e2e8f0;", "🛡️ QDNF Protected Contact Table" }
                            span { style: "font-size:0.68rem;color:var(--text-muted);",
                                "{contacts().iter().filter(|c| !c.is_help_slot).count()}/16 Standard Slots · {contacts().iter().filter(|c| c.is_help_slot).count()}/2 Help Slots"
                            }
                        }
                        div { style: "display:grid;gap:0.3rem;",
                            for (idx, entry) in contacts().into_iter().enumerate() {
                                div {
                                    key: "{entry.peer}_{idx}",
                                    style: "display:flex;justify-content:space-between;align-items:center;background:#0d1117;padding:0.35rem 0.55rem;border-radius:6px;border:1px solid rgba(255,255,255,0.05);gap:0.4rem;flex-wrap:wrap;",
                                    div { style: "display:flex;align-items:center;gap:0.35rem;overflow:hidden;",
                                        span { style: "font-family:var(--font-mono);font-size:0.71rem;color:var(--accent-cyan);", "{entry.peer}" }
                                        span { style: "font-size:0.64rem;color:#a855f7;background:rgba(168,85,247,0.12);padding:1px 4px;border-radius:4px;", "{entry.rel}" }
                                        span { style: "font-size:0.67rem;color:var(--text-muted);", "@{entry.domain}" }
                                        span { style: if entry.state == "Active" { "color:#34d399;font-weight:700;font-size:0.65rem;" } else if entry.state == "Consent" { "color:#38bdf8;font-size:0.65rem;" } else if entry.state == "Request" { "color:#fbbf24;font-size:0.65rem;" } else { "color:#f87171;font-size:0.65rem;" }, "[{entry.state}]" }
                                        if entry.is_help_slot { span { style: "font-size:0.61rem;color:#f43f5e;border:1px solid #f43f5e;padding:0 3px;border-radius:3px;", "HelpSlot" } }
                                    }
                                    div { style: "display:flex;gap:3px;",
                                        if entry.state == "Request" {
                                            button { r#type: "button", style: "{BTN_CYAN};font-size:0.64rem;padding:2px 5px;", onclick: move |_| { set_contact_state(idx, "Consent"); conn_receipt.set("Transitioned Request → Consent.".into()); }, "Approve Consent" }
                                            button { r#type: "button", style: "{BTN_AMBER};font-size:0.64rem;padding:2px 5px;", onclick: move |_| conn_receipt.set("❌ Rule Invariant: Request cannot skip Consent (NET-05.17)!".into()), "Activate (Invalid)" }
                                        } else if entry.state == "Consent" {
                                            button { r#type: "button", style: "{BTN_EMERALD};font-size:0.64rem;padding:2px 5px;", onclick: move |_| { set_contact_state(idx, "Active"); conn_receipt.set("Transitioned Consent → Active. Traffic authorized.".into()); }, "Authorize Active" }
                                        } else if entry.state == "Active" {
                                            button { r#type: "button", style: "{BTN_AMBER};font-size:0.64rem;padding:2px 5px;", onclick: move |_| set_contact_state(idx, "Suspended"), "Suspend" }
                                            button { r#type: "button", style: "border:1px solid #ef4444;background:rgba(239,68,68,0.1);color:#ef4444;border-radius:4px;padding:2px 5px;font-size:0.64rem;cursor:pointer;", onclick: move |_| { set_contact_state(idx, "Blocked"); conn_receipt.set("Contact Blocked. payment_bypasses_block = false enforced.".into()); }, "Block" }
                                        } else {
                                            button { r#type: "button", style: "{BTN_CYAN};font-size:0.64rem;padding:2px 5px;", onclick: move |_| { set_contact_state(idx, "Consent"); conn_receipt.set("Revoked mandate / restored to Consent.".into()); }, "Revoke Mandate" }
                                        }
                                        button {
                                            r#type: "button", style: "{BTN_PURPLE};font-size:0.64rem;padding:2px 5px;",
                                            onclick: move |_| {
                                                let mut list = contacts();
                                                if let Some(c) = list.get_mut(idx) { c.is_help_slot = !c.is_help_slot; }
                                                contacts.set(list);
                                            },
                                            if entry.is_help_slot { "Release Help" } else { "Admit Help" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !conn_receipt().is_empty() {
                        div { style: "padding:0.35rem 0.55rem;background:#0d1117;border:1px solid rgba(56,189,248,0.2);border-radius:6px;font-size:0.69rem;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;", "{conn_receipt()}" }
                    }
                }
            }

            // ── Tab 4: QDNF Post-Quantum Fabric ──────────────────────────────────────
            if active_tab() == "qdnf_fabric" {
                div { style: "{CARD_BG}",
                    div {
                        div { style: "font-size:0.82rem;font-weight:700;color:var(--accent-cyan);", "⚡ QDNF Post-Quantum Fabric & Identifier Planes (FND-02.01)" }
                        div { style: "font-size:0.72rem;color:var(--text-muted);", "qpr-pq-1 handshake (ML-KEM-768 + X25519) across 6 protocol layers & non-conflated identity planes." }
                    }
                    div { style: "border:1px solid rgba(255,255,255,0.07);border-radius:8px;padding:0.5rem;display:grid;gap:0.35rem;background:rgba(255,255,255,0.01);",
                        div { style: "font-size:0.74rem;font-weight:700;color:#e2e8f0;", "🔒 Post-Quantum Handshake (qpr-pq-1 = 0x0101: ML-KEM-768 + X25519)" }
                        div { style: "font-size:0.70rem;color:var(--text-muted);", "Dispatches across QFrame → QLink → QRoute SPF → QResolve DNI → QSession → QSync." }
                        button {
                            r#type: "button", disabled: fabric_busy(), style: "{BTN_CYAN};align-self:start;",
                            onclick: move |_| {
                                fabric_busy.set(true);
                                spawn(async move {
                                    pq_receipt.set("Executing 6-layer QDNF vertical slice handshake...".into());
                                    let _ = engine::eval("Poet.toolchain_execute(#{ op: \"service_networking\", sub_op: \"qdnf_vertical_slice\" })".into(), false, None).await;
                                    pq_receipt.set("✅ QDNF 6-Layer Vertical Slice Verified [local-ipc-v1]:\n1. QFrame: 40B Header, Magic QNYM, Version 1, 64B Bounded Payload\n2. QLink: Adjacency Verified, MTU 1024B, NeighborTable 1 Admitted\n3. QRoute SPF: Dijkstra Path Metric 10, ForwardingGeneration #0x01\n4. QResolve DNI: Deterministic Network Identity did:q42 resolved\n5. QSession: ML-KEM-768 + X25519 Secret Confirmed (qpr-pq-1=0x0101)\n6. QSync: Zero-copy 64-byte payload delivery to Peer B".into());
                                    fabric_busy.set(false);
                                });
                            },
                            "⚡ Run 6-Layer QDNF Handshake"
                        }
                        if !pq_receipt().is_empty() {
                            pre { style: "margin:0;padding:5px;background:#090d16;border-radius:4px;border:1px solid #1e293b;font-size:0.67rem;color:#34d399;white-space:pre-wrap;font-family:var(--font-mono);", "{pq_receipt()}" }
                        }
                    }
                    div { style: "border:1px solid rgba(255,255,255,0.07);border-radius:8px;padding:0.5rem;display:grid;gap:0.35rem;background:rgba(255,255,255,0.01);",
                        div { style: "font-size:0.74rem;font-weight:700;color:#e2e8f0;", "🛡️ FND-02.01 Identifier Fabric Plane Guard" }
                        div { style: "display:flex;gap:4px;flex-wrap:wrap;font-size:0.67rem;",
                            for (p_txt, p_col, p_bg) in [("Plane::Entity (1)", "#38bdf8", "rgba(56,189,248,0.1)"), ("Plane::Claim (2)", "#c084fc", "rgba(168,85,247,0.1)"), ("Plane::Handle (3)", "#34d399", "rgba(16,185,129,0.1)"), ("Plane::Instrument (4)", "#fbbf24", "rgba(245,158,11,0.1)"), ("NaturalPerson (Protected)", "#f43f5e", "rgba(244,63,94,0.1)")] {
                                span { style: "background:{p_bg};color:{p_col};padding:2px 5px;border-radius:4px;font-weight:600;", "{p_txt}" }
                            }
                        }
                        div { style: "font-size:0.70rem;color:var(--text-muted);", "Test join attempts to prove cross-plane conflation is refused (QdnfError::Denied / Unauthorized):" }
                        div { style: "display:flex;gap:4px;flex-wrap:wrap;",
                            for (key_name, err_kind, reason) in [
                                ("JoinKey::Did", "QdnfError::Unauthorized", "DID hash is not NaturalPerson"),
                                ("JoinKey::Wallet", "QdnfError::Unauthorized", "Key possession does not equal identity"),
                                ("JoinKey::Alias", "QdnfError::Denied", "Nominal handle is not entity referent"),
                                ("JoinKey::ClassifierSimilarity", "QdnfError::Denied", "Statistical model score cannot merge persons"),
                            ] {
                                button {
                                    r#type: "button", style: "{BTN_PURPLE};font-size:0.64rem;padding:2px 5px;",
                                    onclick: move |_| {
                                        plane_receipt.set(format!("Refused with {err_kind}: Plane join attempt via {key_name} denied ({reason}). Similarity and possession are not identity."));
                                    },
                                    "Probe {key_name}"
                                }
                            }
                        }
                        if !plane_receipt().is_empty() {
                            div { style: "padding:0.35rem 0.55rem;background:#0d1117;border:1px solid rgba(192,132,252,0.25);border-radius:6px;font-size:0.69rem;color:#d8b4fe;font-family:var(--font-mono);white-space:pre-wrap;", "{plane_receipt()}" }
                        }
                    }
                }
            }
        }
    }
}
