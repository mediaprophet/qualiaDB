//! Person / apparatus fleet management and Four-Plane Identifier Fabric in Settings.

#![allow(non_snake_case)]

use super::{FIELD, PANEL, PRIMARY_BUTTON, SECONDARY_BUTTON, SUCCESS_CARD, WARNING_CARD};
use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::settings::host::invoke_json;
use aether::prelude::*;
use serde::{Deserialize, Serialize};

/// Canonical jury-safe plane enumeration in the Four-Plane Identifier Fabric (`F1` §14.4 / `F5` §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierFabricPlane {
    NaturalAgent,
    ClaimOpinion,
    SpatiotemporalHandle,
    Instrument,
}

pub const PLANE_NATURAL_AGENT_TITLE: &str = "People & Living Beings (NaturalAgent)";
pub const PLANE_CLAIM_OPINION_TITLE: &str = "Claims, Opinions & Co-Attestations (ClaimOpinion)";
pub const PLANE_SPATIOTEMPORAL_HANDLE_TITLE: &str = "Where & When Now (SpatiotemporalHandle)";
pub const PLANE_INSTRUMENT_TITLE: &str = "Cryptographic Tools & Instruments (Instrument)";

pub const VC_ORIGIN_INTEGRITY_DISCLAIMER: &str =
    "Proves issuer origin & integrity, not claim truth";
pub const CO_ATTESTATION_BUNDLE_FORMULA: &str =
    "these signatures, this window, these scoped machines/agents";
pub const DID_Q42_QRC_LABEL: &str = "Storage / Topological Coordinate (QRC)";
pub const NATURAL_AGENT_FORBIDDEN_LABELS: [&str; 3] = ["Thing", "Object", "DID"];

impl IdentifierFabricPlane {
    pub const ALL: [Self; 4] = [
        Self::NaturalAgent,
        Self::ClaimOpinion,
        Self::SpatiotemporalHandle,
        Self::Instrument,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::NaturalAgent => PLANE_NATURAL_AGENT_TITLE,
            Self::ClaimOpinion => PLANE_CLAIM_OPINION_TITLE,
            Self::SpatiotemporalHandle => PLANE_SPATIOTEMPORAL_HANDLE_TITLE,
            Self::Instrument => PLANE_INSTRUMENT_TITLE,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::NaturalAgent => "NaturalAgent",
            Self::ClaimOpinion => "ClaimOpinion",
            Self::SpatiotemporalHandle => "SpatiotemporalHandle",
            Self::Instrument => "Instrument",
        }
    }

    pub const fn border_color(self) -> &'static str {
        match self {
            Self::NaturalAgent => "rgba(52,211,153,.45)",
            Self::ClaimOpinion => "rgba(96,165,250,.45)",
            Self::SpatiotemporalHandle => "rgba(34,211,238,.45)",
            Self::Instrument => "rgba(251,191,36,.45)",
        }
    }
}

