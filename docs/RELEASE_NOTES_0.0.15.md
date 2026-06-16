# QualiaDB Release 0.0.15

## Summary

Version 0.0.15 focuses on data format expansion, documentation improvements, and version synchronization across the ecosystem.

## Changes

### Data Format Support
- **Centralized CSV Parser/Serializer**: Moved from CLI to `qualia-core-db/sparql_library/parsers/csv_parser.rs`
- **Centralized JSON Parser/Serializer**: Moved from CLI to `qualia-core-db/sparql_library/parsers/json_parser.rs`
- **RDF Parser/Serializers**: Enhanced RDF-Star support with multiple serialization formats (NTriples, Turtle, N-Quads, TriG, N3, JSON-LD)
- **Zero-Heap Compliance**: All parsers/serializers maintain zero-heap allocation compliance
- **WASM Bridge**: Added WASM bindings for CSV/JSON parsing and serialization
- **MCP Server**: Added MCP tools for data format parsing and serialization

### Documentation
- **Menu Configuration**: Created `docs/menu.json` for consistent navigation across all documentation pages
- **Documentation Structure**: Analyzed and catalogued all documentation files
- **Standards**: Reviewed documentation standards and ADR files

### Webizen Browser
- **Build Fixes**: Fixed telemetry bridge module path issues
- **Type Annotations**: Fixed Dioxus event handler type annotations
- **Render Module**: Fixed render module re-exports
- **Signal Mutability**: Fixed signal mutability declarations
- **CSS Syntax**: Fixed inline CSS conditional syntax

### Version Updates
- **All Crates**: Updated to version 0.0.15
- **Workspace**: Synchronized version across all Cargo.toml files
- **Configuration**: Updated Tauri configuration to version 0.0.15

## Breaking Changes

None. This release maintains backward compatibility.

## Migration Guide

No migration required. Existing installations will continue to work.

## Testing

- ✅ All crates build successfully
- ✅ Zero-heap compliance verified for CSV parser
- ✅ Rust documentation generated successfully
- ✅ Webizen browser builds successfully

## Known Issues

- XML and XSD support was attempted but deferred due to `quick_xml` version compatibility issues
- Webizen browser frontend requires Dioxus version compatibility fix (dx 0.7.9 vs dioxus 0.6.3)

## Future Work

- Complete documentation review and updates
- Create remarkable online examples
- Resolve XML/XSD parser integration
- Fix Dioxus version compatibility for webizen-studio
