---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# bin Index

## Functionality Overview
Comprehensive index of functionality for `bin`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `cli.rs`
  - `struct Cli`
  - `enum Commands`
  - `fn main`
  - `fn list_extensions`
  - `fn execute_job`
  - `fn check_status`
  - `fn start_daemon`
  - `fn test_extension`
  - `fn test_cli_parsing`
- 📄 `daemon.rs`
  - `fn main`
  - `fn handle_connection`
  - `fn parse_job_request`
  - `fn send_result_to_core`
  - `fn test_job_parsing`
  - `fn test_extension_manager`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