/// Validates that a `NaturalAgent` UI label never uses forbidden objectifying or instrument terms.
pub fn is_valid_natural_agent_label(label: &str) -> bool {
    !NATURAL_AGENT_FORBIDDEN_LABELS.iter().any(|forbidden| {
        label
            .split(|c: char| !c.is_alphanumeric() && c != ':')
            .any(|token| token.eq_ignore_ascii_case(forbidden))
    }) && !label.to_ascii_lowercase().contains("owl:thing")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PersonPublicView {
    person_id: String,
    verifying_key_hex: String,
    #[serde(default)]
    created_at_unix: u64,
    #[serde(default)]
    display_hint: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct DeviceView {
    device_id: String,
    person_id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    hostname: String,
    #[serde(default)]
    control_base_url: String,
    is_local: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct IdentityPlaneView {
    person: PersonPublicView,
    local_device_id: String,
    #[serde(default)]
    devices: Vec<DeviceView>,
    #[serde(default)]
    os_account_is_not_principal: bool,
    #[serde(default)]
    notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersonTransferBundleView {
    format: String,
    person: serde_json::Value,
}

#[component]
/// Aether `IdentityPlanePanel` component.
pub fn IdentityPlanePanel() -> Element {
    let mut plane = use_signal(|| Option::<IdentityPlaneView>::None);
    let mut status = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut control_url = use_signal(String::new);
    let mut import_json = use_signal(String::new);
    let mut peer_json = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let mut refresh = move || {
        busy.set(true);
        error.set(String::new());
        spawn(async move {
            match invoke_json::<IdentityPlaneView>("get_identity_plane", serde_json::json!({}))
                .await
            {
                Ok(value) => {
                    if let Some(local) = value.devices.iter().find(|d| d.is_local) {
                        control_url.set(local.control_base_url.clone());
                    }
                    plane.set(Some(value));
                }
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };

    use_hook(move || refresh());

    rsx! {
        section { style: "display:grid;gap:16px;",
            div { style: "{PANEL}",
                div { style: "display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:8px;",
                    h2 { style: "margin:0;font-size:1.05rem;", "Four-Plane Identifier Fabric (Jury-Safe Enumeration)" }
                    HonestyChip { level: HonestyLevel::Ready, detail: "NaturalAgent · ClaimOpinion · SpatiotemporalHandle · Instrument".to_string() }
                }
                p { style: "margin:0;color:var(--qualia-text-muted);font-size:.76rem;line-height:1.55;",
                    "Identifiers are never human identity. Living beings, claims, spacetime/route coordinates, and cryptographic instruments stay strictly unmerged across all four planes."
                }
            }

            if !error().is_empty() {
                div { style: "{WARNING_CARD}", "{error}" }
            }
            if !status().is_empty() {
                div { style: "{SUCCESS_CARD}", "{status}" }
            }

            // Plane 1: NaturalAgent
            div {
                "data-plane": "{IdentifierFabricPlane::NaturalAgent.code()}",
                style: format!("{PANEL} border-left:4px solid {};", IdentifierFabricPlane::NaturalAgent.border_color()),
                div { style: "display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:8px;",
                    h3 { style: "margin:0;font-size:.92rem;", "{PLANE_NATURAL_AGENT_TITLE}" }
                    HonestyChip { level: HonestyLevel::Ready, detail: "living-SHACL".to_string() }
                }
                p { style: "margin:0 0 10px;color:var(--qualia-text-muted);font-size:.73rem;line-height:1.5;",
                    "Framed strictly as living-SHACL records (D_living). Never labeled or classified as Thing, Object, owl:Thing, or reduced to a DID."
                }
                if let Some(p) = plane() {
                    dl { style: "margin:0;display:grid;grid-template-columns:auto 1fr;gap:8px 12px;font-size:.74rem;",
                        dt { style: "color:var(--qualia-text-muted);", "NaturalAgent record" }
                        dd { style: "margin:0;overflow-wrap:anywhere;font-family:ui-monospace,monospace;font-size:.68rem;", "{p.person.person_id}" }
                        dt { style: "color:var(--qualia-text-muted);", "Ontological framing" }
                        dd { style: "margin:0;", "living-SHACL (disjoint from owl:Thing)" }
                        dt { style: "color:var(--qualia-text-muted);", "OS account" }
                        dd { style: "margin:0;", "Not used as identity" }
                    }
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;margin-top:14px;",
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    match invoke_json::<PersonTransferBundleView>(
                                        "export_person_transfer_bundle",
                                        serde_json::json!({}),
                                    ).await {
                                        Ok(bundle) => {
                                            let text = serde_json::to_string_pretty(&bundle).unwrap_or_default();
                                            import_json.set(text);
                                            status.set("Transfer bundle loaded into the box below — save it privately, then import on another machine.".into());
                                        }
                                        Err(e) => error.set(e),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Export person transfer bundle"
                        }
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>(
                                        "mint_person_webid_tls_cert",
                                        serde_json::json!({}),
                                    ).await {
                                        Ok(v) => {
                                            status.set(format!(
                                                "WebID-TLS cert written: {}",
                                                v.get("cert_path").and_then(|x| x.as_str()).unwrap_or("ok")
                                            ));
                                        }
                                        Err(e) => error.set(e),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Mint WebID-TLS cert for person"
                        }
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| refresh(),
                            "Refresh"
                        }
                    }
                }
            }

            // Plane 2: ClaimOpinion
            div {
                "data-plane": "{IdentifierFabricPlane::ClaimOpinion.code()}",
                style: format!("{PANEL} border-left:4px solid {};", IdentifierFabricPlane::ClaimOpinion.border_color()),
                div { style: "display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:8px;",
                    h3 { style: "margin:0;font-size:.92rem;", "{PLANE_CLAIM_OPINION_TITLE}" }
                    HonestyChip { level: HonestyLevel::Ready, detail: "CoAttestationBundle".to_string() }
                }
                dl { style: "margin:0;display:grid;grid-template-columns:auto 1fr;gap:8px 12px;font-size:.74rem;",
                    dt { style: "color:var(--qualia-text-muted);", "Time window" }
                    dd { style: "margin:0;", "Bounded validity interval (not_before .. expires_at); never timeless truth" }
                    dt { style: "color:var(--qualia-text-muted);", "CoAttestationBundle" }
                    dd { style: "margin:0;font-weight:700;", "\"{CO_ATTESTATION_BUNDLE_FORMULA}\"" }
                    dt { style: "color:var(--qualia-text-muted);", "VC origin & integrity" }
                    dd { style: "margin:0;",
                        span {
                            "data-vc-disclaimer": "true",
                            style: "display:inline-block;padding:2px 8px;border-radius:999px;border:1px solid rgba(251,191,36,.45);background:rgba(120,53,15,.28);color:#fde68a;font-size:.68rem;font-weight:700;",
                            "{VC_ORIGIN_INTEGRITY_DISCLAIMER}"
                        }
                    }
                }
            }

            // Plane 3: SpatiotemporalHandle
            div {
                "data-plane": "{IdentifierFabricPlane::SpatiotemporalHandle.code()}",
                style: format!("{PANEL} border-left:4px solid {};", IdentifierFabricPlane::SpatiotemporalHandle.border_color()),
                div { style: "display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:8px;",
                    h3 { style: "margin:0;font-size:.92rem;", "{PLANE_SPATIOTEMPORAL_HANDLE_TITLE}" }
                    HonestyChip { level: HonestyLevel::Ready, detail: "DNI · USRI · QRC".to_string() }
                }
                dl { style: "margin:0;display:grid;grid-template-columns:auto 1fr;gap:8px 12px;font-size:.74rem;",
                    dt { style: "color:var(--qualia-text-muted);", "DNI route epoch" }
                    dd { style: "margin:0;", "Ephemeral Dynamic Network Identifier (RAR max 8 DNIs, distinct from QSessionProof)" }
                    dt { style: "color:var(--qualia-text-muted);", "USRI / CB-USRI" }
                    dd { style: "margin:0;font-family:ui-monospace,monospace;font-size:.68rem;", "urn:omni:* + 16-byte stack CB-USRI cosmic/geospatial coordinate (L_-2 .. L_12)" }
                    dt { style: "color:var(--qualia-text-muted);", "did:q42" }
                    dd { style: "margin:0;",
                        span {
                            "data-qrc-label": "true",
                            style: "display:inline-block;padding:2px 8px;border-radius:999px;border:1px solid rgba(34,211,238,.45);background:rgba(8,51,68,.4);color:#67e8f9;font-size:.68rem;font-weight:700;",
                            "{DID_Q42_QRC_LABEL}"
                        }
                        span { style: "margin-left:8px;color:var(--qualia-text-muted);font-size:.7rem;", "(60-bit FNV-1a + MSB 1<<63 pointer; never human identity)" }
                    }
                }
            }

            // Plane 4: Instrument
            div {
                "data-plane": "{IdentifierFabricPlane::Instrument.code()}",
                style: format!("{PANEL} border-left:4px solid {};", IdentifierFabricPlane::Instrument.border_color()),
                div { style: "display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:8px;",
                    h3 { style: "margin:0;font-size:.92rem;", "{PLANE_INSTRUMENT_TITLE}" }
                    div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                        HonestyChip { level: HonestyLevel::Ready, detail: "Scoped Instruments".to_string() }
                        HonestyChip { level: HonestyLevel::Planned, detail: "Interactive hardware key-rotation wizard".to_string() }
                    }
                }
                dl { style: "margin:0;display:grid;grid-template-columns:auto 1fr;gap:8px 12px;font-size:.74rem;",
                    dt { style: "color:var(--qualia-text-muted);", "Cryptographic DIDs" }
                    dd { style: "margin:0;font-family:ui-monospace,monospace;font-size:.68rem;", "did:qi (genesis SHA-256 + qi.generation) · did:webizen" }
                    dt { style: "color:var(--qualia-text-muted);", "Relation-scoped locators" }
                    dd { style: "margin:0;", "Directional pairwise locators (A -> B != B -> A; no-proxy-launder enforced)" }
                    dt { style: "color:var(--qualia-text-muted);", "Wallets & online accounts" }
                    dd { style: "margin:0;", "WalletShape & OnlineAccountShape instruments (never person-merge keys)" }
                    dt { style: "color:var(--qualia-text-muted);", "Hardware & sensor IDs" }
                    dd { style: "margin:0;", "MachineDeviceId & SensorId (Etalon ObservationEnvelope sources)" }
                    dt { style: "color:var(--qualia-text-muted);", "Biometric separation" }
                    dd { style: "margin:0;", "BiometricFamily (modality/template schema) vs. mutable BiometricInstance (consent- and time-scoped sample)" }
                    if let Some(p) = plane() {
                        dt { style: "color:var(--qualia-text-muted);", "Active verifying key" }
                        dd { style: "margin:0;overflow-wrap:anywhere;font-family:ui-monospace,monospace;font-size:.68rem;", "{p.person.verifying_key_hex}" }
                    }
                }
            }

            if let Some(p) = plane() {
                div { style: "{PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.92rem;", "This apparatus (local device)" }
                    p { style: "margin:0 0 10px;font-size:.72rem;color:var(--qualia-text-muted);line-height:1.5;",
                        "Device ID: "
                        span { style: "font-family:ui-monospace,monospace;font-size:.68rem;", "{p.local_device_id}" }
                    }
                    label { style: "display:block;font-size:.72rem;font-weight:750;margin-bottom:6px;",
                        "Control base URL (LAN address other machines use to send jobs here)"
                    }
                    input {
                        value: "{control_url}",
                        placeholder: "http://192.168.1.20:8080",
                        style: "{FIELD}",
                        oninput: move |e| control_url.set(e.value()),
                    }
                    button {
                        r#type: "button",
                        style: format!("{PRIMARY_BUTTON} margin-top:12px;"),
                        disabled: busy(),
                        onclick: move |_| {
                            let url = control_url();
                            busy.set(true);
                            spawn(async move {
                                match invoke_json::<IdentityPlaneView>(
                                    "set_local_control_base_url",
                                    serde_json::json!({ "url": url }),
                                ).await {
                                    Ok(v) => {
                                        plane.set(Some(v));
                                        status.set("Control URL saved.".into());
                                    }
                                    Err(e) => error.set(e),
                                }
                                busy.set(false);
                            });
                        },
                        "Save control URL"
                    }
                }

                div { style: "{PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.92rem;", "Fleet devices" }
                    if p.devices.is_empty() {
                        p { style: "margin:0;color:var(--qualia-text-muted);font-size:.74rem;", "No devices yet." }
                    } else {
                        div { style: "display:grid;gap:8px;",
                            for d in p.devices {
                                div { style: "padding:12px;border:1px solid var(--qualia-border);border-radius:11px;",
                                    div { style: "display:flex;justify-content:space-between;gap:8px;flex-wrap:wrap;",
                                        strong { style: "font-size:.78rem;",
                                            if d.label.is_empty() { "Apparatus" } else { "{d.label}" }
                                        }
                                        span { style: "font-size:.64rem;font-weight:800;text-transform:uppercase;",
                                            if d.is_local { "Local" } else { "Peer" }
                                        }
                                    }
                                    div { style: "margin-top:6px;font-family:ui-monospace,monospace;font-size:.66rem;overflow-wrap:anywhere;", "{d.device_id}" }
                                    if !d.control_base_url.is_empty() {
                                        div { style: "margin-top:4px;font-size:.7rem;color:var(--qualia-text-muted);", "URL: {d.control_base_url}" }
                                    }
                                    if !d.hostname.is_empty() {
                                        div { style: "margin-top:2px;font-size:.7rem;color:var(--qualia-text-muted);", "Host: {d.hostname} (informational only)" }
                                    }
                                }
                            }
                        }
                    }
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;margin-top:12px;",
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    match invoke_json::<usize>("retry_remote_job_outbox", serde_json::json!({})).await {
                                        Ok(n) => status.set(format!("Retried remote outbox; delivered {}.", n)),
                                        Err(e) => error.set(e),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Retry remote job outbox"
                        }
                    }
                }
            }

            div { style: "{PANEL}",
                h3 { style: "margin:0 0 8px;font-size:.92rem;", "Import person transfer bundle" }
                textarea {
                    value: "{import_json}",
                    rows: "5",
                    placeholder: "Paste person transfer bundle JSON (format qualia.person.transfer.v1)",
                    style: format!("{FIELD} min-height:100px;font-family:ui-monospace,monospace;font-size:.68rem;"),
                    oninput: move |e| import_json.set(e.value()),
                }
                button {
                    r#type: "button",
                    style: format!("{PRIMARY_BUTTON} margin-top:10px;"),
                    disabled: busy() || import_json().trim().is_empty(),
                    onclick: move |_| {
                        let raw = import_json();
                        busy.set(true);
                        spawn(async move {
                            match serde_json::from_str::<serde_json::Value>(&raw) {
                                Ok(bundle) => {
                                    let payload = serde_json::json!({ "bundle": bundle });
                                    match invoke_json::<IdentityPlaneView>("import_person_transfer_bundle", payload).await {
                                        Ok(v) => {
                                            plane.set(Some(v));
                                            status.set("Person principal imported; local apparatus re-bound.".into());
                                        }
                                        Err(e) => error.set(e),
                                    }
                                }
                                Err(e) => error.set(format!("Invalid JSON: {}", e)),
                            }
                            busy.set(false);
                        });
                    },
                    "Import person bundle"
                }
            }

            div { style: "{PANEL}",
                h3 { style: "margin:0 0 8px;font-size:.92rem;", "Register peer apparatus" }
                textarea {
                    value: "{peer_json}",
                    rows: "4",
                    placeholder: "Peer device JSON: device_id, person_id, control_base_url",
                    style: format!("{FIELD} min-height:88px;font-family:ui-monospace,monospace;font-size:.68rem;"),
                    oninput: move |e| peer_json.set(e.value()),
                }
                button {
                    r#type: "button",
                    style: format!("{PRIMARY_BUTTON} margin-top:10px;"),
                    disabled: busy() || peer_json().trim().is_empty(),
                    onclick: move |_| {
                        let raw = peer_json();
                        busy.set(true);
                        spawn(async move {
                            match serde_json::from_str::<serde_json::Value>(&raw) {
                                Ok(device) => {
                                    let payload = serde_json::json!({ "device": device });
                                    match invoke_json::<IdentityPlaneView>("register_remote_apparatus_device", payload).await {
                                        Ok(v) => {
                                            plane.set(Some(v));
                                            status.set("Peer apparatus registered.".into());
                                        }
                                        Err(e) => error.set(e),
                                    }
                                }
                                Err(e) => error.set(format!("Invalid JSON: {}", e)),
                            }
                            busy.set(false);
                        });
                    },
                    "Register peer device"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_plane_labels_match_f5_locked_voice() {
        assert_eq!(
            IdentifierFabricPlane::NaturalAgent.title(),
            "People & Living Beings (NaturalAgent)"
        );
        assert_eq!(
            IdentifierFabricPlane::ClaimOpinion.title(),
            "Claims, Opinions & Co-Attestations (ClaimOpinion)"
        );
        assert_eq!(
            IdentifierFabricPlane::SpatiotemporalHandle.title(),
            "Where & When Now (SpatiotemporalHandle)"
        );
        assert_eq!(
            IdentifierFabricPlane::Instrument.title(),
            "Cryptographic Tools & Instruments (Instrument)"
        );
        assert!(is_valid_natural_agent_label(
            IdentifierFabricPlane::NaturalAgent.title()
        ));
        assert!(!is_valid_natural_agent_label("Thing"));
        assert!(!is_valid_natural_agent_label("Object"));
        assert!(!is_valid_natural_agent_label("Person DID"));
        assert!(!is_valid_natural_agent_label("owl:Thing subclass"));
    }

    #[test]
    fn vc_disclaimer_and_did_q42_qrc_labels_are_exact() {
        assert_eq!(
            VC_ORIGIN_INTEGRITY_DISCLAIMER,
            "Proves issuer origin & integrity, not claim truth"
        );
        assert_eq!(
            CO_ATTESTATION_BUNDLE_FORMULA,
            "these signatures, this window, these scoped machines/agents"
        );
        assert_eq!(DID_Q42_QRC_LABEL, "Storage / Topological Coordinate (QRC)");
        assert!(!DID_Q42_QRC_LABEL.to_ascii_lowercase().contains("identity"));
    }
}
