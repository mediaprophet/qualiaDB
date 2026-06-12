# Explanatory Memorandum for SPARQL-MM Specification

## Purpose
This memorandum provides context and rationale for the unofficial draft specification of SPARQL-MM (Multimedia Extension for SPARQL 1.1), as documented in the ReSpec format. SPARQL-MM extends the SPARQL 1.1 Query Language to enable querying of multimedia data within RDF triplestores, focusing on temporal and spatial relationships of media fragments. The specification aims to formalize the syntax, semantics, and usage of SPARQL-MM functions, while incorporating support for related standards, including SPARQL 1.2 Federated Query and W3C Media Fragments URI 1.0.

## Background
SPARQL-MM, originally developed by Thomas Kurz at Salzburg Research, is an open-source project hosted on GitHub (`https://github.com/tkurz/sparql-mm`). It is implemented for the Sesame triplestore and available via Maven Central. The extension addresses the need for querying multimedia data (e.g., video, audio, images) in Semantic Web applications by providing functions to analyze temporal and spatial properties of media fragments. These fragments are identified using the W3C Media Annotations Ontology and represented as URIs per the W3C Media Fragments URI 1.0 specification.

The specification was drafted to provide a structured, standards-aligned reference for developers and researchers, ensuring compatibility with W3C standards and facilitating integration with modern SPARQL engines.

## Key Features
The SPARQL-MM specification includes the following key features:

1. **Temporal and Spatial Functions**:
   - Functions such as `mm:precedes`, `mm:temporalIntermediate`, and `mm:duration` handle temporal relationships of media fragments (e.g., time ranges like `#t=10,20`).
   - Functions like `mm:rightBeside`, `mm:spatialIntersection`, and `mm:boundingBox` manage spatial relationships (e.g., regions like `#xywh=160,120,320,240`).
   - These functions operate on media fragments defined by the Media Annotations Ontology and W3C Media Fragments URI 1.0 standards.

2. **SPARQL 1.2 Federated Query Support**:
   - SPARQL-MM supports distributed queries across remote SPARQL endpoints using the `SERVICE` clause, as defined in the SPARQL 1.2 Federated Query specification.
   - This enables querying multimedia data stored in multiple triplestores, with consistent application of SPARQL-MM functions.

3. **Integration with Sesame**:
   - The extension is implemented for the Sesame framework, using Java Class Loader Technology for seamless integration.
   - It is packaged and distributed via Maven Central, ensuring accessibility for developers.

4. **Standards Compliance**:
   - The specification aligns with SPARQL 1.1 Query Language, Media Annotations Ontology, and W3C Media Fragments URI 1.0.
   - It recommends (but does not mandate) support for media fragment URIs and federated queries, allowing flexibility for implementers.

## Rationale for Updates
The specification incorporates several updates to address errors and enhance compatibility:

1. **License Error Resolution**:
   - The original draft included an invalid `license: "mit"` field in the ReSpec configuration, which caused a validation error. The field was removed to allow ReSpec to select a default license, ensuring compliance with ReSpec’s supported options (`cc0`, `w3c-software`, etc.).

2. **SPARQL 1.2 Federated Query Integration**:
   - Support for federated queries was added to align with modern SPARQL use cases, where multimedia data may be distributed across multiple endpoints. This feature enables SPARQL-MM to remain relevant in decentralized Semantic Web applications.
   - A dedicated section (`federated-query-support`) was included with an example demonstrating the use of SPARQL-MM functions in a `SERVICE` clause.

3. **W3C Media Fragments URI 1.0 Reference**:
   - References to the W3C Media Fragments URI 1.0 specification were added to ensure that media fragment handling aligns with a W3C standard. This clarifies the representation of temporal and spatial fragments (e.g., `#t=10,20`, `#xywh=160,120,320,240`) and enhances interoperability.
   - Sections such as `functions`, `temporal-functions`, and `spatial-functions` were updated to explicitly tie function semantics to media fragment URIs.

## Structure of the Specification
The specification is organized as follows:
- **Abstract and Introduction**: Provide an overview of SPARQL-MM’s purpose and scope, highlighting its multimedia focus and standards alignment.
- **Conformance**: Specifies requirements for SPARQL 1.1 compliance and optional support for federated queries and media fragments.
- **Namespace**: Defines the SPARQL-MM function namespace (`http://linkedmultimedia.org/sparql-mm/ns/2.0.0/function#`).
- **Functions**: Details temporal and spatial functions, with examples illustrating their usage in SPARQL queries.
- **Federated Query Support**: Describes integration with SPARQL 1.2 Federated Query, including an example.
- **Integration**: Outlines how to integrate SPARQL-MM with Sesame via Maven Central.
- **Examples and Acknowledgments**: References additional test cases and credits contributors.

## Intended Audience
The specification targets:
- **Developers** implementing SPARQL-MM in Semantic Web applications or triplestores.
- **Researchers** exploring multimedia querying in RDF environments.
- **Standards enthusiasts** interested in extensions to SPARQL for multimedia data.

## Limitations
- **Unofficial Status**: SPARQL-MM is not endorsed by the W3C or any standards body, limiting its formal adoption.
- **Sesame-Specific Implementation**: The current implementation is tailored for Sesame, which may restrict its use with other triplestores.
- **Optional Features**: Support for federated queries and media fragments is optional, which may lead to inconsistent implementations.

## Future Considerations
Future iterations of the specification could:
- Extend support to other triplestores (e.g., Jena, Blazegraph).
- Incorporate additional multimedia standards, such as MPEG-7 or EXIF.
- Seek community feedback via the GitHub repository to refine function semantics or add new features.

## Conclusion
The SPARQL-MM specification provides a robust framework for querying multimedia data in RDF triplestores, with a focus on temporal and spatial relationships. By aligning with SPARQL 1.1, SPARQL 1.2 Federated Query, Media Annotations Ontology, and W3C Media Fragments URI 1.0, it ensures compatibility with Semantic Web standards. The updates in this draft address technical errors and enhance functionality, making SPARQL-MM a valuable tool for multimedia Semantic Web applications.

**Date**: June 21, 2025