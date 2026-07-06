//! Mail rules engine — evaluates a mailbox's [`MailRules`](crate::domains::MailRules)
//! against an inbound message.
//!
//! This is a **pure** decision layer: no filesystem, no network, no clock. Given the rules
//! configured on an address (a rule-bearing mailbox) and an [`InboundMessage`], it returns a
//! [`MailVerdict`] describing whether the message is delivered, quarantined, or rejected, plus the
//! priority/notify hints and a human-readable trail of `reasons` for auditability.
//!
//! The rules themselves live on [`crate::domains::MailRules`]; this module only interprets them.

use serde::{Deserialize, Serialize};

/// An inbound message presented to the rules engine for a delivery decision.
///
/// This is the minimal envelope the engine needs — it does not carry the message body; delivery
/// decisions here are made from addressing, sender verification state, subject and size alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboundMessage {
    /// The sender's address (e.g. `alice@example.org`).
    pub from_address: String,
    /// The recipient mailbox address this message was delivered to.
    pub to_address: String,
    /// The sender's DID, if one was presented.
    pub sender_did: Option<String>,
    /// Whether the sender's identity was verified (DID-signed / established relationship).
    pub sender_verified: bool,
    /// The message subject line.
    pub subject: String,
    /// The message size in bytes.
    pub size_bytes: usize,
}

/// The outcome of evaluating [`MailRules`](crate::domains::MailRules) against an [`InboundMessage`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MailVerdict {
    /// Whether the message is delivered at all. `false` when rejected.
    pub deliver: bool,
    /// Whether the message, though delivered, is routed to quarantine rather than the inbox.
    pub quarantined: bool,
    /// If the message was rejected, a short human-readable reason; `None` when delivered.
    pub rejected: Option<String>,
    /// Priority hint propagated from the rules (0 = normal; higher = more important).
    pub priority: i8,
    /// Whether the recipient should be notified on receipt.
    pub notify: bool,
    /// A human-readable trail of the rules that fired, for auditability.
    pub reasons: Vec<String>,
}

/// Evaluate `rules` against `msg` and produce a [`MailVerdict`].
///
/// Decision order:
/// 1. If the rules require a verified sender and the sender is not verified, the message is
///    **rejected** (not delivered) and the function returns early.
/// 2. Otherwise the message is **delivered**. If the rules quarantine incoming mail, it is
///    delivered to quarantine rather than the inbox.
/// 3. The `priority` and `notify` hints are carried through from the rules, with reasons recorded
///    for a non-zero priority and for quarantine.
pub fn evaluate(rules: &crate::domains::MailRules, msg: &InboundMessage) -> MailVerdict {
    let mut reasons: Vec<String> = Vec::new();

    // (1) Verified-sender gate — fail closed, no delivery.
    if rules.require_verified_sender && !msg.sender_verified {
        reasons.push("rejected: unverified sender".to_string());
        return MailVerdict {
            deliver: false,
            quarantined: false,
            rejected: Some("unverified sender".to_string()),
            priority: rules.priority,
            notify: rules.notify,
            reasons,
        };
    }

    // (2) Delivered. Quarantine still delivers, but to the quarantine store.
    let quarantined = rules.quarantine;
    if quarantined {
        reasons.push("quarantined by rule".to_string());
    }

    // (3) Priority / notify hints.
    if rules.priority > 0 {
        reasons.push(format!("priority set to {}", rules.priority));
    }

    MailVerdict {
        deliver: true,
        quarantined,
        rejected: None,
        priority: rules.priority,
        notify: rules.notify,
        reasons,
    }
}

/// Compute the retention cutoff (unix seconds) for a message received at `received_unix`.
///
/// Returns `Some(received_unix + retention_days * 86400)` when a finite retention is configured,
/// or `None` when `retention_days == 0` (keep indefinitely).
pub fn retention_cutoff_unix(rules: &crate::domains::MailRules, received_unix: u64) -> Option<u64> {
    if rules.retention_days > 0 {
        Some(received_unix + rules.retention_days as u64 * 86_400)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::MailRules;

    fn msg(sender_verified: bool) -> InboundMessage {
        InboundMessage {
            from_address: "alice@example.org".to_string(),
            to_address: "junkmail@me.example".to_string(),
            sender_did: Some("did:example:alice".to_string()),
            sender_verified,
            subject: "hello".to_string(),
            size_bytes: 1024,
        }
    }

    #[test]
    fn unverified_sender_is_rejected() {
        let rules = MailRules {
            require_verified_sender: true,
            ..Default::default()
        };
        let verdict = evaluate(&rules, &msg(false));
        assert!(!verdict.deliver, "unverified sender must not be delivered");
        assert!(!verdict.quarantined);
        assert_eq!(verdict.rejected.as_deref(), Some("unverified sender"));
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.contains("unverified sender")));
    }

    #[test]
    fn verified_sender_passes_the_gate() {
        let rules = MailRules {
            require_verified_sender: true,
            ..Default::default()
        };
        let verdict = evaluate(&rules, &msg(true));
        assert!(verdict.deliver);
        assert_eq!(verdict.rejected, None);
    }

    #[test]
    fn quarantine_delivers_to_quarantine() {
        let rules = MailRules {
            quarantine: true,
            ..Default::default()
        };
        let verdict = evaluate(&rules, &msg(true));
        assert!(verdict.deliver, "quarantined mail is still delivered");
        assert!(verdict.quarantined);
        assert_eq!(verdict.rejected, None);
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r == "quarantined by rule"));
    }

    #[test]
    fn priority_passes_through() {
        let rules = MailRules {
            priority: 7,
            notify: true,
            ..Default::default()
        };
        let verdict = evaluate(&rules, &msg(true));
        assert_eq!(verdict.priority, 7);
        assert!(verdict.notify);
        assert!(verdict.reasons.iter().any(|r| r.contains("priority set to 7")));
    }

    #[test]
    fn retention_cutoff_some_and_none() {
        let received: u64 = 1_000_000;

        let keep_forever = MailRules {
            retention_days: 0,
            ..Default::default()
        };
        assert_eq!(retention_cutoff_unix(&keep_forever, received), None);

        let thirty_days = MailRules {
            retention_days: 30,
            ..Default::default()
        };
        assert_eq!(
            retention_cutoff_unix(&thirty_days, received),
            Some(received + 30 * 86_400)
        );
    }
}
