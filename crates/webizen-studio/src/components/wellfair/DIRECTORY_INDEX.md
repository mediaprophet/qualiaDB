---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# wellfair Index

## Functionality Overview

Contains Webizen Studio's Wellfair/Selfhood/Care/Lived-Memory/Practice surfaces and their
desktop-host adapters. The folder currently combines domain panels, Sanctuary and consent
rituals, Anatomy/Chora spatial experiences, library/project/work interfaces, shared
honesty-state components, and low-level Tauri clients.

## File & Subdirectory Manifest

- `accountability_panel.rs`: Accountability and conduct-record presentation.
- `agency_panel.rs`: Agency, rights, and normative-control surface.
- `anatomy_3d_panel.rs`: Dual-vocabulary interactive Anatomy renderer, including public
  GLB acquisition, bounded `.10d` compilation/cache, local GPU rendering and reference-body controls.
- `anatomy_panel.rs`: Flat Anatomy/body overview and related records.
- `assessment_panel.rs`: Wellfair assessment entry and result presentation.
- `audit_panel.rs`: Audit and assurance records.
- `chora_host_client.rs`: Tauri bridge for Chora worlds, time, regions, layers, downloads,
  planting, and GPU camera state.
- `chora_panel.rs`: Naturalised place-and-time and advanced spatio-temporal Chora vocabulary
  over the same universe/world/layer engine, temporal navigation, regions, and GPU layer loading.
- `clinical_panel.rs`: Gated clinical interface.
- `communications_panel.rs`: Wellfair communication pathways using theme-safe field, list, and status contrast.
- `comorbidity_panel.rs`: Ontology-backed Anatomy comorbidity evaluation over the local
  Qualia graph, with organ focus and explicit empty/error states.
- `consent_panel.rs`: Consent-grant and revocation UI.
- `credentials_panel.rs`: Credentials and identity evidence.
- `decoy_retention_panel.rs`: Decoy/retention safeguards.
- `disclosure_inquiry_panel.rs`: Disclosure inquiry and response controls.
- `finance_panel.rs`: Finance records and project-adjacent financial acts.
- `guardianship_panel.rs`: Guardianship and multi-party consent.
- `health_panel.rs`: Health summaries and entry points.
- `host_client/`: Typed desktop-host adapters. See `host_client/DIRECTORY_INDEX.md`.
- `host_dto.rs`: Shared DTOs for host responses.
- `library_panel.rs`: Lived Memory search, ingest, facets, timeline/map, observer projection,
  morphology, sharing, and advanced controls.
- `life_panel.rs`: Personal life-record overview.
- `medication_panel.rs`: Medication records.
- `mod.rs`: Module routing and exported Wellfair panels.
- `pairing_panel.rs`: Device/pairing interaction.
- `personal_panel.rs`: Personal profile and life-data controls.
- `projects_panel.rs`: Wellfair project creation and contribution logging.
- `qapp_publish_panel.rs`: qApp publishing from the Wellfair host.
- `receipts_panel.rs`: Receipts and transaction evidence.
- `safeguards_panel.rs`: Safeguard configuration and status.
- `sanctuary_panel.rs`: Sanctuary unlock, protected state, and boundary ritual.
- `scorecard_panel.rs`: Configurable accountability/scorecard presentation.
- `semantic_library.rs`: Naturalised three-pane Semantic Library, semantic collections,
  search, native file import routing, and provenance-receipt registration.
- `semantic_library/`: Focused Semantic Library view components for overview, pipeline,
  item inspection, metrics, and empty collections.
- `shared/`: Reusable sensitivity, provenance, offline, sync, and domain-chrome components.
  See `shared/DIRECTORY_INDEX.md`.
- `shell.rs`: Wellfair multi-panel shell and internal navigation.
- `sleep_panel.rs`: Sleep/wellbeing records.
- `social_book_panel.rs`: Social-book and relation records.
- `sync_backup_panel.rs`: Backup and synchronisation workflow.
- `sync_panel.rs`: Sync state and actions.
- `tools_panel.rs`: Wellfair technical tools.
- `welfare_panel.rs`: Welfare-support interface.
- `wellbeing_panel.rs`: Wellbeing logging and history.
- `work_board_panel.rs`: Project work-board and immutable transitions.

## Changelog

- **2026-07-29**: Created a semantic index during the capability naturalisation audit,
  including Anatomy, Chora, Memory, Practice, Care, Sanctuary, and host/shared layers.
- **2026-07-29**: Documented the 0.0.28 natural/technical Chora and Anatomy vocabulary,
  GLB-to-`.10d` path, and communications contrast repair.
- **2026-07-29**: Added the naturalised Semantic Library and real Anatomy comorbidity
  surface while retaining the original library as the Advanced Technical workbench.
