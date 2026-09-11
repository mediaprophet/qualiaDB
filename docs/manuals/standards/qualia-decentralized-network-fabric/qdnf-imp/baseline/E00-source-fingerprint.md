# E00 source fingerprint and R01–R19 claim map

Recorded 2026-09-09 on branch `cursor/qdnf-enhancement-e00-e01-cb60`.

## Source state

| Item | Value |
|---|---|
| Integration HEAD at start | `5646b818d4e6fa63f0b7c4c6a1a469dca382339a` (`origin/0.0.37`) |
| Review baseline | `17b6b467c8a4548e302f603674dd594a67be7398` |
| Intervening commits | docs/plan, Pages CI, benchmark stamps — **no QDNF security repairs** |
| Linux runner | rustc 1.98.1, cargo 1.98.1, `x86_64-unknown-linux-gnu` |
| MSVC runner | **unavailable in this environment** (explicit, not claimed) |

Original 30 packages in `task-registry.json` remain **pending**. This map does not bulk-check them.

## Finding → original child packages

| Finding | Enhancement | Original packages (not marked complete) |
|---|---|---|
| R01 forged Active/Allow | E01, E02, E04 | FND-02, RT-03, NET-05 |
| R02 plaintext stream / discarded keys | E02, E04 | CRY-02, NET-02, NET-05 |
| R03 unkeyed Finished / unbound KDF | E02 | CRY-01, CRY-02 |
| R04 generation-only rekey | E02, E09 | CRY-02, NET-05 |
| R05 raw Ethernet unsupported | E05 | NET-01, OPS-01 |
| R06 QSR first-byte modulo | E07 | NET-04 |
| R07 route without target / SPF demo | E07, E08 | NET-03, NET-04 |
| R08 synthetic replication bytes | E06, E11 | CORE-03, SVC-01 |
| R09 in-memory receipts | E06, E11 | CORE-02, SVC-01, EVD-01 |
| R10 per-peer ledgers / CellTable flags | E03, E04, E10 | CORE-01, RT-01, RT-02 |
| R11 arbitrary release amounts | E03 | RT-01 |
| R12 ArenaAdmit vs SlgArena | E03, E06 | CORE-01 |
| R13 explicit alloc counter | E00, E03 | QA-01 |
| R14 default libp2p-compat | E20 | OPS-01, REL-01 |
| R15 signed/supported booleans | E01, E12 | FND-02, SEM-01 |
| R16 clinical inactive/ciphertext | E02, E14 | NET-05, SEM-01 |
| R17 RemainingTarget / payment bypass | E17 | ECO-01 |
| R18 mailbox metadata delivery | E11, E19 | SVC-03, EVD-01 |
| R19 join / sameAs | E12 | FND-02, SEM-01 |

## Qualification gate (E00.5)

Instrument failure of allocator intercept, wire-payload oracle, or crypto oracle fails qualification even when component state-table tests pass. See `crates/qualia-core-db/src/net/qdnf/harness/qualification.rs`.
