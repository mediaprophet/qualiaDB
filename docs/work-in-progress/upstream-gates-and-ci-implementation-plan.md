# Upstream Gates & CI Implementation Plan (QG-01, QG-15, QG-09 & Desktop CI)

Status: **execution plan and test ledger** (2026-10-03).
Reference sources:
- `https://github.com/mediaprophet/qualiaDB/actions/runs/36991474986` (failed Desktop Release)
- `C:/github/game-demo/docs/planning/19-qualiadb-upstream-gate-work-orders.md`
- `docs/work-in-progress/q42-mutable-journal-game-proposal.md`
- `AGENTS.md` (Zero-heap hot path Tier 1, 48-byte Super-Quin, 42MB Sentinel)

---

## 1. Operating Rules for This Implementation

1. **Implement everything before starting testing loops.**
   Do not write 5 lines of code and then run minutes/hours of tests in a tight debugging cycle. Complete the full code architecture across all target files first.
2. **Strict conformance to `AGENTS.md`**:
   - `NQuin` is 48 bytes (six `u64` fields, 5-field XOR fold parity).
   - Zero heap inside hot tick loops and evaluator frames.
   - Caller supplies fixed-size `&mut [T]` output buffers.
3. **No stubbing in WASM persistence**:
   Remove the dummy `Ok(())` / `Vec::new()` mocks in the persistent path.

---

## 2. Test Ledger: What Must Pass

Before any test is run, the following test cases must be implemented:

### Suite A: Desktop CI & Frontend Build Verification
- [x] **A.1**: Toolchain pinned to `1.98.1` in `.github/workflows/release-desktop.yml` on both Windows and macOS jobs.
- [x] **A.2**: `scripts/build_frontend.sh` verifies/installs `esbuild@0.27.3` via npm and provisions `wasm-opt` (Binaryen) before `dx build` runs with `NO_DOWNLOADS=1`.
- [x] **A.3**: `scripts/build_frontend.ps1` maintains exact version parity with CLI and `Cargo.toml`.
- [x] **A.4**: `webizen-studio` links cleanly for `wasm32-unknown-unknown` without missing wasm-bindgen adapters.

### Suite B: QG-15 Mutable Q42 Journal (`.q42j`)
Located in `crates/qualia-core-db/src/q42/journal.rs` (and registered in `q42/mod.rs`):
- [x] **B.1 `test_q42j_header_roundtrip`**:
  Serialize and deserialize a 128-byte `Q42J` file header with version, features, base `.q42` SHA-256 digest, generation, and Sanctuary privacy flag.
- [x] **B.2 `test_q42j_single_transaction_commit_and_recover`**:
  Write 1 transaction containing a command header, Quin add/remove deltas, and commit frame; flush and recover. Assert exact state matches.
- [x] **B.3 `test_q42j_multi_quin_atomicity`**:
  Verify multiple Quins inside a transaction commit atomically or not at all.
- [x] **B.4 `test_q42j_torn_transaction_tail_discard`**:
  Simulate power loss / browser crash by writing half of a transaction frame at the file tail without a commit marker. Replay must successfully recover the preceding committed transactions and cleanly ignore the torn tail.
- [x] **B.5 `test_q42j_corrupt_committed_frame_fails_closed`**:
  Alter a single byte in a committed transaction frame. Replay must detect the checksum/digest mismatch and fail closed with `JournalError::CorruptedFrame`.
- [x] **B.6 `test_q42j_base_digest_mismatch_rejected`**:
  Opening a `.q42j` journal against the wrong base `.q42` snapshot digest immediately returns `JournalError::BaseDigestMismatch`.
- [x] **B.7 `test_q42j_sanctuary_privacy_default`**:
  Verify new journals default to `PrivacyClass::Sanctuary` and forbid public magnet export.
- [x] **B.8 `test_q42j_checkpoint_rotation`**:
  Create a new sealed `.q42` checkpoint generation, update the generation pointer, and reset the journal cursor.

### Suite C: QG-01 Canonical HMC Bundle (`QBDL`)
Located in `crates/qualia-core-db/src/bundle/`:
- [x] **C.1 `test_hmc_qbdl_pack_read_write_identical_entry_slices`**:
  Pack a `.10d` asset, a Q42 manifest, and an HCF description into a `QBDL` container. Verify each entry slice extracted from the bundle is bit-identical to the input file.
- [x] **C.2 `test_hmc_qbdl_tampered_byte_rejection`**:
  Tamper with 1 byte in a bundled payload. The reader must detect CRC32C / SHA-256 failure and reject with a descriptive error.
- [x] **C.3 `test_hmc_qbdl_cbor_manifest_validation`**:
  Verify the CBOR index accurately maps keys, MIME/kind tags, and page alignments (64-byte boundary).

### Suite D: QG-09 Deterministic Fixed-Tick RTS Simulation Engine
Located in `crates/qualia-core-db/src/simulation/fixed_tick.rs`:
- [x] **D.1 `test_fixed_tick_multi_agent_deterministic_replay`**:
  Run a 1,000-tick simulation with multiple agents (moving, harvesting, building) with seeded PRNG. Replay identical commands from the same seed native and verify final state hash is 100% bit-identical.
- [x] **D.2 `test_fixed_tick_rejected_command_leaves_state_intact`**:
  Submit an invalid / unauthorized command (e.g. insufficient resources, collision). Verify command is rejected with typed receipt and world state is unaltered.
- [x] **D.3 `test_fixed_tick_zero_heap_budget_compliance`**:
  Verify hot tick iteration uses preallocated/caller-supplied buffers without heap allocations in the step kernel.

---

## 3. Implementation Sequence (Code First, Then Test)

1. **Step 1**: Fix CI workflows & frontend scripts:
   - Edit `.github/workflows/release-desktop.yml`: set `toolchain: "1.98.1"`.
   - Edit `scripts/build_frontend.sh`: add `esbuild` detection/installation and `wasm-opt` provisioning.
2. **Step 2**: Implement `q42j` (Mutable Q42 Journal) in `crates/qualia-core-db/src/q42/journal.rs`.
   - Define file header, frame types, checksums, atomic commit, replay parser, and storage trait (supporting File and in-memory byte cursor / OPFS).
   - Register in `crates/qualia-core-db/src/q42/mod.rs` and `lib.rs`.
3. **Step 3**: Solidify QG-01 HMC Bundle conformance:
   - Ensure `crates/qualia-core-db/src/bundle/` exposes a clean, typed public API for packing and reading canonical game assets.
4. **Step 4**: Implement QG-09 Fixed-Tick Engine in `crates/qualia-core-db/src/simulation/fixed_tick.rs`:
   - Fixed-step loop, deterministic PRNG, command queue, reducer receipts, and state hashing.
   - Register in `crates/qualia-core-db/src/lib.rs`.
5. **Step 5**: Write all unit and integration tests defined in §2.
6. **Step 6**: Execute test pass and verify all suites green.
