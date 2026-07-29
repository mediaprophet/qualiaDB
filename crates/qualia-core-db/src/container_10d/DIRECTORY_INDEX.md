---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# container_10d Index

## Functionality Overview

Implements the sealed `.10d` binary container contract used for quantised geometry,
Tensor10D nodes, provenance, topology, and spatial indexes. The format provides fixed
headers, aligned typed sections, deterministic encoding, per-section and whole-file
integrity, and caller-buffered readers for render/query paths.

## File & Subdirectory Manifest

- `axis_role.rs`: Declares Tensor10D axis roles and validates profile/role combinations.
- `conformance.rs`: Golden-layout and conformance helpers for `.10d` producers/consumers.
- `crc32c.rs`: CRC-32C implementation used by sections and whole-file sealing.
- `header.rs`: Parses and emits the fixed 64-byte container header.
- `integrity.rs`: Computes, seals, and verifies whole-file integrity.
- `mesh_section.rs`: Encodes and decodes quantised triangle meshes.
- `metric_check.rs`: Validates metric/topology compatibility for Tensor10D profiles.
- `mod.rs`: Container module boundary and public re-exports.
- `node_section.rs`: Encodes and reads caller-buffered Tensor10D node sections.
- `provenance_section.rs`: Encodes and decodes in-container source, licence, semantic
  metadata, verification-credential, and timestamp provenance.
- `section.rs`: Section types, alignment tiers, table encoding, canonical ordering, and
  bounded parsing.
- `spatial_index_section.rs`: Serialises and parses BVH and kd-tree spatial indexes.
- `topology_section.rs`: Serialises and parses half-edge mesh topology.

## Changelog

- **2026-07-29**: Created a comprehensive semantic index reflecting the implemented
  provenance, topology, spatial-index, Tensor10D, and integrity sections.
