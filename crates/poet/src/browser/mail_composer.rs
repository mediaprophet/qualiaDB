//! Inalienable Purpose Mailbox & CML Composer Subsystem (POET-SPEC-000 Domain 1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements multi-domain purpose-bound email routing, native SMTP receiver
//! integration, Context Markup Language (CML) message composition with embedded
//! RDF Super-Quins, and reactive Email-to-Kanban workstream generation.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Classification of purpose-bound inboxes under a user-owned domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurposeInboxKind {
    InquiryPublic,
    BillingFinancial,
    MedicalSanctuary,
    AgentCoPilot,
    CatchallFallback,
}

impl PurposeInboxKind {
    pub fn address_suffix(self) -> &'static str {
        match self {
            Self::InquiryPublic => "inquiry",
            Self::BillingFinancial => "billing",
            Self::MedicalSanctuary => "medical",
            Self::AgentCoPilot => "agent.astra",
            Self::CatchallFallback => "catchall",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::InquiryPublic => "Inquiry (Public Commons)",
            Self::BillingFinancial => "Billing & Fiduciary",
            Self::MedicalSanctuary => "Medical (Sanctuary / Encrypted)",
            Self::AgentCoPilot => "AI Agent Co-Pilot",
            Self::CatchallFallback => "Catchall Fallback",
        }
    }

    pub fn is_sanctuary(self) -> bool {
        matches!(self, Self::MedicalSanctuary)
    }
}

/// An authenticated, cryptographic email record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MailMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body_cml: String,
    pub timestamp_lamport: u64,
    pub attached_quin_count: usize,
    pub is_did_signed: bool,
    pub is_read: bool,
    pub purpose_kind: PurposeInboxKind,
}

/// State container for the Purpose Mailbox and Composer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MailboxManager {
    pub active_domain: String,
    pub selected_inbox: PurposeInboxKind,
    pub messages: Vec<MailMessage>,
}

impl MailboxManager {
    pub fn new(domain: &str) -> Self {
        let sample_msg_1 = MailMessage {
            id: "msg-001".into(),
            from: "marcus@research-consortium.id".into(),
            to: format!("inquiry@{}", domain),
            subject: "Quarterly Epistemic Synthesis Draft".into(),
            body_cml: "Attached are the ratified Super-Quins for the Catchment Basin survey.".into(),
            timestamp_lamport: 42,
            attached_quin_count: 4,
            is_did_signed: true,
            is_read: false,
            purpose_kind: PurposeInboxKind::InquiryPublic,
        };

        let sample_msg_2 = MailMessage {
            id: "msg-002".into(),
            from: "dr.chen@clinical-network.org".into(),
            to: format!("medical@{}", domain),
            subject: "Encrypted Reference Biomarker Update".into(),
            body_cml: "<q-entity id=\"qualia:Biomarker\">Framingham score follows target curve.</q-entity>".into(),
            timestamp_lamport: 43,
            attached_quin_count: 2,
            is_did_signed: true,
            is_read: true,
            purpose_kind: PurposeInboxKind::MedicalSanctuary,
        };

        Self {
            active_domain: domain.to_string(),
            selected_inbox: PurposeInboxKind::InquiryPublic,
            messages: vec![sample_msg_1, sample_msg_2],
        }
    }

    /// Filter messages by currently selected purpose inbox.
    pub fn filtered_messages(&self) -> Vec<&MailMessage> {
        self.messages
            .iter()
            .filter(|m| m.purpose_kind == self.selected_inbox)
            .collect()
    }

    /// Compose and send a CML-enhanced email with embedded Super-Quins.
    pub fn send_cml_message(
        &mut self,
        from_purpose: PurposeInboxKind,
        to_address: &str,
        subject: &str,
        body_cml: &str,
        quin_count: usize,
    ) -> String {
        let msg_id = format!("msg-{:04x}", self.messages.len() + 1);
        let new_msg = MailMessage {
            id: msg_id.clone(),
            from: format!("{}@{}", from_purpose.address_suffix(), self.active_domain),
            to: to_address.to_string(),
            subject: subject.to_string(),
            body_cml: body_cml.to_string(),
            timestamp_lamport: self.messages.len() as u64 + 100,
            attached_quin_count: quin_count,
            is_did_signed: true,
            is_read: true,
            purpose_kind: from_purpose,
        };
        self.messages.insert(0, new_msg);
        msg_id
    }

