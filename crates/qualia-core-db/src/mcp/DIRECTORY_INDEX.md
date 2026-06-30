---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# mcp Index

## Functionality Overview
Comprehensive index of functionality for `mcp`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mcp_cooperation.rs`
  - `struct CallerStandpoint`
  - `enum CooperationVerdict`
  - `fn caller_grounded`
  - `fn authorize`
  - `fn authorize_call`
  - `fn enforcement_enabled`
  - `fn sp`
  - `fn t`
  - `fn unverified_caller_is_denied`
  - `fn ungrounded_caller_is_denied`
  - `fn verified_grounded_ordinary_call_is_authorized`
  - `fn non_derogable_violation_request_is_blocked_by_policy`
  - `fn authorize_call_resolves_grounding_from_the_graph`
- 📄 `mcp_format_impls.rs`
  - `fn json_str`
  - `fn json_u64`
  - `fn json_bool`
  - `fn quin_to_json`
  - `fn apply_context`
  - `fn resolve_predicate_hash`
  - `fn parse_csv_datatype`
  - `fn parse_json_datatype`
  - `fn parse_csv_profile`
  - `fn parse_json_profile`
  - `fn read_input_bytes`
  - `struct QuinCollector`
  - `impl QuinCollector`
  - `fn new`
  - `fn push`
  - *(...and 15 more)*
- 📄 `mcp_server.rs`
  - `enum McpRuntimeState`
  - `enum McpSystemError`
  - `struct McpIntentFrame`
  - `struct RawToolPayload`
  - `struct McpToolDescriptor`
  - `fn extract_raw_json_string`
  - `fn stable_mcp_tools`
  - `fn daemon_health_ok`
  - `fn resolve_repo_root`
  - `fn hex_nibble`
  - `fn parse_sanctuary_override`
  - `fn build_intent_frame`
  - `fn tool_list_json`
  - `fn system_resource_json`
  - `fn error_message`
  - *(...and 15 more)*
- 📄 `mcp_stub_impls.rs`
  - `fn json_str`
  - `fn json_u64`
  - `fn qualia_storage_path`
  - `fn qapps_root`
  - `fn list_qapp_dir_names`
  - `fn resolve_qapp_dir`
  - `fn bundled_qapp_source_candidates`
  - `fn read_qapp_manifest_value`
  - `fn quins_to_ntriples`
  - `fn quin_to_json`
  - `fn query_sparql`
  - `fn get_graph_stats`
  - `fn list_ontologies`
  - `fn llm_infer`
  - `fn llm_chat`
  - *(...and 29 more)*
- 📄 `mcp_tool_impls.rs`
  - `fn parse_tool_args`
  - `fn list_capabilities`
  - `fn json_str`
  - `fn json_f64`
  - `fn json_u64`
  - `fn json_bool`
  - `fn json_f64_array`
  - `fn json_u8_array`
  - `fn parse_quin`
  - `fn parse_quin_slice`
  - `fn ensure_parity`
  - `fn parse_matrix_def`
  - `fn matrix_operation`
  - `fn algebra_solve_polynomial`
  - `fn algebra_matrix_analyze`
  - *(...and 37 more)*
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
