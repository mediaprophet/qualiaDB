//! Pure helpers for the Talk → Mail daily inbox.
//!
//! Keep presentation logic here so the pane can stay a thin reader of
//! `mail_list` / `mail_get` / `mail_receiver_*` without Domains admin soup.

/// Local SMTP receiver as the inbox understands it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiverState {
    Running { bind: String },
    Held,
}

/// A purpose (or relationship) mailbox shown in the inbox sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailboxChip {
    pub address: String,
    pub local_part: String,
    pub label: String,
    pub kind: MailboxKind,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxKind {
    Purpose,
    Relationship,
    Other,
}

/// Draft produced by Reply — no Directory lookup required.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyDraft {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl ReceiverState {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// Map `mail_receiver_status` JSON onto a held / running state.
pub fn receiver_from_status(value: &serde_json::Value) -> ReceiverState {
    let running = value
        .get("running")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !running {
        return ReceiverState::Held;
    }
    let bind = value
        .get("bind")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            value
                .get("default_bind")
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "127.0.0.1:2525".to_string());
    ReceiverState::Running { bind }
}

pub fn text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn mailbox_kind(value: &serde_json::Value) -> MailboxKind {
    match value.get("kind").and_then(serde_json::Value::as_str) {
        Some("Purpose") => MailboxKind::Purpose,
        Some("Relationship") => MailboxKind::Relationship,
        _ => MailboxKind::Other,
    }
}

pub fn mailbox_label(local: &str) -> String {
    match local.trim().to_ascii_lowercase().as_str() {
        "frontdoor" => "Front door".into(),
        "junkmail" => "Junk".into(),
        "mygov" => "Government".into(),
        "newsletters" => "Newsletters".into(),
        "catchall" => "Catch-all".into(),
        other if other.is_empty() => "Mailbox".into(),
        other => other.to_string(),
    }
}

/// Purpose inboxes first, then relationship addresses. Disabled mailboxes stay visible.
pub fn mailbox_chips(addresses: &[serde_json::Value]) -> Vec<MailboxChip> {
    let mut chips: Vec<MailboxChip> = addresses
        .iter()
        .filter_map(|value| {
            let address = text(value, "address");
            if address.is_empty() {
                return None;
            }
            let local_part = {
                let raw = text(value, "local_part");
                if raw.is_empty() {
                    address.split('@').next().unwrap_or_default().to_string()
                } else {
                    raw
                }
            };
            Some(MailboxChip {
                address,
                label: mailbox_label(&local_part),
                kind: mailbox_kind(value),
                enabled: value
                    .get("enabled")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true),
                local_part,
            })
        })
        .collect();
    chips.sort_by(|a, b| {
        kind_rank(a.kind).cmp(&kind_rank(b.kind)).then_with(|| {
            a.local_part
                .to_ascii_lowercase()
                .cmp(&b.local_part.to_ascii_lowercase())
        })
    });
    chips
}

fn kind_rank(kind: MailboxKind) -> u8 {
    match kind {
        MailboxKind::Purpose => 0,
        MailboxKind::Relationship => 1,
        MailboxKind::Other => 2,
    }
}

pub fn message_mailbox(message: &serde_json::Value) -> String {
    let mailbox = text(message, "mailbox");
    if mailbox.is_empty() {
        text(message, "to_address")
    } else {
        mailbox
    }
}

/// Filter landed mail by selected mailbox. `mailbox == None` is All.
pub fn filter_messages<'a>(
    messages: &'a [serde_json::Value],
    mailbox: Option<&str>,
    include_quarantine: bool,
) -> Vec<&'a serde_json::Value> {
    messages
        .iter()
        .filter(|message| {
            if !include_quarantine
                && message
                    .get("quarantined")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            {
                return false;
            }
            match mailbox {
                None => true,
                Some(selected) => {
                    message_mailbox(message).eq_ignore_ascii_case(selected)
                        || text(message, "to_address").eq_ignore_ascii_case(selected)
                }
            }
        })
        .collect()
}

