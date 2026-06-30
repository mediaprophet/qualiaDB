---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# identity Index

## Functionality Overview
Comprehensive index of functionality for `identity`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[credentials](credentials/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `access_modality.rs`
  - `enum AccessModality`
  - `enum DataTier`
  - `enum AccessError`
  - `impl AccessModality`
  - `fn can_access`
  - `fn traditional_web_access`
  - `fn human_centric_access`
- 📄 `agency.rs`
  - `enum AgencyError`
  - `fn compute_scoped_merkle_root`
  - `fn sign_agency_root`
  - `fn verify_human_agency`
  - `fn stamp_fiduciary_metadata`
  - `fn scrub_quin_volatile`
  - `fn sign_graph_mutation`
  - `fn derive_lane_key`
  - `fn test_human_agency_verification`
  - `fn derive_lane_key_is_deterministic_and_salt_bound`
- 📄 `identifier.rs`
  - `enum IdentifierError`
  - `fn parse_did_q42`
  - `fn fnv1a`
  - `fn test_valid_did_q42_msb`
  - `fn test_invalid_prefix`
  - `fn test_empty_payload`
  - `fn test_bare_prefix_without_colon`
  - `fn test_deterministic_output`
  - `fn test_distinct_payloads_produce_distinct_pointers`
  - `fn pointer_is_base_hash_or_msb`
- 📄 `key_vault.rs`
  - `struct KeyVault`
  - `impl KeyVault`
  - `fn new`
  - `fn load_or_generate`
  - `fn derive_key`
  - `fn sign_payload`
  - `fn get_master_key_bytes`
  - `fn public_key_bytes_for_context`
  - `fn verify_signature`
  - `fn generate_webid_tls_cert`
  - `fn issue_qapp_token`
  - `fn verify_qapp_token`
  - `struct QappTokenPayload`
  - `enum SubgraphLayer`
  - `impl SubgraphLayer`
  - *(...and 22 more)*
- 📄 `mod.rs`
- 📄 `profiles.rs`
  - `struct CapabilityProfile`
  - `impl CapabilityProfile`
  - `fn allows_engine`
  - `fn allows_intent`
  - `fn has_ontology`
- 📄 `vault_manifest.rs`
  - `struct VaultManifest`
  - `struct VocabularyLD`
  - `struct TermDefinition`
  - `struct CollectionLD`
  - `struct CapabilityLD`
  - `struct VaultManifestProcessor`
  - `impl VaultManifestProcessor`
  - `fn from_volume`
  - `fn to_cbor_ld`
  - `fn from_cbor_ld`
  - `fn to_compact_cbor_ld`
  - `fn from_compact_cbor_ld`
  - `fn validate_manifest`
  - `fn q42_context`
  - `struct CompactVaultManifest`
  - *(...and 15 more)*
- 📄 `webizen_identifiers.rs`
  - `struct WebizenIdentity`
  - `impl WebizenIdentity`
  - `fn new`
  - `fn with_did`
  - `fn is_webizen_id`
  - `struct WebizenRegistry`
  - `impl WebizenRegistry`
  - `fn register_webizen`
  - `fn get_webizen`
  - `fn get_webizen_by_webid`
  - `fn verify_signature`
  - `fn verify_quin_signature`
  - `fn serialize_quin_for_signature`
  - `impl Default`
  - `fn default`
  - *(...and 10 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
