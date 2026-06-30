---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# shacl_extensions Index

## Functionality Overview
Comprehensive index of functionality for `shacl_extensions`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `config.rs`
  - `struct LogConfiguration`
  - `struct LogLevel`
  - `struct LogEntry`
  - `struct LogRetention`
  - `struct LogExportFormat`
  - `struct SystemTrayConfiguration`
  - `struct TrayMenuItem`
  - `struct TrayStatusIndicator`
  - `struct TrayAction`
  - `struct StorageConfiguration`
  - `struct NetworkConfiguration`
  - `struct TaxRecipientConfiguration`
  - `struct SecurityConfiguration`
  - `impl LogConfiguration`
  - `fn to_opcodes`
  - *(...and 13 more)*
- 📄 `identity.rs`
  - `enum CryptoScheme`
  - `struct IdentifierBinding`
  - `enum IdentityValidation`
  - `fn validate_enumerated_identity`
  - `fn enumerated_identity_opcodes`
  - `struct ShapeRoute`
  - `fn shapes_for_locus`
  - `fn loci_for_shape`
  - `fn route_is_local`
  - `enum ShaclSeverity`
  - `enum OperationMode`
  - `struct ShapeViolation`
  - `struct DegradationOutcome`
  - `fn degrade_violations`
  - `struct CredentialGate`
  - *(...and 10 more)*
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