pub fn unread_in_mailbox(messages: &[serde_json::Value], mailbox: Option<&str>) -> usize {
    filter_messages(messages, mailbox, true)
        .into_iter()
        .filter(|message| {
            !message
                .get("read")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .count()
}

pub fn reply_draft(message: &serde_json::Value) -> ReplyDraft {
    let from = {
        let mailbox = message_mailbox(message);
        if mailbox.is_empty() {
            text(message, "to_address")
        } else {
            mailbox
        }
    };
    let to = text(message, "from_address");
    let subject = reply_subject(&text(message, "subject"));
    let original = text(message, "body");
    let quoted = if original.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n---\nOn {to} wrote:\n{}",
            original
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    ReplyDraft {
        from,
        to,
        subject,
        body: quoted,
    }
}

pub fn reply_subject(subject: &str) -> String {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        "Re: (no subject)".into()
    } else if trimmed.to_ascii_lowercase().starts_with("re:") {
        trimmed.to_string()
    } else {
        format!("Re: {trimmed}")
    }
}

/// Outbound SMTP is optional. Empty/missing host → held, not a fake send.
pub fn smtp_ready(transport: &serde_json::Value) -> bool {
    transport
        .get("smtp")
        .and_then(|smtp| smtp.get("host"))
        .and_then(serde_json::Value::as_str)
        .map(|host| !host.trim().is_empty())
        .unwrap_or(false)
}

pub fn extract_messages(list: &serde_json::Value) -> Vec<serde_json::Value> {
    list.get("messages")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .or_else(|| list.as_array().cloned())
        .unwrap_or_default()
}

pub fn extract_addresses(list: &serde_json::Value) -> Vec<serde_json::Value> {
    list.as_array()
        .cloned()
        .or_else(|| {
            list.get("addresses")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
}

pub fn inbox_counts_line(list: &serde_json::Value) -> String {
    if let Some(counts) = list.get("counts") {
        let total = counts.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let unread = counts.get("unread").and_then(|v| v.as_u64()).unwrap_or(0);
        let quarantine = counts
            .get("quarantine")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        return format!("{total} landed · {unread} unread · {quarantine} held in quarantine");
    }
    let n = extract_messages(list).len();
    format!("{n} landed")
}

/// Compact UTC stamp without pulling chrono into the studio crate.
pub fn format_received_at(unix: u64) -> String {
    if unix == 0 {
        return String::new();
    }
    let days = unix / 86_400;
    let rem = unix % 86_400;
    let hour = rem / 3_600;
    let min = (rem % 3_600) / 60;
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}")
}

/// Howard Hinnant civil-from-days (UTC, Unix epoch).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_097) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn receiver_held_when_not_running() {
        assert_eq!(
            receiver_from_status(&json!({ "running": false })),
            ReceiverState::Held
        );
        assert_eq!(receiver_from_status(&json!({})), ReceiverState::Held);
    }

    #[test]
    fn receiver_running_uses_bind() {
        assert_eq!(
            receiver_from_status(&json!({
                "running": true,
                "bind": "0.0.0.0:2525"
            })),
            ReceiverState::Running {
                bind: "0.0.0.0:2525".into()
            }
        );
    }

    #[test]
    fn purpose_inboxes_sort_ahead_of_relationship() {
        let chips = mailbox_chips(&[
            json!({
                "address": "bob@alice.example",
                "local_part": "bob",
                "kind": "Relationship",
                "enabled": true
            }),
            json!({
                "address": "frontdoor@alice.example",
                "local_part": "frontdoor",
                "kind": "Purpose",
                "enabled": true
            }),
        ]);
        assert_eq!(chips.len(), 2);
        assert_eq!(chips[0].label, "Front door");
        assert_eq!(chips[0].kind, MailboxKind::Purpose);
        assert_eq!(chips[1].kind, MailboxKind::Relationship);
    }

    #[test]
    fn filter_and_unread_respect_mailbox_and_quarantine() {
        let messages = vec![
            json!({
                "mailbox": "frontdoor@a.example",
                "to_address": "frontdoor@a.example",
                "read": false,
                "quarantined": false
            }),
            json!({
                "mailbox": "junkmail@a.example",
                "to_address": "junkmail@a.example",
                "read": false,
                "quarantined": true
            }),
        ];
        assert_eq!(filter_messages(&messages, None, false).len(), 1);
        assert_eq!(
            filter_messages(&messages, Some("junkmail@a.example"), true).len(),
            1
        );
        assert_eq!(unread_in_mailbox(&messages, Some("frontdoor@a.example")), 1);
        assert_eq!(unread_in_mailbox(&messages, None), 2);
    }

    #[test]
    fn reply_draft_uses_mailbox_as_from_without_directory() {
        let draft = reply_draft(&json!({
            "from_address": "friend@elsewhere.example",
            "to_address": "frontdoor@alice.example",
            "mailbox": "frontdoor@alice.example",
            "subject": "Hello",
            "body": "Can you read this?"
        }));
        assert_eq!(draft.from, "frontdoor@alice.example");
        assert_eq!(draft.to, "friend@elsewhere.example");
        assert_eq!(draft.subject, "Re: Hello");
        assert!(draft.body.contains("> Can you read this?"));
    }

    #[test]
    fn reply_subject_does_not_stack_re() {
        assert_eq!(reply_subject("Re: Hello"), "Re: Hello");
        assert_eq!(reply_subject(""), "Re: (no subject)");
    }

    #[test]
    fn smtp_ready_requires_a_host() {
        assert!(!smtp_ready(&json!({ "smtp": null })));
        assert!(!smtp_ready(&json!({ "smtp": { "host": "  " } })));
        assert!(smtp_ready(
            &json!({ "smtp": { "host": "smtp.example.org" } })
        ));
    }

    #[test]
    fn counts_line_reads_mail_list_shape() {
        let line = inbox_counts_line(&json!({
            "messages": [],
            "counts": { "total": 4, "unread": 2, "quarantine": 1 }
        }));
        assert_eq!(line, "4 landed · 2 unread · 1 held in quarantine");
    }

    #[test]
    fn unix_epoch_formats_utc() {
        assert_eq!(format_received_at(0), "");
        assert_eq!(format_received_at(1_704_067_200), "2024-01-01 00:00");
    }
}
