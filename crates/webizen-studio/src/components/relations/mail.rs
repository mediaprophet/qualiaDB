//! Daily Talk → Mail inbox — purpose mailboxes + landed mail, not Domains admin.

use super::mail_model::{
    extract_addresses, extract_messages, filter_messages, format_received_at, inbox_counts_line,
    mailbox_chips, message_mailbox, receiver_from_status, reply_draft, smtp_ready, text,
    unread_in_mailbox, MailboxKind, ReceiverState,
};
use crate::components::settings::host::invoke_json;
use crate::components::settings::{
    EMPTY_CARD, FIELD, PANEL, PRIMARY_BUTTON, SECONDARY_BUTTON, SELECTED_ROW, SUCCESS_CARD,
    WARNING_CARD,
};
use dioxus::prelude::*;

const LIST_ROW: &str = "width:100%;display:flex;flex-direction:column;align-items:flex-start;gap:3px;padding:10px 12px;border:1px solid var(--qualia-border);border-radius:10px;background:transparent;color:var(--qualia-text);font:inherit;text-align:left;cursor:pointer;";
const LIST_ROW_ON: &str = "width:100%;display:flex;flex-direction:column;align-items:flex-start;gap:3px;padding:10px 12px;border:1px solid var(--qualia-accent);border-radius:10px;background:var(--qualia-accent-glow);color:var(--qualia-text);font:inherit;text-align:left;cursor:pointer;";

