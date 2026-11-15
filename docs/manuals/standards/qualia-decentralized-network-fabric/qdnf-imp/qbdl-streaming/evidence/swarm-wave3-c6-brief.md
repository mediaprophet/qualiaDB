# Wave 3-C6 — qualification records (support, queued 3B)

**Do not launch until Wave 3A source is routed and integrator cargo is green.** Docs only. No production crate writes. No cargo.

## Gap

`qualification.md` lists gates. Live evidence is scattered logs. C6 for **this close-out** is a truthful map from tests that actually ran to `net/qdnf/harness/qualification.rs` `EvidenceClass` values. Mixing classes into one “all tests passed” claim is forbidden.

## Allowed writes

- NEW `docs/.../qbdl-streaming/evidence/swarm-wave3-c6-records.md`

Do not edit `qualification.md` unless the integrator grants it after this file exists.

## Record, separately

| Class | What may be cited | What may not |
|---|---|---|
| Component | C5 HTTP loopback; Wave 3-H mixed HTTP; in-process `ProtectedSession` 17/17; planner unit tests | Named as Process |
| Process | `two_process.rs` ProtectedSession useful-leaf (after its cargo); Windows named-pipe / Unix UDS already-green local_stream process tests | PhysicalNetwork; browser |
| PhysicalNetwork | nothing in this wave | Inferring from Process or loopback HTTP |
| Application | nothing in this wave | Browser/WASM, Etalon consumer, Commons |

For each cited run: package/gate, dirty fingerprint note (tree is dirty — say so), exact command, log path, exit, test names, class. Point at existing UTF-8 logs under `evidence/` (`swarm-c1-full-2026-11-14.log`, Wave 3 cargo log once it exists).

G01 two-host native without HTTP/DNS/IP/libp2p stays **open** (PhysicalNetwork). G02 binding parity is Component-only until Process HTTP is a different machine. G11 browser stays later. G12 WSS framing cargo 11/11 is Component/process-framing — do not upgrade it to mixed-HMC WSS parity unless a mixed WSS test actually ran.

ZIP `.hmc` unsupported in this record.

## Stop

`swarm-wave3-c6-records.md` with a table. No production edits. No commit. No claim that QBS-09 is complete.