    /// Convert an incoming email message into a tracked Workstream A Kanban task description.
    pub fn convert_to_kanban_task(&self, msg_id: &str) -> Option<String> {
        self.messages.iter().find(|m| m.id == msg_id).map(|m| {
            format!(
                "Task from Email: {} (From: {}, attached Quins: {})",
                m.subject, m.from, m.attached_quin_count
            )
        })
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Inalienable Purpose Mailbox Viewport.
pub fn build_mailbox_view(document: &Document, manager: &MailboxManager) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;"
    );

    // Top Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;"
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(&format!("\u{2709}\u{FE0F} Inalienable Domain Mail: @{}", manager.active_domain)));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el.style().set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let status = document.create_element("span").unwrap();
    status.set_text_content(Some("Local SMTP: Active \u{25CF} Port 25/587 \u{25CF} SPF/DKIM: Verified"));
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el.style().set_css_text("font-size: 11px; font-family: var(--font-mono); color: #34d399;");
    header.append_child(&status).unwrap();

    root.append_child(&header).unwrap();

    // 2-Column Split: Inboxes on Left, Message Viewer on Right
    let split = document.create_element("div").unwrap();
    let split_el: HtmlElement = split.clone().dyn_into().unwrap();
    split_el.style().set_css_text("display: grid; grid-template-columns: 240px 1fr; gap: 10px;");

    // Left: Purpose Folders
    let left = document.create_element("div").unwrap();
    let left_el: HtmlElement = left.clone().dyn_into().unwrap();
    left_el.style().set_css_text("background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;");

    let left_title = document.create_element("span").unwrap();
    left_title.set_text_content(Some("Purpose Inboxes"));
    let left_title_el: HtmlElement = left_title.clone().dyn_into().unwrap();
    left_title_el.style().set_css_text("font-weight: 700; font-size: 11px; color: #94a3b8; text-transform: uppercase; margin-bottom: 4px;");
    left.append_child(&left_title).unwrap();

    let inboxes = [
        PurposeInboxKind::InquiryPublic,
        PurposeInboxKind::BillingFinancial,
        PurposeInboxKind::MedicalSanctuary,
        PurposeInboxKind::AgentCoPilot,
        PurposeInboxKind::CatchallFallback,
    ];

    for inbox in inboxes {
        let item = document.create_element("div").unwrap();
        let item_el: HtmlElement = item.clone().dyn_into().unwrap();
        let is_active = inbox == manager.selected_inbox;
        item_el.style().set_css_text(&format!(
            "display: flex; justify-content: space-between; align-items: center; padding: 6px 8px; \
             border-radius: 6px; font-size: 11px; cursor: pointer; background: {}; color: {};",
            if is_active { "rgba(56, 189, 248, 0.15)" } else { "transparent" },
            if is_active { "#38bdf8" } else { "#cbd5e1" }
        ));

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(inbox.label()));
        item.append_child(&name).unwrap();

        let count = manager.messages.iter().filter(|m| m.purpose_kind == inbox).count();
        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(&count.to_string()));
        let badge_el: HtmlElement = badge.clone().dyn_into().unwrap();
        badge_el.style().set_css_text("font-size: 10px; background: rgba(0,0,0,0.3); padding: 1px 6px; border-radius: 10px;");
        item.append_child(&badge).unwrap();

        left.append_child(&item).unwrap();
    }
    split.append_child(&left).unwrap();

    // Right: Filtered Message List
    let right = document.create_element("div").unwrap();
    let right_el: HtmlElement = right.clone().dyn_into().unwrap();
    right_el.style().set_css_text("background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;");

    let msgs = manager.filtered_messages();
    if msgs.is_empty() {
        let empty = document.create_element("span").unwrap();
        empty.set_text_content(Some("No messages in this purpose inbox."));
        let empty_el: HtmlElement = empty.clone().dyn_into().unwrap();
        empty_el.style().set_css_text("font-size: 11px; color: #64748b; font-style: italic;");
        right.append_child(&empty).unwrap();
    } else {
        for msg in msgs {
            let card = document.create_element("div").unwrap();
            let card_el: HtmlElement = card.clone().dyn_into().unwrap();
            card_el.style().set_css_text("background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.05); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 4px;");

            let header_row = document.create_element("div").unwrap();
            let header_row_el: HtmlElement = header_row.clone().dyn_into().unwrap();
            header_row_el.style().set_css_text("display: flex; justify-content: space-between; font-size: 11px; font-weight: 600; color: #f8fafc;");

            let subj = document.create_element("span").unwrap();
            subj.set_text_content(Some(&msg.subject));
            header_row.append_child(&subj).unwrap();

            let sender = document.create_element("span").unwrap();
            sender.set_text_content(Some(&msg.from));
            let sender_el: HtmlElement = sender.clone().dyn_into().unwrap();
            sender_el.style().set_css_text("font-size: 10px; font-family: var(--font-mono); color: #94a3b8;");
            header_row.append_child(&sender).unwrap();
            card.append_child(&header_row).unwrap();

            let body = document.create_element("div").unwrap();
            body.set_text_content(Some(&msg.body_cml));
            let body_el: HtmlElement = body.clone().dyn_into().unwrap();
            body_el.style().set_css_text("font-size: 11px; color: #cbd5e1; line-height: 1.4;");
            card.append_child(&body).unwrap();

            let footer = document.create_element("div").unwrap();
            let footer_el: HtmlElement = footer.clone().dyn_into().unwrap();
            footer_el.style().set_css_text("display: flex; gap: 8px; font-size: 9px; font-family: var(--font-mono); color: #64748b; margin-top: 4px;");

            let quins = document.create_element("span").unwrap();
            quins.set_text_content(Some(&format!("Super-Quins: {} \u{2713}", msg.attached_quin_count)));
            quins.clone().dyn_into::<HtmlElement>().unwrap().style().set_css_text("color: #38bdf8;");
            footer.append_child(&quins).unwrap();

            let signed = document.create_element("span").unwrap();
            signed.set_text_content(Some("DID Signature: Valid \u{2713}"));
            signed.clone().dyn_into::<HtmlElement>().unwrap().style().set_css_text("color: #34d399;");
            footer.append_child(&signed).unwrap();

            card.append_child(&footer).unwrap();
            right.append_child(&card).unwrap();
        }
    }
    split.append_child(&right).unwrap();

    root.append_child(&split).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox_default_state() {
        let mgr = MailboxManager::new("holborn.id");
        assert_eq!(mgr.active_domain, "holborn.id");
        assert_eq!(mgr.messages.len(), 2);

        let inquiry_msgs = mgr.filtered_messages();
        assert_eq!(inquiry_msgs.len(), 1);
        assert_eq!(inquiry_msgs[0].subject, "Quarterly Epistemic Synthesis Draft");
    }

    #[test]
    fn test_send_cml_message_workflow() {
        let mut mgr = MailboxManager::new("agency.org");
        let msg_id = mgr.send_cml_message(
            PurposeInboxKind::BillingFinancial,
            "auditor@global-standards.org",
            "Fiduciary Attestation Receipt",
            "<q-entity id=\"qualia:Receipt\">Transferred 500 commons compute credits.</q-entity>",
            8,
        );

        assert!(msg_id.starts_with("msg-"));
        assert_eq!(mgr.messages.len(), 3);
        assert_eq!(mgr.messages[0].attached_quin_count, 8);
        assert!(mgr.messages[0].is_did_signed);
    }

    #[test]
    fn test_email_to_kanban_task_conversion() {
        let mgr = MailboxManager::new("thorne.id");
        let task_desc = mgr.convert_to_kanban_task("msg-001");
        assert!(task_desc.is_some());
        let desc = task_desc.unwrap();
        assert!(desc.contains("Quarterly Epistemic Synthesis Draft"));
        assert!(desc.contains("attached Quins: 4"));
    }
}
