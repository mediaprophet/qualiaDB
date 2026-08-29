# POET Social & Communication Specification

**Document ID:** `POET-SPEC-005`  
**Status:** Canonical Domain Specification  
**Scope:** DID-attributed messaging, threaded channels, mention feeds, Semantic Library attachments, and channel moderation in POET.

---

## 1. Overview & Communication Paradigm

Social communication in POET is peer-to-peer, DID-attributed, and privacy-preserving. Users communicate directly across semantic channels, retain full ownership of their communication archives, and attach rich knowledge artifacts from the Semantic Library without corporate platform surveillance.

```
+-----------------------------------------------------------------------------------+
|                        SOCIAL & COMMUNICATION TOPOLOGY                            |
+-----------------------------------------------------------------------------------+
|  [Semantic Channels & Threads]  <===> [Rich Message Composer]                     |
|  - Topic IRIs & visibility            - Markdown / rich text authoring            |
|  - Open / Request / Invite policies   - @DID mentions (bounded up to 16)          |
|  - Threaded replies & references      - Semantic Library attachment references    |
|                                                                                   |
|  [Activity Hub & Notifications] <===> [Channel Administration & Moderation]       |
|  - Recipient-attributed notifications - Creator-appointed moderator roles         |
|  - Irreversible read-state receipts   - Non-destructive hide receipts (no erasure)|
|  - Expiring voluntary presence        - Blocked relationship enforcement          |
+-----------------------------------------------------------------------------------+
```

---

## 2. Channels, Threads & Direct Discussions

- **Channel Policy Types:**
  - `Open`: Any participant DID can join and participate immediately.
  - `Request`: Contributor DIDs submit join requests requiring approval from channel moderators.
  - `Invite`: Closed, private channels accessible only via cryptographic invitations issued by the channel creator.
- **Threaded Discussions:** Full support for branching discussion threads with atomic parent message validation and safe contextual rendering.

---

## 3. Rich Message Composer & Attachments

- **Authoring Experience:** Clean Markdown-enabled editor supporting headings, code blocks, lists, quotes, and emoji.
- **DID Mentions:** Autocomplete for `@DID` and user handles; creates dedicated, immutable `social_notification` records for each mentioned entity.
- **Semantic Library Attachments:**
  - Files, documents, datasets, and media are ingested into the **Semantic Library** first.
  - Messages attach canonical Semantic Library URIs (`attachment_uri`, media type, sensitivity label, and title) rather than embedding unbounded binary blobs in message records.
  - Rendered in UI as secure, interactive previews with provenance cards.

---

## 4. Notifications, Presence & Activity Hub

- **Notification Inbox:** Dedicated hub displaying mention notifications and channel invitation alerts addressed to the active DID.
- **Read State:** Unread notifications display an active badge; marking a notification as read creates a permanent, immutable receipt that cannot be forged or reversed.
- **Voluntary Presence:** Scoped, time-expiring presence announcements (e.g., `Available`, `Focused`, `Away`) published over Pulse SSE without continuous background location tracking.

---

## 5. Channel Administration & Non-Destructive Moderation

- **Role Transitions:** Channel creators can appoint or revoke moderator roles through cryptographically signed participant records.
- **Non-Destructive Moderation:** Moderators can hide abusive or inappropriate messages by emitting attributed `social_moderation_hide` receipts. The original message record is preserved in the audit log for accountability, while public rendering displays a moderation notice.
- **Blocked Relationships:** Blocked DIDs are strictly prevented from sending direct messages or requesting channel access.

---

## 6. Social Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-SOC-001` | **Threaded Channel Conversations** | Multi-channel discussion feed with thread hierarchy and parent message reference validation. | `social_workspace.rs`, `social_lifecycle.rs` |
| `POET-SOC-002` | **Rich Message Composer** | Markdown editor supporting bold/italic, code blocks, lists, and inline emoji. | `social_workspace.rs`, `mail_composer.rs` |
| `POET-SOC-003` | **DID Mentions & Notifications** | `@DID` mention autocomplete generating dedicated recipient notification receipts. | `social_notifications.rs` |
| `POET-SOC-004` | **Semantic Library Attachments** | Attach documents, datasets, and media by referencing canonical Semantic Library URIs. | `social_workspace.rs`, `semantic_library_view.rs` |
| `POET-SOC-005` | **Immutable Read-State Hub** | Recipient notification inbox with unread badges and irreversible read-state transitions. | `social_notifications.rs`, `social_inbox.rs` |
| `POET-SOC-006` | **Scoped Voluntary Presence** | Expiring presence broadcasts over Pulse SSE with customizable status chips. | `social_presence.rs` |
| `POET-SOC-007` | **Channel Role Administration** | Creator appointment of moderators and participant role management. | `social_lifecycle.rs`, `social_moderation.rs` |
| `POET-SOC-008` | **Non-Destructive Moderation** | Moderator hide receipts replacing rendered text while preserving underlying evidence. | `social_moderation.rs` |
| `POET-SOC-009` | **Blocked Relationship Enforcement** | Reject incoming direct messages and requests from DIDs with active block receipts. | `social_lifecycle.rs` |
