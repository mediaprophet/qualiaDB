---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# net Index

## Functionality Overview
Comprehensive index of functionality for `net`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `acoustic_ble_mesh.rs`
  - `struct MeshNetworkManager`
  - `struct AcousticNetwork`
  - `struct AcousticNode`
  - `enum NodeType`
  - `struct AcousticCapabilities`
  - `enum ModulationType`
  - `enum ErrorCorrectionType`
  - `enum NodeStatus`
  - `struct Location`
  - `struct AcousticChannelManager`
  - `struct AcousticChannel`
  - `enum ChannelAllocationStrategy`
  - `struct AcousticModemController`
  - `enum ModemType`
  - `struct SignalProcessingConfig`
  - *(...and 205 more)*
- 📄 `ebpf_filter.rs`
  - `enum RuleAction`
  - `struct FilterRule`
  - `struct RuleStore`
  - `impl RuleStore`
  - `fn insert`
  - `fn remove`
  - `struct BpfProgLoad`
  - `fn bpf_prog_load`
  - `struct EbpfLinuxFilter`
  - `impl EbpfLinuxFilter`
  - `fn new`
  - `impl Drop`
  - `fn drop`
  - `impl NetworkFilter`
  - `fn kind`
  - *(...and 22 more)*
- 📄 `ebpf_firewall.rs`
  - `struct EbpfFirewall`
  - `struct EbpfProgram`
  - `enum ProgramType`
  - `struct SocketInfo`
  - `enum SocketType`
  - `enum Protocol`
  - `struct SocketAddress`
  - `enum AddressFamily`
  - `struct FirewallRule`
  - `enum RuleAction`
  - `struct PacketModification`
  - `enum ModificationOperation`
  - `struct RuleCondition`
  - `enum ConditionOperator`
  - `struct PerformanceMonitor`
  - *(...and 49 more)*
- 📄 `host_topology.rs`
  - `enum AdapterClass`
  - `impl AdapterClass`
  - `fn from_wgpu`
  - `enum HostMemoryTopology`
  - `struct AdapterDesc`
  - `struct HostTopology`
  - `fn backend_rank`
  - `fn probe_host_topology`
  - `impl HostTopology`
  - `fn has_heterogeneous_overflow`
  - `fn summary`
  - `fn h0_probes_host_topology`
- 📄 `mod.rs`
- 📄 `nym_adapter.rs`
  - `struct NymConfig`
  - `impl Default`
  - `fn default`
  - `fn initialize_nym_proxy`
  - `fn request_testnet_faucet_funds`
  - `fn route_through_mixnet`
- 📄 `sonic_token.rs`
  - `struct SonicToken`
  - `enum SonicEventType`
  - `impl SonicToken`
  - `fn pack`
  - `fn delta_time`
  - `fn event_type`
  - `fn channel`
  - `fn note`
  - `fn velocity`
  - `fn tensor_index`
  - `fn flags`
  - `fn pitch_from_tensor`
  - `fn note_on`
  - `fn parametric_pulse`
  - `fn pack_roundtrip`
  - *(...and 1 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
