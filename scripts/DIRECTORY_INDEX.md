---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# scripts Index

## Functionality Overview
Comprehensive index of functionality for `scripts`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[cross-linux](cross-linux/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `auto_transcode_and_scrub.ps1`
  - Sequential model transcoding (.safetensors, .gguf) -> .p64 & build artifact scrubbing engine
- 📄 `build_frontend.ps1`
- 📄 `build_master_provenance.py`
- 📄 `build_w3c_mail_q42.sh`
- 📄 `fetch_massive_datasets.ps1`
- 📄 `fetch_wordnet.sh`
- 📄 `fetch_wordnet_release.ps1`
- 📄 `fetch_wordnet_release.sh`
- 📄 `fix_docs_nav.py`
  - `def main`
- 📄 `generate_icons.sh`
- 📄 `import_lod_cloud_catalog.py`
  - `class ProbeResult`
  - `class DatasetReport`
  - `def slug_id`
  - `def normalize_license`
  - `def media_to_format`
  - `def estimate_size_mb`
  - `def categorize`
  - `def lod_status_ok`
  - `def probe_url`
  - `def _probe_get`
  - `def collect_candidate_urls`
  - `def pick_best_download`
  - `def build_catalog_entry`
  - `def load_lod_data`
  - `def load_existing_ontology_ids`
  - *(...and 6 more)*
- 📄 `ingest_princeton_wordnet.ps1`
- 📄 `ingest_princeton_wordnet.sh`
- 📄 `inventory_w3c_archives.py`
  - `def load_existing_namespaces`
  - `def normalize_ns`
  - `def normalize_ns_key`
  - `def extract_namespace`
  - `def valid_namespace`
  - `def file_score`
  - `def slugify`
  - `def label_from_slug`
  - `def prefix_from_ns`
  - `def default_search_from_file`
  - `def should_skip_path`
  - `def stem_family`
  - `def collect_semantic_files`
  - `def vocab_units`
  - `def pick_canonical`
  - *(...and 3 more)*
- 📄 `llm_bench_runner.js`
  - `const fs`
  - `const args`
  - `const suiteMap`
  - `function runBenchmarks`
  - `const results`
- 📄 `make_icon.py`
- 📄 `make_png_icon.py`
- 📄 `make_real_icon.py`
- 📄 `mcp-call.mjs`
  - `const argsJson`
  - `const bind`
  - `const port`
  - `const line`
  - `const socket`
- 📄 `merge_fibo_domain.py`
  - `def merge_domain`
- 📄 `merge_ontology_manifest.py`
  - `def build_entries`
  - `def merge_group`
- 📄 `package-qualia-wasm.ps1`
- 📄 `package_android_pwa.ps1`
- 📄 `patch-portal-wasm-js.sh`
- 📄 `prepare_bundled_ontologies.sh`
- 📄 `prepare_dublincore_ontologies.sh`
- 📄 `prepare_fibo_ontologies.sh`
- 📄 `prepare_geonames_ontologies.sh`
- 📄 `prepare_purl_ontologies.sh`
- 📄 `prepare_schemaorg_benchmark.ps1`
- 📄 `prepare_schemaorg_benchmark.sh`
- 📄 `prepare_w3c_archives.sh`
- 📄 `prepare_w3c_ontologies.sh`
- 📄 `replace-tailwind-cdn.py`
  - `def css_href`
  - `def main`
- 📄 `replace_lexicon.py`
  - `def replace_in_file`
- 📄 `run_constrained_benchmarks.sh`
- 📄 `scrape_docs_to_rdf.py`
  - `def pdf_text`
  - `def pdf_meta_date`
  - `def docx_text_and_date`
  - `def best_date`
  - `def main`
- 📄 `scrape_w3c_mail_to_rdf.py`
  - `def fetch`
  - `def parse_author`
  - `def rfc822_to_xsd`
  - `def parse_results`
  - `def enrich_body`
  - `def person_uri`
  - `def main`
- 📄 `setup_llm_models.py`
  - `class ModelDownloader`
  - `def __init__`
  - `def download_model`
  - `def _create_placeholder_file`
  - `def _create_gguf_header`
  - `def verify_model`
  - `def create_model_config`
  - `def setup_models`
  - `def main`
- 📄 `sync-portal-design-kit.ps1`
- 📄 `sync_docs_version.py`
  - `def read_version`
  - `def should_skip`
  - `def sync_file`
  - `def main`
- 📄 `test_converter.ps1`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
