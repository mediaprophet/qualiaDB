# UX Design Instructions & Architecture

This document serves as the foundational reference for the User Experience (UX) design patterns and builder architecture within the QualiaDB ecosystem.

## 1. App Builder Paradigms

The platform supports two distinct modes for building and editing QApps. 

### The Default: "Human-Centric" App Builder
This is the primary, default experience for users. It is designed to be highly accessible, intuitive, and focused on natural interaction rather than technical configuration.

### The "Advanced Mode" (Technical Environment)
The current highly-technical interface operates as an "Advanced Mode" (conceptually similar to MIT App Inventor). 
- **Availability:** This technical version *must* remain available to users but must **not** be set as the default app builder.
- **Purpose:** It serves as the deep-dive environment for complex configuration, granular control, and direct visual programming of QApp components.
- **Ontological Design:** The advanced environment must include a dedicated system for "ontological design." This provides users with the tools to deeply model their underlying data:
  - Define custom axioms and rules.
  - Integrate and utilize external semantic libraries (such as WordNet) to establish robust personal ontologies.
  - Utilize a visual, graph-based representation (similar to a mind-map) to architect and visualize complex semantic structures and relationships.

## 2. Component Parity & Evolution

The development of the "Human-Centric" builder and the "Advanced Mode" are deeply intertwined:
- **Continuous Integration:** Each time new functionality or UI elements are developed for the "Human-Centric" app builder, the underlying components necessary to create those elements MUST be simultaneously added to the Advanced Mode environment.
- **Editable QApps:** By maintaining this strict parity, the Advanced Mode inherently provides the means for users to deeply 'edit' and customize their QApps (which may have been generated or built in the Human-Centric mode) using the advanced visual/technical environment.
