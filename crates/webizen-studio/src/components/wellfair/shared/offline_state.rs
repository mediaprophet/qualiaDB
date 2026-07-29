use super::super::host_dto::{NetworkExposure, WellfairHostSnapshot};
use dioxus::prelude::*;

#[component]
pub fn OfflineState(snapshot: WellfairHostSnapshot) -> Element {
    let (label, detail, tone) = match snapshot.network {
        NetworkExposure::Offline => (
            "Offline",
            "No network adapters active. Local vault and receipts remain available.",
            "#6c757d",
        ),
        NetworkExposure::LocalOnly => (
            "Local only",
            "Loopback services only. No external endpoints are enabled.",
            "#457b9d",
        ),
        NetworkExposure::ExternalCapable => (
            "External capable",
            "Some adapters may reach external endpoints when explicitly approved.",
            "#e9c46a",
        ),
    };

    rsx! {
        div {
            role: "status",
            aria_live: "polite",
            style: "display:flex;flex-direction:column;gap:0.2rem;padding:0.5rem 0.75rem;border-radius:8px;border:1px solid {tone}44;background:{tone}11;",
            strong { style: "color:{tone};font-size:0.85rem;", "{label}" }
            span { style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);", "{detail}" }
        }
    }
}
