# Resource Catalog for QualiaDB

This directory contains human-editable YAML files that define available external resources (LLMs, Ontologies, and SPARQL endpoints).

The goal is to allow the application to discover, download, and manage these resources **without requiring code changes or recompilation** when new resources are added.

## Philosophy

- **Sovereign & Offline-first**: Prefer local copies with provenance. Public sources are treated as discovery/download origins, not runtime dependencies.
- **Dynamic**: Update lists by editing YAML files (git-friendly).
- **Extensible**: Easy to add new resource types or metadata fields.
- **QualiaDB Native**: Designed to integrate with the existing download/persistence system, Super-Quin storage, SHACL validation, and provenance tracking.

## Directory Contents

| File                    | Purpose                                      |
|-------------------------|----------------------------------------------|
| `catalog.yaml`          | Master index pointing to the other registries |
| `llms.yaml`             | Downloadable LLM / model packages (GGUF focus) |
| `ontologies.yaml`       | OWL/RDF ontologies (small and large)         |
| `sparql_endpoints.yaml` | Public SPARQL endpoints with metadata        |
| `README.md`             | This file                                    |

## How It Works (High Level)

1. On startup or explicit command, QualiaDB loads `catalog.yaml` and the referenced registry files.
2. The UI / CLI can display, filter, and search available resources.
3. When a user selects a resource:
   - LLMs → Download via existing persistence layer + store metadata.
   - Ontologies → Download + optional SHACL validation + import into graph with provenance.
   - SPARQL → Store endpoint details + optional query templates.
4. All decisions (licenses, size limits, verification) are controlled via `config/resources.yaml`.

## Adding New Resources

Simply edit the appropriate `.yaml` file and commit. No code changes required.

Future enhancements may include an optional "Refresh Catalog" feature that pulls from Hugging Face, BioPortal, LOV, etc., but always stores results locally.

## Configuration

See `../config/resources.example.yaml` for runtime preferences (preferred quantization, license filters, download behavior, etc.).

## Branch & Development

This feature is being developed on the `feat/resource-catalog` branch.