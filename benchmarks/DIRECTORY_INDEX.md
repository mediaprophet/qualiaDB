---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# benchmarks Index

## Functionality Overview
Comprehensive index of functionality for `benchmarks`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[comunica](comunica/DIRECTORY_INDEX.md)`
- 📁 `[data](data/DIRECTORY_INDEX.md)`
- 📁 `[oxigraph](oxigraph/DIRECTORY_INDEX.md)`
- 📁 `[qualia](qualia/DIRECTORY_INDEX.md)`
- 📁 `[qualia_wasm](qualia_wasm/DIRECTORY_INDEX.md)`
- 📁 `[results](results/DIRECTORY_INDEX.md)`
- 📁 `[surrealdb](surrealdb/DIRECTORY_INDEX.md)`
- 📁 `[wasm_prolog](wasm_prolog/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `common.py`
  - `def latency_stats_ms`
  - `def peak_rss_mb`
  - `def apply_512mb_limit`
  - `def file_size_bytes`
  - `def file_size_mb`
  - `def record_dataset_file_metrics`
  - `def generate_ntriples`
- 📄 `datasets.py`
  - `def _workspace_root`
  - `def _schemaorg_source_path`
  - `def _schemaorg_q42_base_path`
  - `def _synthetic_profile`
  - `def _scan_external_nt`
  - `def _schemaorg_profile`
  - `def load_dataset_profile`
  - `def list_dataset_profiles`
- 📄 `environment.py`
  - `def _utc_now`
  - `def _host_class`
  - `def _total_ram_gb`
  - `def collect_device_manifest`
  - `def collect_ci_environment`
  - `def _harness_runner_label`
  - `def collect_harness_environment`
  - `def fetch_daemon_health`
  - `def fetch_daemon_execution_environment`
  - `def merge_execution_environment`
- 📄 `harness.py`
  - `def _qualia_format_engines`
  - `def _qualia_daemon_healthy`
  - `def normalize_result`
  - `def run_engine`
  - `def merge_into_output`
  - `def main`
- 📄 `requirements.txt`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
