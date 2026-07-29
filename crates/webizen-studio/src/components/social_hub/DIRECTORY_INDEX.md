---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# social_hub Index

## Functionality Overview

Implements the current Webizen **Relations** habitat mounted at `/talk`. The
surface brings together local/person-to-person chat, people and invite flows,
public reception/domain setup, semantic mail, and cooperative projects. It
calls the desktop social, mail, mesh, vault and WellFair host APIs through the
Tauri bridge.

The folder is capability-rich but currently mixes natural relationship actions
with advanced transport administration. The UX audit at
`docs/audits/webizen-setup-relations-ux-addendum-2026-07-29.md` proposes
preserving these functions behind one Relations information architecture with a
separate Advanced Technical presentation.

## File & Subdirectory Manifest

- `mod.rs`
  - Owns `SocialHub`, the Relations route shell and shared state.
  - Provides Chat, People, Reception, Mail and Projects tabs.
  - Loads setup status and redirects an unconfigured person toward the next
    available task.
  - Embeds `ConnectChat` and the WellFair mail panel.
- `people.rs`
  - Owns profile presentation, signed invites, invite/package acceptance,
    magic links, contacts, social peers and group-chat creation.
  - Starts/stops SocialWebNet mesh connections and edits peer endpoints.
  - Hands accepted project packages into Chat or Projects.
- `reception.rs`
  - Owns public-facing domain identity, domain registration and DNS/front-door
    record presentation.
- `projects.rs`
  - Owns cooperative project creation, membership, project-scoped group chat,
    vault lifecycle and share packages.
- `helpers.rs`
  - Provides Tauri invocation, JSON normalization, clipboard helpers, project
    persistence, vault-state helpers and front-door form loading.
- `types.rs`
  - Defines `HubTab` and the shared visual constants used by the habitat.

## Capability Boundaries

- The Chat tab delegates to `../connect_chat.rs`.
- The Mail tab delegates to the WellFair mail UI; it is distinct from
  `wellfair/communications_panel.rs`, which handles companion live-share
  consent.
- Personal-directory and care/social-book facets also exist outside this
  folder and need a shared relationship view model rather than record
  conflation.
- `/nexus` is a knowledge/research canvas, not the canonical People surface.

## Changelog

- **2026-07-29**: Added a comprehensive source and UX-boundary index during the
  setup, settings, Relations and communications audit.
