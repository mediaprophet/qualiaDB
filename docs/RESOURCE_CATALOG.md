# Resource Catalog Implementation Plan

**Branch:** `feat/resource-catalog`

## Goals
- Dynamic resource management without recompilation
- Sovereign and offline-first by default
- Clean integration with existing download/persistence system

## Current Progress
- YAML registries created and expanded
- Rust types and loader implemented
- Basic structure ready for CLI and UI integration

## Integration Notes (Download / Persistence System)

The Resource Catalog should integrate with the existing download system added in v0.0.5:

- Use the same download destination and verification logic.
- Record provenance when importing ontologies or models into the Super-Quin graph.
- For LLMs: Store metadata + optional checksum alongside the GGUF file.
- For Ontologies: After download, optionally run SHACL validation before importing.
- For SPARQL: Store endpoint metadata in the management graph so it can be queried later.

## CLI Command Stubs (Suggested)

```bash
qualia resources list llms
qualia resources list ontologies
qualia resources list sparql
qualia resources show <id>
qualia resources download <id>
qualia resources import-ontology <id>
```

## Usage Example (Rust)

```rust
use std::path::Path;
use qualia_db::resources::{ResourceCatalog, load_catalog};

fn main() {
    let mut catalog = ResourceCatalog::new();
    catalog.load_from_files(Path::new("resources/catalog.yaml"))
        .expect("Failed to load resource catalog");

    println!("Loaded {} LLMs", catalog.llm_count());
    println!("Loaded {} Ontologies", catalog.ontology_count());
    println!("Loaded {} SPARQL endpoints", catalog.sparql_count());

    // Example: Find edge-friendly LLMs
    for llm in &catalog.llms {
        if let Some(tags) = &llm.tags {
            if tags.contains(&"edge".to_string()) {
                println!("Edge model available: {}", llm.name);
            }
        }
    }
}
```

## Next Priorities
1. Wire the catalog into application startup
2. Implement `qualia resources` CLI commands
3. Connect downloads to the existing persistence layer
4. Add UI browser in React/Vite

## Notes
- Keep `allow_external_refresh` false by default for sovereignty.
- Consider making the catalog itself importable as RDF into QualiaDB.