use super::super::host_dto::SyncQueueState;
use dioxus::prelude::*;

#[component]
pub fn SyncState(state: SyncQueueState, pending_jobs: u32) -> Element {
    let (label, tone) = match state {
        SyncQueueState::Idle => ("Sync idle", "#6c757d"),
        SyncQueueState::Queued => ("Queued", "#457b9d"),
        SyncQueueState::Sending => ("Sending", "#2a9d8f"),
        SyncQueueState::Acknowledged => ("Acknowledged", "#2a9d8f"),
        SyncQueueState::Conflicted => ("Conflicted", "#e76f51"),
        SyncQueueState::Rejected => ("Rejected", "#e76f51"),
        SyncQueueState::Revoked => ("Revoked", "#9d0208"),
    };

    rsx! {
        div {
            role: "status",
            style: "display:flex;align-items:center;justify-content:space-between;gap:0.75rem;padding:0.45rem 0.65rem;border-radius:8px;border:1px solid {tone}44;",
            span { style: "font-size:0.82rem;font-weight:600;color:{tone};", "{label}" }
            span { style: "font-size:0.75rem;color:var(--qualia-text-muted,#666);", "Jobs pending: {pending_jobs}" }
        }
    }
}