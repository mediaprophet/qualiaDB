---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# agent_qa Index

## Functionality Overview

Scenario-driven, bounded automated verification for coding agents working on
Webizen. It emits stable JSON, classifies failures and follows the repository's
temporary-artifact ownership and cleanup rules.

## File & Subdirectory Manifest

- `runner.py`: Shell-free scenario executor, artifact budget enforcement,
  process-tree timeout enforcement, failure classification and JSON/Markdown report generation.
- `check_scope.py`: Formats only files owned by this programme, protecting unrelated dirty-tree work.
- `test_runner.py`: Unit tests for failure classification and the shell-free
  scenario manifest contract.
- `scenarios.json`: Versioned contract, compile, self-contained browser UI, and full scenarios.
- `scope.json`: Explicit 0.0.28 Rust formatting ownership boundary.
- `README.md`: Usage, output and retention contract.

## Changelog

- **2026-07-29**: Created the 0.0.28 agent-oriented automated QA harness.
- **2026-07-29**: Added process-tree cleanup, artifact-I/O classification, scoped formatting,
  and a self-contained Playwright UI scenario.
