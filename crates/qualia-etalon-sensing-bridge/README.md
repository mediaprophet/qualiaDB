# QualiaDB–Etalon sensing bridge

This library maps Etalon observation envelopes into QualiaDB measurement records,
retaining the original observation alongside the derived index. It also provides
radio/NFC mapping, canonical sidecars, signed custody and container-bound
publication/read/recovery composition.

The source now lives in the canonical QualiaDB checkout. Use sibling checkouts:

```text
github/
  etalon/
  qualiaDB/
    crates/qualia-etalon-sensing-bridge/
```

The relative Etalon dependencies in `Cargo.toml` resolve through that layout.
No `.audit` directory or Git worktree is required. The bridge is a standalone
workspace so applications can opt in without adding vendor runtimes or native
device backends to QualiaDB's default dependency graph.

From the QualiaDB repository root:

```console
cargo test --manifest-path crates/qualia-etalon-sensing-bridge/Cargo.toml
cargo run --manifest-path applications/sensing-inspect/Cargo.toml -- --help
```

For integration, construct an Etalon `ObservationEnvelope` and a checked Qualia
`ResolvedFieldPayloadBinding`, then call `bridge_observation` with a concrete
`BridgeAuthority`. The authority binds exact payload bytes and source/unit/clock/
calibration namespaces to the application's current policy. Environmental
observations need not invent a human subject. Human-centered ownership,
Sanctuary and guardianship decisions remain QualiaDB host integrations.

`bridge_observation_with_consent` is an optional adapter for existing signed
consent grants. Capture/processing permission never automatically authorizes
storage, reading, sharing or export. Persistence must retain the complete
observation sidecar and authenticated payload binding, not only the derived index.

The native SDR capture-payload codec can retain raw I/Q and native device/config
declarations as an opaque custody payload. Its decoded metadata is historical
and untrusted; the host authority must validate its relationship to the selected
field and observation. A signature does not establish scientific truth,
calibration or physical device identity.

The migration source inventory is recorded in
`docs/integration/sensing-canonical-migration-2026-11-01.json`. It pins the
original copied bytes before subsequent integration fixes. The old source
worktree is retained as an archive; it is not a build input.