#[component]
pub fn MailInboxPane() -> Element {
    let mut addresses = use_signal(Vec::<serde_json::Value>::new);
    let mut messages = use_signal(Vec::<serde_json::Value>::new);
    let mut counts_line = use_signal(String::new);
    let mut selected_mailbox = use_signal(Option::<String>::default);
    let mut selected_id = use_signal(String::new);
    let mut selected = use_signal(|| Option::<serde_json::Value>::None);
    let mut receiver = use_signal(|| ReceiverState::Held);
    let mut transport = use_signal(|| serde_json::json!({ "smtp": null }));
    let mut status = use_signal(String::new);
    let mut loading = use_signal(|| true);
    let mut starting = use_signal(|| false);
    let mut sending = use_signal(|| false);
    let mut composing = use_signal(|| false);
    let mut include_quarantine = use_signal(|| true);
    let mut compose_from = use_signal(String::new);
    let mut compose_to = use_signal(String::new);
    let mut compose_subject = use_signal(String::new);
    let mut compose_body = use_signal(String::new);

    let mut refresh = move || {
        loading.set(true);
        spawn(async move {
            let addr_v =
                invoke_json::<serde_json::Value>("list_mail_addresses", serde_json::json!({}))
                    .await;
            let list_v = invoke_json::<serde_json::Value>(
                "mail_list",
                serde_json::json!({ "includeQuarantine": true }),
            )
            .await;
            let recv_v =
                invoke_json::<serde_json::Value>("mail_receiver_status", serde_json::json!({}))
                    .await;
            let smtp_v = invoke_json::<serde_json::Value>(
                "load_mail_transport_config",
                serde_json::json!({}),
            )
            .await;

            match addr_v {
                Ok(value) => addresses.set(extract_addresses(&value)),
                Err(error) => status.set(error),
            }
            match list_v {
                Ok(value) => {
                    messages.set(extract_messages(&value));
                    counts_line.set(inbox_counts_line(&value));
                }
                Err(error) => {
                    if status().is_empty() {
                        status.set(error);
                    }
                }
            }
            if let Ok(value) = recv_v {
                receiver.set(receiver_from_status(&value));
            }
            if let Ok(value) = smtp_v {
                transport.set(value);
            }
            loading.set(false);
        });
    };
    use_hook(move || refresh());

    let start_receiver = move |_| {
        starting.set(true);
        status.set(String::new());
        spawn(async move {
            match invoke_json::<serde_json::Value>(
                "mail_receiver_start",
                serde_json::json!({ "bind": "127.0.0.1:2525" }),
            )
            .await
            {
                Ok(value) => {
                    receiver.set(receiver_from_status(&serde_json::json!({
                        "running": true,
                        "bind": value.get("bind").and_then(|v| v.as_str()).unwrap_or("127.0.0.1:2525")
                    })));
                    status.set("Receiver started. New mail can land here.".into());
                }
                Err(error) => status.set(error),
            }
            starting.set(false);
        });
    };

    let mut open_message = move |id: String| {
        selected_id.set(id.clone());
        spawn(async move {
            match invoke_json::<serde_json::Value>(
                "mail_get",
                serde_json::json!({ "id": id.clone() }),
            )
            .await
            {
                Ok(value) => {
                    selected.set(Some(value));
                    let _ = invoke_json::<serde_json::Value>(
                        "mail_set_read",
                        serde_json::json!({ "id": id, "read": true }),
                    )
                    .await;
                    if let Ok(list) = invoke_json::<serde_json::Value>(
                        "mail_list",
                        serde_json::json!({ "includeQuarantine": true }),
                    )
                    .await
                    {
                        messages.set(extract_messages(&list));
                        counts_line.set(inbox_counts_line(&list));
                    }
                }
                Err(error) => status.set(error),
            }
        });
    };

    let begin_reply = move |_| {
        if let Some(message) = selected() {
            let draft = reply_draft(&message);
            compose_from.set(draft.from);
            compose_to.set(draft.to);
            compose_subject.set(draft.subject);
            compose_body.set(draft.body);
            composing.set(true);
        }
    };

    let begin_compose = move |_| {
        let chips = mailbox_chips(&addresses());
        let from = selected_mailbox().unwrap_or_else(|| {
            chips
                .iter()
                .find(|chip| chip.kind == MailboxKind::Purpose && chip.enabled)
                .map(|chip| chip.address.clone())
                .unwrap_or_default()
        });
        compose_from.set(from);
        compose_to.set(String::new());
        compose_subject.set(String::new());
        compose_body.set(String::new());
        composing.set(true);
    };

    let send_mail = move |_| {
        if !smtp_ready(&transport()) {
            status.set(
                "Held / not yet — outbound SMTP is not configured. You can still read mail that has landed. Reception holds domain and transport setup.".into(),
            );
            return;
        }
        sending.set(true);
        spawn(async move {
            let mail = serde_json::json!({
                "from": compose_from().trim(),
                "to": compose_to().trim(),
                "subject": compose_subject(),
                "body": compose_body(),
            });
            let smtp = transport()
                .get("smtp")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            match invoke_json::<serde_json::Value>(
                "mail_send",
                serde_json::json!({
                    "smtpJson": smtp.to_string(),
                    "mailJson": mail.to_string(),
                }),
            )
            .await
            {
                Ok(_) => {
                    composing.set(false);
                    status.set(format!("Sent to {}.", compose_to().trim()));
                }
                Err(error) => status.set(error),
            }
            sending.set(false);
        });
    };

    let delete_selected = move |_| {
        let id = selected_id();
        if id.is_empty() {
            return;
        }
        spawn(async move {
            match invoke_json::<serde_json::Value>("mail_delete", serde_json::json!({ "id": id }))
                .await
            {
                Ok(_) => {
                    selected_id.set(String::new());
                    selected.set(None);
                    composing.set(false);
                    refresh();
                }
                Err(error) => status.set(error),
            }
        });
    };

    let chips = mailbox_chips(&addresses());
    let visible = {
        let mailbox = selected_mailbox();
        filter_messages(&messages(), mailbox.as_deref(), include_quarantine())
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    let held = matches!(receiver(), ReceiverState::Held);

    rsx! {
        section {
            "data-mail-daily": "true",
            style: "height:100%;min-height:0;display:flex;flex-direction:column;overflow:hidden;",
            header { style: "padding:16px 18px 12px;border-bottom:1px solid #243044;flex-shrink:0;",
                div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:12px;flex-wrap:wrap;",
                    div {
                        h2 { style: "margin:0;font-size:1.18rem;", "Mail" }
                        p { style: "margin:5px 0 0;color:var(--qualia-text-muted);font-size:.76rem;line-height:1.5;max-width:46rem;",
                            "Purpose inboxes and mail that has landed on this machine. This is not Gmail — read, reply, start the receiver. Domain DNS stays under Reception."
                        }
                    }
                    div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                        button { style: "{SECONDARY_BUTTON}", onclick: move |_| refresh(),
                            if loading() { "Refreshing…" } else { "Refresh" }
                        }
                        button { style: "{PRIMARY_BUTTON}", onclick: begin_compose, "New message" }
                    }
                }
                p { role: "status", style: "margin:8px 0 0;font-size:.7rem;color:#94a3b8;",
                    if counts_line().is_empty() { "No landed mail yet." } else { "{counts_line}" }
                }
            }

            if held {
                div { style: "{WARNING_CARD} margin:12px 18px 0;flex-shrink:0;",
                    strong { "Held / not yet" }
                    p { style: "margin:6px 0 10px;font-size:.76rem;line-height:1.45;",
                        "The mail receiver is not running. Mail already here stays readable. New messages will not land until you start it."
                    }
                    button {
                        style: "{PRIMARY_BUTTON}",
                        disabled: starting(),
                        onclick: start_receiver,
                        if starting() { "Starting receiver…" } else { "Start receiver" }
                    }
                }
            } else if let ReceiverState::Running { bind } = receiver() {
                div { style: "{SUCCESS_CARD} margin:12px 18px 0;flex-shrink:0;font-size:.74rem;",
                    "Receiver running on {bind}. Purpose inboxes below collect what lands."
                }
            }

            if !status().is_empty() {
                p { role: "status", style: "margin:10px 18px 0;font-size:.72rem;color:#fde68a;", "{status}" }
            }

            div { style: "flex:1;min-height:0;display:grid;grid-template-columns:200px minmax(220px,1fr) minmax(260px,1.2fr);gap:0;border-top:1px solid #243044;margin-top:12px;",
                aside { style: "min-height:0;overflow-y:auto;padding:12px;border-right:1px solid #243044;display:flex;flex-direction:column;gap:6px;",
                    div { style: "font-size:.62rem;font-weight:800;letter-spacing:.08em;text-transform:uppercase;color:#a78bfa;margin-bottom:4px;",
                        "Purpose inboxes"
                    }
                    button {
                        style: if selected_mailbox().is_none() { SELECTED_ROW } else { crate::components::settings::ROW },
                        onclick: move |_| selected_mailbox.set(None),
                        "All landed"
                        span { style: "margin-left:auto;color:#94a3b8;font-size:.65rem;",
                            "{unread_in_mailbox(&messages(), None)}"
                        }
                    }
                    if chips.is_empty() {
                        div { style: "{EMPTY_CARD}",
                            "No purpose inboxes yet. Reception is where a domain is onboarded. This list fills after that — you do not need Domains admin to read mail that has already landed."
                        }
                    } else {
                        for chip in chips {
                            {
                                let address = chip.address.clone();
                                let address_sel = address.clone();
                                let unread = unread_in_mailbox(&messages(), Some(&address));
                                let on = selected_mailbox().as_deref() == Some(address.as_str());
                                let kind_label = match chip.kind {
                                    MailboxKind::Purpose => "purpose",
                                    MailboxKind::Relationship => "relationship",
                                    MailboxKind::Other => "mailbox",
                                };
                                rsx! {
                                    button {
                                        style: if on { SELECTED_ROW } else { crate::components::settings::ROW },
                                        onclick: move |_| selected_mailbox.set(Some(address_sel.clone())),
                                        div { style: "display:flex;flex-direction:column;align-items:flex-start;min-width:0;",
                                            span { "{chip.label}" }
                                            span { style: "font-size:.62rem;color:#94a3b8;",
                                                if chip.enabled { "{kind_label}" } else { "{kind_label} · paused" }
                                            }
                                        }
                                        span { style: "margin-left:auto;color:#94a3b8;font-size:.65rem;", "{unread}" }
                                    }
                                }
                            }
                        }
                    }
                    button {
                        style: "{SECONDARY_BUTTON} margin-top:8px;",
                        onclick: move |_| include_quarantine.set(!include_quarantine()),
                        if include_quarantine() { "Hide quarantine" } else { "Show quarantine" }
                    }
                }

                div { style: "min-height:0;overflow-y:auto;padding:12px;border-right:1px solid #243044;display:flex;flex-direction:column;gap:6px;",
                    if visible.is_empty() {
                        div { style: "{EMPTY_CARD}",
                            if held {
                                "Nothing to read yet. Start the receiver, then mail that lands will list here."
                            } else {
                                "Inbox is empty. Mail to a purpose address lands here — no provider host required."
                            }
                        }
                    } else {
                        for message in visible {
                            {
                                let id = text(&message, "id");
                                let id_click = id.clone();
                                let subject = text(&message, "subject");
                                let from = text(&message, "from_address");
                                let mailbox = message_mailbox(&message);
                                let read = message.get("read").and_then(|v| v.as_bool()).unwrap_or(false);
                                let quarantined = message.get("quarantined").and_then(|v| v.as_bool()).unwrap_or(false);
                                let when = format_received_at(
                                    message.get("received_at").and_then(|v| v.as_u64()).unwrap_or(0),
                                );
                                let on = selected_id() == id;
                                rsx! {
                                    button {
                                        style: if on { LIST_ROW_ON } else { LIST_ROW },
                                        onclick: move |_| open_message(id_click.clone()),
                                        span { style: if read { "font-weight:500;font-size:.8rem;" } else { "font-weight:800;font-size:.8rem;" },
                                            if subject.is_empty() { "(no subject)" } else { "{subject}" }
                                        }
                                        span { style: "font-size:.68rem;color:#94a3b8;", "{from} → {mailbox}" }
                                        span { style: "font-size:.62rem;color:#64748b;",
                                            if quarantined { "{when} · quarantine" } else { "{when}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "min-height:0;overflow-y:auto;padding:14px 16px 2rem;display:flex;flex-direction:column;gap:10px;",
                    if composing() {
                        div { style: "{PANEL}",
                            h3 { style: "margin:0 0 8px;font-size:.95rem;",
                                if compose_subject().to_ascii_lowercase().starts_with("re:") { "Reply" } else { "New message" }
                            }
                            if !smtp_ready(&transport()) {
                                p { style: "margin:0 0 8px;font-size:.72rem;color:#fde68a;line-height:1.45;",
                                    "Held / not yet — sending needs outbound SMTP. Draft here; Reception holds transport setup. Reading does not wait on that."
                                }
                            }
                            label { style: "font-size:.68rem;color:#94a3b8;", "From" }
                            input { style: "{FIELD} margin-bottom:8px;", value: "{compose_from}", oninput: move |e| compose_from.set(e.value()) }
                            label { style: "font-size:.68rem;color:#94a3b8;", "To" }
                            input { style: "{FIELD} margin-bottom:8px;", value: "{compose_to}", oninput: move |e| compose_to.set(e.value()) }
                            label { style: "font-size:.68rem;color:#94a3b8;", "Subject" }
                            input { style: "{FIELD} margin-bottom:8px;", value: "{compose_subject}", oninput: move |e| compose_subject.set(e.value()) }
                            label { style: "font-size:.68rem;color:#94a3b8;", "Message" }
                            textarea {
                                style: "{FIELD} min-height:160px;resize:vertical;",
                                value: "{compose_body}",
                                oninput: move |e| compose_body.set(e.value())
                            }
                            div { style: "display:flex;gap:8px;margin-top:10px;flex-wrap:wrap;",
                                button {
                                    style: "{PRIMARY_BUTTON}",
                                    disabled: sending(),
                                    onclick: send_mail,
                                    if sending() { "Sending…" } else { "Send" }
                                }
                                button { style: "{SECONDARY_BUTTON}", onclick: move |_| composing.set(false), "Cancel" }
                            }
                        }
                    } else if let Some(message) = selected() {
                        {
                            let subject = text(&message, "subject");
                            let from = text(&message, "from_address");
                            let to = text(&message, "to_address");
                            let body = text(&message, "body");
                            let when = format_received_at(
                                message.get("received_at").and_then(|v| v.as_u64()).unwrap_or(0),
                            );
                            let quarantined = message.get("quarantined").and_then(|v| v.as_bool()).unwrap_or(false);
                            rsx! {
                                div {
                                    h3 { style: "margin:0 0 6px;font-size:1.05rem;",
                                        if subject.is_empty() { "(no subject)" } else { "{subject}" }
                                    }
                                    p { style: "margin:0;font-size:.74rem;color:#94a3b8;line-height:1.45;",
                                        "From {from}"
                                    }
                                    p { style: "margin:2px 0 0;font-size:.72rem;color:#64748b;",
                                        "To {to} · {when}"
                                        if quarantined { " · quarantine" }
                                    }
                                    pre { style: "margin:12px 0 0;white-space:pre-wrap;word-break:break-word;font:inherit;font-size:.8rem;line-height:1.5;color:#e5edf8;",
                                        "{body}"
                                    }
                                    div { style: "display:flex;gap:8px;margin-top:14px;flex-wrap:wrap;",
                                        button { style: "{PRIMARY_BUTTON}", onclick: begin_reply, "Reply" }
                                        button { style: "{SECONDARY_BUTTON}", onclick: delete_selected, "Delete" }
                                    }
                                }
                            }
                        }
                    } else {
                        div { style: "{EMPTY_CARD}",
                            "Select a message to read. Reply uses the purpose inbox it landed on — no address book required."
                        }
                    }
                }
            }
        }
    }
}
