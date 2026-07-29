---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# host_client Index

## Functionality Overview

Provides typed WebAssembly-to-Tauri adapters for Wellfair domain operations. These clients
translate Dioxus actions into desktop commands and decode host responses for domain panels.

## File & Subdirectory Manifest

- `accountability.rs`: Accountability record client.
- `agency.rs`: Agency and rights client.
- `anatomy_qapp.rs`: Anatomy qApp/asset acquisition client plus typed comorbidity verdict
  retrieval from the desktop host.
- `anatomy_render.rs`: Cached `.10d` Anatomy renderer client.
- `clinical.rs`: Clinical host operations.
- `decoy.rs`: Decoy/retention operations.
- `disclosure.rs`: Disclosure inquiry operations.
- `encryption.rs`: Protected-data encryption operations.
- `finance.rs`: Finance host operations.
- `guardianship.rs`: Guardianship and multi-party consent operations.
- `keychain.rs`: Key/vault state operations.
- `library.rs`: Lived Memory ingest, query, sharing, and catalogue operations.
- `mod.rs`: Module routing and common invoke helpers.
- `physiological.rs`: Physiology state operations.
- `pwa.rs`: Progressive/offline host integration.
- `safeguards.rs`: Safeguard operations.
- `sync_backup.rs`: Sync and backup operations.
- `view_api.rs`: Shared entity-view session, observer, selection, and morphology client.
- `wellbeing.rs`: Wellbeing host operations.
- `work_items.rs`: Project work-item operations.

## Changelog

- **2026-07-29**: Created a complete host-client manifest and recorded the entity-view,
  Anatomy-render, and domain command boundaries.
- **2026-07-29**: Added the typed Anatomy comorbidity response boundary.
